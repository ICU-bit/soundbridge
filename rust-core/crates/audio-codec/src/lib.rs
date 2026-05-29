use audio_core::{AudioBuffer, AudioFormat, SampleFormat};
use opus::{
    Application, Bitrate as OpusBitrate, Channels as OpusChannels,
    Decoder as OpusDecoder, Encoder as OpusEncoder,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("opus encoder error: {0}")]
    EncoderError(String),
    #[error("opus decoder error: {0}")]
    DecoderError(String),
    #[error("invalid frame size: {0}")]
    InvalidFrameSize(String),
    #[error("invalid sample rate: {0}")]
    InvalidSampleRate(u32),
    #[error("invalid channel config: {0}")]
    InvalidChannelConfig(String),
    #[error("buffer size mismatch: expected {expected}, got {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },
    #[error("encoding failed: {0}")]
    EncodingFailed(String),
    #[error("decoding failed: {0}")]
    DecodingFailed(String),
}

pub type Result<T> = std::result::Result<T, CodecError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRate {
    Hz44100 = 44100,
    Hz48000 = 48000,
}

impl SampleRate {
    pub fn value(&self) -> u32 {
        *self as u32
    }

    pub fn from_u32(value: u32) -> Result<Self> {
        match value {
            44100 => Ok(SampleRate::Hz44100),
            48000 => Ok(SampleRate::Hz48000),
            _ => Err(CodecError::InvalidSampleRate(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelConfig {
    Mono = 1,
    Stereo = 2,
}

impl ChannelConfig {
    pub fn count(&self) -> u16 {
        *self as u16
    }

    pub fn to_opus_channels(&self) -> OpusChannels {
        match self {
            ChannelConfig::Mono => OpusChannels::Mono,
            ChannelConfig::Stereo => OpusChannels::Stereo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bitrate {
    Kbps64 = 64000,
    Kbps128 = 128000,
    Kbps256 = 256000,
}

impl Bitrate {
    pub fn bits_per_second(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSize {
    Ms10 = 10,
    Ms20 = 20,
    Ms40 = 40,
}

impl FrameSize {
    pub fn milliseconds(&self) -> u32 {
        *self as u32
    }

    pub fn samples(&self, sample_rate: SampleRate) -> usize {
        let ms = self.milliseconds() as usize;
        let hz = sample_rate.value() as usize;
        (hz * ms) / 1000
    }
}

#[derive(Debug, Clone)]
pub struct OpusConfig {
    pub sample_rate: SampleRate,
    pub channels: ChannelConfig,
    pub bitrate: Bitrate,
    pub frame_size: FrameSize,
    pub application: Application,
}

impl Default for OpusConfig {
    fn default() -> Self {
        Self {
            sample_rate: SampleRate::Hz48000,
            channels: ChannelConfig::Mono,
            bitrate: Bitrate::Kbps128,
            frame_size: FrameSize::Ms20,
            application: Application::Audio,
        }
    }
}

impl OpusConfig {
    pub fn new(
        sample_rate: SampleRate,
        channels: ChannelConfig,
        bitrate: Bitrate,
        frame_size: FrameSize,
    ) -> Self {
        Self {
            sample_rate,
            channels,
            bitrate,
            frame_size,
            application: Application::Audio,
        }
    }

    pub fn with_application(mut self, application: Application) -> Self {
        self.application = application;
        self
    }

    pub fn frame_size_samples(&self) -> usize {
        self.frame_size.samples(self.sample_rate)
    }

    pub fn samples_per_channel(&self) -> usize {
        self.frame_size_samples() * self.channels.count() as usize
    }

    pub fn to_audio_format(&self) -> AudioFormat {
        AudioFormat {
            sample_rate: self.sample_rate.value(),
            channels: self.channels.count(),
            sample_format: SampleFormat::F32,
        }
    }
}

const MAX_PACKET_SIZE: usize = 4000;

pub struct OpusEncoderCodec {
    encoder: OpusEncoder,
    config: OpusConfig,
}

impl OpusEncoderCodec {
    pub fn new(config: OpusConfig) -> Result<Self> {
        let encoder = OpusEncoder::new(
            config.sample_rate.value(),
            config.channels.to_opus_channels(),
            config.application,
        )
        .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        let mut codec = Self { encoder, config };
        codec.apply_bitrate()?;
        Ok(codec)
    }

    fn apply_bitrate(&mut self) -> Result<()> {
        self.encoder
            .set_bitrate(OpusBitrate::Bits(self.config.bitrate.bits_per_second()))
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;
        Ok(())
    }

    pub fn encode(&mut self, buffer: &AudioBuffer<f32>) -> Result<Vec<u8>> {
        let samples = buffer.samples();
        let expected_samples = self.config.samples_per_channel();

        if samples.len() != expected_samples {
            return Err(CodecError::BufferSizeMismatch {
                expected: expected_samples,
                actual: samples.len(),
            });
        }

        let frame_size = self.config.frame_size_samples();
        let mut output = vec![0u8; MAX_PACKET_SIZE];

        let encoded_size = self
            .encoder
            .encode_vec(samples, frame_size, &mut output)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))?;

        output.truncate(encoded_size);
        Ok(output)
    }

    pub fn encode_interleaved(&mut self, samples: &[f32]) -> Result<Vec<u8>> {
        let expected_samples = self.config.samples_per_channel();

        if samples.len() != expected_samples {
            return Err(CodecError::BufferSizeMismatch {
                expected: expected_samples,
                actual: samples.len(),
            });
        }

        let frame_size = self.config.frame_size_samples();
        let mut output = vec![0u8; MAX_PACKET_SIZE];

        let encoded_size = self
            .encoder
            .encode_vec(samples, frame_size, &mut output)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))?;

        output.truncate(encoded_size);
        Ok(output)
    }

    pub fn config(&self) -> &OpusConfig {
        &self.config
    }
}

pub struct OpusDecoderCodec {
    decoder: OpusDecoder,
    config: OpusConfig,
}

impl OpusDecoderCodec {
    pub fn new(config: OpusConfig) -> Result<Self> {
        let decoder = OpusDecoder::new(
            config.sample_rate.value(),
            config.channels.to_opus_channels(),
        )
        .map_err(|e| CodecError::DecoderError(e.to_string()))?;

        Ok(Self { decoder, config })
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<AudioBuffer<f32>> {
        let frame_size = self.config.frame_size_samples();

        let decoded = self
            .decoder
            .decode_vec(data, frame_size, false)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        let format = self.config.to_audio_format();
        let audio_buffer = AudioBuffer::new(decoded, format)
            .map_err(|_| CodecError::DecodingFailed("failed to create audio buffer".to_string()))?;

        Ok(audio_buffer)
    }

    pub fn decode_into(&mut self, data: &[u8], output: &mut [f32]) -> Result<usize> {
        let frame_size = self.config.frame_size_samples();
        let expected_size = self.config.samples_per_channel();

        if output.len() < expected_size {
            return Err(CodecError::BufferSizeMismatch {
                expected: expected_size,
                actual: output.len(),
            });
        }

        let decoded = self
            .decoder
            .decode_vec(data, frame_size, false)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        let decoded_len = decoded.len();
        output[..decoded_len].copy_from_slice(&decoded);
        Ok(decoded_len)
    }

    pub fn config(&self) -> &OpusConfig {
        &self.config
    }
}

pub struct OpusCodec {
    encoder: OpusEncoder,
    decoder: OpusDecoder,
    config: OpusConfig,
}

impl OpusCodec {
    pub fn new(config: OpusConfig) -> Result<Self> {
        let encoder = OpusEncoder::new(
            config.sample_rate.value(),
            config.channels.to_opus_channels(),
            config.application,
        )
        .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        let decoder = OpusDecoder::new(
            config.sample_rate.value(),
            config.channels.to_opus_channels(),
        )
        .map_err(|e| CodecError::DecoderError(e.to_string()))?;

        let mut codec = Self {
            encoder,
            decoder,
            config,
        };
        codec.apply_bitrate()?;
        Ok(codec)
    }

    fn apply_bitrate(&mut self) -> Result<()> {
        self.encoder
            .set_bitrate(OpusBitrate::Bits(self.config.bitrate.bits_per_second()))
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;
        Ok(())
    }

    pub fn encode(&mut self, buffer: &AudioBuffer<f32>) -> Result<Vec<u8>> {
        let samples = buffer.samples();
        let expected_samples = self.config.samples_per_channel();

        if samples.len() != expected_samples {
            return Err(CodecError::BufferSizeMismatch {
                expected: expected_samples,
                actual: samples.len(),
            });
        }

        let frame_size = self.config.frame_size_samples();
        let mut output = vec![0u8; MAX_PACKET_SIZE];

        let encoded_size = self
            .encoder
            .encode_vec(samples, frame_size, &mut output)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))?;

        output.truncate(encoded_size);
        Ok(output)
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<AudioBuffer<f32>> {
        let frame_size = self.config.frame_size_samples();

        let decoded = self
            .decoder
            .decode_vec(data, frame_size, false)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        let format = self.config.to_audio_format();
        let audio_buffer = AudioBuffer::new(decoded, format)
            .map_err(|_| CodecError::DecodingFailed("failed to create audio buffer".to_string()))?;

        Ok(audio_buffer)
    }

    pub fn encode_decode(&mut self, buffer: &AudioBuffer<f32>) -> Result<AudioBuffer<f32>> {
        let encoded = self.encode(buffer)?;
        self.decode(&encoded)
    }

    pub fn config(&self) -> &OpusConfig {
        &self.config
    }
}

pub struct AudioCodec {
    opus: OpusCodec,
}

impl AudioCodec {
    pub fn new() -> Result<Self> {
        let config = OpusConfig::default();
        let opus = OpusCodec::new(config)?;
        Ok(Self { opus })
    }

    pub fn with_config(config: OpusConfig) -> Result<Self> {
        let opus = OpusCodec::new(config)?;
        Ok(Self { opus })
    }

    pub fn encode(&mut self, buffer: &AudioBuffer<f32>) -> Result<Vec<u8>> {
        self.opus.encode(buffer)
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<AudioBuffer<f32>> {
        self.opus.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_samples(count: usize) -> Vec<f32> {
        (0..count)
            .map(|i| {
                let frequency = 440.0;
                let sample_rate = 48000.0;
                let t = i as f32 / sample_rate;
                (2.0 * std::f32::consts::PI * frequency * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_opus_config_default() {
        let config = OpusConfig::default();
        assert_eq!(config.sample_rate, SampleRate::Hz48000);
        assert_eq!(config.channels, ChannelConfig::Mono);
        assert_eq!(config.bitrate, Bitrate::Kbps128);
        assert_eq!(config.frame_size, FrameSize::Ms20);
    }

    #[test]
    fn test_opus_config_custom() {
        let config = OpusConfig::new(
            SampleRate::Hz44100,
            ChannelConfig::Stereo,
            Bitrate::Kbps256,
            FrameSize::Ms40,
        );
        assert_eq!(config.sample_rate, SampleRate::Hz44100);
        assert_eq!(config.channels, ChannelConfig::Stereo);
        assert_eq!(config.bitrate, Bitrate::Kbps256);
        assert_eq!(config.frame_size, FrameSize::Ms40);
    }

    #[test]
    fn test_sample_rate_conversion() {
        assert_eq!(SampleRate::Hz44100.value(), 44100);
        assert_eq!(SampleRate::Hz48000.value(), 48000);
        assert_eq!(SampleRate::from_u32(44100).unwrap(), SampleRate::Hz44100);
        assert_eq!(SampleRate::from_u32(48000).unwrap(), SampleRate::Hz48000);
        assert!(SampleRate::from_u32(22050).is_err());
    }

    #[test]
    fn test_frame_size_samples() {
        let frame_10ms = FrameSize::Ms10;
        assert_eq!(frame_10ms.samples(SampleRate::Hz48000), 480);
        assert_eq!(frame_10ms.samples(SampleRate::Hz44100), 441);

        let frame_20ms = FrameSize::Ms20;
        assert_eq!(frame_20ms.samples(SampleRate::Hz48000), 960);
        assert_eq!(frame_20ms.samples(SampleRate::Hz44100), 882);

        let frame_40ms = FrameSize::Ms40;
        assert_eq!(frame_40ms.samples(SampleRate::Hz48000), 1920);
        assert_eq!(frame_40ms.samples(SampleRate::Hz44100), 1764);
    }

    #[test]
    fn test_opus_encoder_decoder_mono() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms20,
        );

        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_test_samples(960);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = encoder.encode(&input_buffer).unwrap();
        assert!(!encoded.is_empty());

        let decoded_buffer = decoder.decode(&encoded).unwrap();
        let decoded_samples = decoded_buffer.samples();

        assert_eq!(decoded_samples.len(), samples.len());
        assert_ne!(decoded_samples, &[0.0; 960], "decoded output must not be silence");
    }

    #[test]
    fn test_opus_encoder_decoder_stereo() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );

        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_test_samples(1920);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = encoder.encode(&input_buffer).unwrap();
        assert!(!encoded.is_empty());

        let decoded_buffer = decoder.decode(&encoded).unwrap();
        let decoded_samples = decoded_buffer.samples();

        assert_eq!(decoded_samples.len(), samples.len());
    }

    #[test]
    fn test_opus_codec_combined() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms10,
        );

        let mut codec = OpusCodec::new(config).unwrap();

        let samples = create_test_samples(480);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded_buffer = codec.decode(&encoded).unwrap();

        assert_eq!(decoded_buffer.samples().len(), samples.len());
        assert_ne!(
            decoded_buffer.samples(),
            vec![0.0f32; 480].as_slice(),
            "decoded must not be silence"
        );
    }

    #[test]
    fn test_opus_codec_roundtrip_different_bitrates() {
        let sample_rate = SampleRate::Hz48000;
        let channels = ChannelConfig::Mono;
        let frame_size = FrameSize::Ms20;
        let samples = create_test_samples(960);

        for bitrate in [Bitrate::Kbps64, Bitrate::Kbps128, Bitrate::Kbps256] {
            let config = OpusConfig::new(sample_rate, channels, bitrate, frame_size);
            let mut codec = OpusCodec::new(config).unwrap();

            let format = config.to_audio_format();
            let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

            let encoded = codec.encode(&input_buffer).unwrap();
            assert!(!encoded.is_empty(), "encoding failed for bitrate {:?}", bitrate);

            let decoded_buffer = codec.decode(&encoded).unwrap();
            assert_eq!(decoded_buffer.samples().len(), samples.len());
        }
    }

    #[test]
    fn test_opus_codec_roundtrip_different_frame_sizes() {
        let sample_rate = SampleRate::Hz48000;
        let channels = ChannelConfig::Mono;
        let bitrate = Bitrate::Kbps128;

        let frame_10ms = create_test_samples(480);
        let frame_20ms = create_test_samples(960);
        let frame_40ms = create_test_samples(1920);

        for (frame_size, samples) in [
            (FrameSize::Ms10, &frame_10ms),
            (FrameSize::Ms20, &frame_20ms),
            (FrameSize::Ms40, &frame_40ms),
        ] {
            let config = OpusConfig::new(sample_rate, channels, bitrate, frame_size);
            let mut codec = OpusCodec::new(config).unwrap();

            let format = config.to_audio_format();
            let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

            let encoded = codec.encode(&input_buffer).unwrap();
            let decoded_buffer = codec.decode(&encoded).unwrap();

            assert_eq!(decoded_buffer.samples().len(), samples.len());
        }
    }

    #[test]
    fn test_opus_codec_roundtrip_different_sample_rates() {
        let channels = ChannelConfig::Mono;
        let bitrate = Bitrate::Kbps128;
        let frame_size = FrameSize::Ms20;

        let samples_44100 = create_test_samples(882);
        let samples_48000 = create_test_samples(960);

        let config_44100 = OpusConfig::new(SampleRate::Hz44100, channels, bitrate, frame_size);
        let mut codec_44100 = OpusCodec::new(config_44100).unwrap();

        let format_44100 = config_44100.to_audio_format();
        let input_buffer_44100 = AudioBuffer::new(samples_44100.clone(), format_44100).unwrap();

        let encoded_44100 = codec_44100.encode(&input_buffer_44100).unwrap();
        let decoded_44100 = codec_44100.decode(&encoded_44100).unwrap();
        assert_eq!(decoded_44100.samples().len(), samples_44100.len());

        let config_48000 = OpusConfig::new(SampleRate::Hz48000, channels, bitrate, frame_size);
        let mut codec_48000 = OpusCodec::new(config_48000).unwrap();

        let format_48000 = config_48000.to_audio_format();
        let input_buffer_48000 = AudioBuffer::new(samples_48000.clone(), format_48000).unwrap();

        let encoded_48000 = codec_48000.encode(&input_buffer_48000).unwrap();
        let decoded_48000 = codec_48000.decode(&encoded_48000).unwrap();
        assert_eq!(decoded_48000.samples().len(), samples_48000.len());
    }

    #[test]
    fn test_audio_codec_default() {
        let mut codec = AudioCodec::new().unwrap();
        let samples = create_test_samples(960);
        let format = codec.opus.config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded_buffer = codec.decode(&encoded).unwrap();

        assert_eq!(decoded_buffer.samples().len(), samples.len());
    }

    #[test]
    fn test_audio_codec_with_config() {
        let config = OpusConfig::new(
            SampleRate::Hz44100,
            ChannelConfig::Stereo,
            Bitrate::Kbps256,
            FrameSize::Ms40,
        );
        let mut codec = AudioCodec::with_config(config).unwrap();

        let samples = create_test_samples(3528);
        let format = codec.opus.config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded_buffer = codec.decode(&encoded).unwrap();

        assert_eq!(decoded_buffer.samples().len(), samples.len());
    }

    #[test]
    fn test_encode_buffer_size_mismatch() {
        let config = OpusConfig::default();
        let mut encoder = OpusEncoderCodec::new(config).unwrap();

        let samples = create_test_samples(100);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples, format).unwrap();

        let result = encoder.encode(&input_buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_opus_application_voip() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms20,
        )
        .with_application(opus::Application::Voip);

        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_test_samples(960);
        let format = codec.config().to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn test_opus_application_lowdelay() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms10,
        )
        .with_application(opus::Application::LowDelay);

        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_test_samples(480);
        let format = codec.config().to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn test_encode_interleaved() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );

        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let samples = create_test_samples(1920);

        let encoded = encoder.encode_interleaved(&samples).unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_decode_into() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms20,
        );

        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_test_samples(960);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = encoder.encode(&input_buffer).unwrap();

        let mut output = vec![0f32; 960];
        let decoded_count = decoder.decode_into(&encoded, &mut output).unwrap();

        assert_eq!(decoded_count, 960);
        assert_ne!(output, vec![0.0f32; 960], "decode_into must not produce silence");
    }

    #[test]
    fn test_decode_not_silence() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );

        let mut codec = OpusCodec::new(config).unwrap();

        let samples = create_test_samples(960);
        let format = config.to_audio_format();
        let input_buffer = AudioBuffer::new(samples.clone(), format).unwrap();

        let encoded = codec.encode(&input_buffer).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        let max_amplitude = decoded
            .samples()
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_amplitude > 0.01,
            "decoded audio must have non-trivial amplitude, got max={}",
            max_amplitude
        );
    }
}
