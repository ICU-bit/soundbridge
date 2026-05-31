pub mod fec;

use audio_core::{AudioBuffer, AudioFormat, SampleFormat};
use opus::{
    Application, Bitrate as OpusBitrate, Channels as OpusChannels, Decoder as OpusDecoder,
    Encoder as OpusEncoder,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("opus encoder error: {0}")]
    EncoderError(String),
    #[error("opus decoder error: {0}")]
    DecoderError(String),
    #[error("invalid sample rate: {0}")]
    InvalidSampleRate(u32),
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
    Kbps96 = 96000,
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

#[derive(Debug, Clone, Copy)]
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

    pub fn total_samples(&self) -> usize {
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

#[allow(dead_code)]
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

    /// 运行时动态调整码率（带宽自适应）
    pub fn set_bitrate(&mut self, bitrate: Bitrate) -> Result<()> {
        self.config.bitrate = bitrate;
        self.apply_bitrate()
    }

    /// 获取当前码率
    pub fn bitrate(&self) -> Bitrate {
        self.config.bitrate
    }

    fn encode_samples(&mut self, samples: &[f32]) -> Result<Vec<u8>> {
        let expected = self.config.total_samples();
        if samples.len() != expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: samples.len(),
            });
        }

        // 转换 f32 到 i16
        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        let frame_size = self.config.frame_size_samples();
        let encoded = self
            .encoder
            .encode_vec(&samples_i16, frame_size)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))?;

        Ok(encoded)
    }

    pub fn encode(&mut self, buffer: &AudioBuffer<f32>) -> Result<Vec<u8>> {
        self.encode_samples(buffer.samples())
    }

    pub fn encode_interleaved(&mut self, samples: &[f32]) -> Result<Vec<u8>> {
        self.encode_samples(samples)
    }

    /// 编码音频数据到预分配缓冲区（零分配版本）
    pub fn encode_interleaved_into(
        &mut self,
        samples: &[f32],
        i16_buf: &mut [i16],
        output: &mut [u8],
    ) -> Result<usize> {
        let expected = self.config.total_samples();
        if samples.len() != expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: samples.len(),
            });
        }

        if i16_buf.len() < expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: i16_buf.len(),
            });
        }

        // 转换 f32 到 i16（使用预分配缓冲区）
        for (dst, &src) in i16_buf.iter_mut().zip(samples.iter()) {
            *dst = (src * 32767.0).clamp(-32768.0, 32767.0) as i16;
        }

        let encoded_len = self
            .encoder
            .encode(&i16_buf[..expected], output)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))?;

        Ok(encoded_len)
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
        let expected = self.config.total_samples();
        let channels = self.config.channels.count() as usize;

        let mut output = vec![0.0f32; expected];
        let decoded_count = self
            .decoder
            .decode_float(data, &mut output, false)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        // decoded_count 是每通道样本数，需要乘以通道数
        let total_decoded = decoded_count * channels;
        output.truncate(total_decoded);

        let format = self.config.to_audio_format();
        let audio_buffer = AudioBuffer::new(output, format)
            .map_err(|_| CodecError::DecodingFailed("failed to create audio buffer".to_string()))?;

        Ok(audio_buffer)
    }

    pub fn decode_into(&mut self, data: &[u8], output: &mut [f32]) -> Result<usize> {
        let expected = self.config.total_samples();
        let channels = self.config.channels.count() as usize;

        if output.len() < expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: output.len(),
            });
        }

        let decoded_count = self
            .decoder
            .decode_float(data, output, false)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        // 返回总样本数（每通道样本数 * 通道数）
        Ok(decoded_count * channels)
    }

    pub fn config(&self) -> &OpusConfig {
        &self.config
    }
}

pub struct OpusCodec {
    encoder: OpusEncoderCodec,
    decoder: OpusDecoderCodec,
    config: OpusConfig,
}

impl OpusCodec {
    pub fn new(config: OpusConfig) -> Result<Self> {
        let encoder = OpusEncoderCodec::new(config)?;
        let decoder = OpusDecoderCodec::new(config)?;
        Ok(Self {
            encoder,
            decoder,
            config,
        })
    }

    pub fn encode(&mut self, buffer: &AudioBuffer<f32>) -> Result<Vec<u8>> {
        self.encoder.encode(buffer)
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<AudioBuffer<f32>> {
        self.decoder.decode(data)
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

    fn create_sine_samples(count: usize, freq: f32, sample_rate: f32) -> Vec<f32> {
        (0..count)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    fn create_stereo_samples(count: usize) -> Vec<f32> {
        let per_ch = count / 2;
        let left = create_sine_samples(per_ch, 440.0, 48000.0);
        let right = create_sine_samples(per_ch, 880.0, 48000.0);
        let mut interleaved = Vec::with_capacity(count);
        for i in 0..per_ch {
            interleaved.push(left[i]);
            interleaved.push(right[i]);
        }
        interleaved
    }

    fn assert_not_silence(samples: &[f32], label: &str) {
        let max_amp = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp > 0.01,
            "{}: decoded audio must not be silence, max_amplitude={}",
            label,
            max_amp
        );
    }

    #[test]
    fn test_config_default() {
        let config = OpusConfig::default();
        assert_eq!(config.sample_rate, SampleRate::Hz48000);
        assert_eq!(config.channels, ChannelConfig::Mono);
        assert_eq!(config.bitrate, Bitrate::Kbps128);
        assert_eq!(config.frame_size, FrameSize::Ms20);
    }

    #[test]
    fn test_config_custom() {
        let config = OpusConfig::new(
            SampleRate::Hz44100,
            ChannelConfig::Stereo,
            Bitrate::Kbps256,
            FrameSize::Ms40,
        );
        assert_eq!(config.sample_rate, SampleRate::Hz44100);
        assert_eq!(config.channels, ChannelConfig::Stereo);
        assert_eq!(config.total_samples(), 1764 * 2);
    }

    #[test]
    fn test_sample_rate_conversion() {
        assert_eq!(SampleRate::Hz44100.value(), 44100);
        assert_eq!(SampleRate::Hz48000.value(), 48000);
        assert_eq!(SampleRate::from_u32(44100).unwrap(), SampleRate::Hz44100);
        assert!(SampleRate::from_u32(22050).is_err());
    }

    #[test]
    fn test_frame_size_samples() {
        assert_eq!(FrameSize::Ms10.samples(SampleRate::Hz48000), 480);
        assert_eq!(FrameSize::Ms20.samples(SampleRate::Hz48000), 960);
        assert_eq!(FrameSize::Ms40.samples(SampleRate::Hz48000), 1920);
        assert_eq!(FrameSize::Ms20.samples(SampleRate::Hz44100), 882);
    }

    #[test]
    fn test_encode_decode_mono() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_sine_samples(960, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();

        let encoded = codec.encode(&input).unwrap();
        assert!(!encoded.is_empty());

        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 960);
        assert_not_silence(decoded.samples(), "mono encode/decode");
    }

    #[test]
    fn test_encode_decode_stereo() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_stereo_samples(1920);
        let input = AudioBuffer::new(samples, format).unwrap();

        let encoded = codec.encode(&input).unwrap();
        assert!(!encoded.is_empty());

        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 1920);
        assert_not_silence(decoded.samples(), "stereo encode/decode");
    }

    #[test]
    fn test_roundtrip_different_bitrates() {
        let samples = create_sine_samples(960, 440.0, 48000.0);
        for bitrate in [Bitrate::Kbps64, Bitrate::Kbps128, Bitrate::Kbps256] {
            let config = OpusConfig::new(
                SampleRate::Hz48000,
                ChannelConfig::Mono,
                bitrate,
                FrameSize::Ms20,
            );
            let format = config.to_audio_format();
            let mut codec = OpusCodec::new(config).unwrap();
            let input = AudioBuffer::new(samples.clone(), format).unwrap();
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded).unwrap();
            assert_eq!(decoded.samples().len(), 960);
            assert_not_silence(decoded.samples(), &format!("bitrate {:?}", bitrate));
        }
    }

    #[test]
    fn test_roundtrip_different_frame_sizes() {
        for (frame_size, count) in [
            (FrameSize::Ms10, 480),
            (FrameSize::Ms20, 960),
            (FrameSize::Ms40, 1920),
        ] {
            let config = OpusConfig::new(
                SampleRate::Hz48000,
                ChannelConfig::Mono,
                Bitrate::Kbps128,
                frame_size,
            );
            let format = config.to_audio_format();
            let mut codec = OpusCodec::new(config).unwrap();
            let samples = create_sine_samples(count, 440.0, 48000.0);
            let input = AudioBuffer::new(samples, format).unwrap();
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded).unwrap();
            assert_eq!(decoded.samples().len(), count);
        }
    }

    #[test]
    fn test_roundtrip_stereo_different_bitrates() {
        let samples = create_stereo_samples(1920);
        for bitrate in [Bitrate::Kbps64, Bitrate::Kbps128, Bitrate::Kbps256] {
            let config = OpusConfig::new(
                SampleRate::Hz48000,
                ChannelConfig::Stereo,
                bitrate,
                FrameSize::Ms20,
            );
            let format = config.to_audio_format();
            let mut codec = OpusCodec::new(config).unwrap();
            let input = AudioBuffer::new(samples.clone(), format).unwrap();
            let encoded = codec.encode(&input).unwrap();
            let decoded = codec.decode(&encoded).unwrap();
            assert_eq!(decoded.samples().len(), 1920);
            assert_not_silence(decoded.samples(), &format!("stereo bitrate {:?}", bitrate));
        }
    }

    #[test]
    fn test_roundtrip_different_sample_rates() {
        // Opus 支持的采样率�?000, 12000, 16000, 24000, 48000
        let sr = SampleRate::Hz48000;
        let count = FrameSize::Ms20.samples(sr);
        let config = OpusConfig::new(sr, ChannelConfig::Mono, Bitrate::Kbps128, FrameSize::Ms20);
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_sine_samples(count, 440.0, sr.value() as f32);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), count);
    }

    #[test]
    fn test_application_voip() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms20,
        )
        .with_application(Application::Voip);
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_sine_samples(960, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 960);
    }

    #[test]
    fn test_application_lowdelay() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms10,
        )
        .with_application(Application::LowDelay);
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();
        let samples = create_sine_samples(480, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 480);
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
        let samples = create_stereo_samples(1920);
        let encoded = encoder.encode_interleaved(&samples).unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_decode_into_mono() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = config.to_audio_format();
        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_sine_samples(960, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = encoder.encode(&input).unwrap();

        let mut output = vec![0f32; 960];
        let count = decoder.decode_into(&encoded, &mut output).unwrap();
        assert_eq!(count, 960);
        assert_not_silence(&output, "decode_into mono");
    }

    #[test]
    fn test_decode_into_stereo() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = config.to_audio_format();
        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_stereo_samples(1920);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = encoder.encode(&input).unwrap();

        let mut output = vec![0f32; 1920];
        let count = decoder.decode_into(&encoded, &mut output).unwrap();
        assert_eq!(count, 1920);
        assert_not_silence(&output, "decode_into stereo");
    }

    #[test]
    fn test_decode_into_buffer_too_small() {
        let config = OpusConfig::default();
        let format = config.to_audio_format();
        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let mut decoder = OpusDecoderCodec::new(config).unwrap();

        let samples = create_sine_samples(960, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = encoder.encode(&input).unwrap();

        let mut output = vec![0f32; 100];
        let result = decoder.decode_into(&encoded, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_buffer_size_mismatch() {
        let config = OpusConfig::default();
        let format = config.to_audio_format();
        let mut encoder = OpusEncoderCodec::new(config).unwrap();
        let samples = create_sine_samples(100, 440.0, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();
        let result = encoder.encode(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_codec_default() {
        let mut codec = AudioCodec::new().unwrap();
        let samples = create_sine_samples(960, 440.0, 48000.0);
        let format = codec.opus.config.to_audio_format();
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 960);
        assert_not_silence(decoded.samples(), "AudioCodec default");
    }

    #[test]
    fn test_audio_codec_stereo() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let mut codec = AudioCodec::with_config(config).unwrap();
        let samples = create_stereo_samples(1920);
        let format = codec.opus.config.to_audio_format();
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.samples().len(), 1920);
        assert_not_silence(decoded.samples(), "AudioCodec stereo");
    }

    #[test]
    fn test_encoder_runtime_bitrate_change() {
        let config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let mut codec = OpusEncoderCodec::new(config).unwrap();

        // 初始码率
        assert_eq!(codec.bitrate(), Bitrate::Kbps128);

        // 动态调整到 64kbps
        codec.set_bitrate(Bitrate::Kbps64).unwrap();
        assert_eq!(codec.bitrate(), Bitrate::Kbps64);

        // 动态调整到 96kbps
        codec.set_bitrate(Bitrate::Kbps96).unwrap();
        assert_eq!(codec.bitrate(), Bitrate::Kbps96);

        // 动态调整到 256kbps
        codec.set_bitrate(Bitrate::Kbps256).unwrap();
        assert_eq!(codec.bitrate(), Bitrate::Kbps256);

        // 调整码率后仍然能正常编码
        let samples = create_sine_samples(960, 440.0, 48000.0);
        let mut i16_buf = vec![0i16; 960];
        let mut opus_buf = vec![0u8; 1500];
        let encoded_len = codec
            .encode_interleaved_into(&samples, &mut i16_buf, &mut opus_buf)
            .unwrap();
        assert!(encoded_len > 0);
    }

    #[test]
    fn test_bitrate_kbps96_value() {
        assert_eq!(Bitrate::Kbps96 as i32, 96000);
        assert_eq!(Bitrate::Kbps96.bits_per_second(), 96000);
    }
}
