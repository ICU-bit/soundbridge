//! Integration tests for audio-codec crate.
//!
//! Tests the public API from an external perspective, verifying
//! cross-component interactions and real-world usage patterns.

use audio_codec::*;
use audio_core::{AudioBuffer, SampleFormat};
use opus::Application;

// ============================================================================
// Helper functions
// ============================================================================

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

// ============================================================================
// Cross-encoder-decoder roundtrip tests
// ============================================================================

#[test]
fn test_separate_encoder_decoder_roundtrip() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();

    // Create separate encoder and decoder
    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    // Encode
    let samples = create_sine_samples(960, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();
    assert!(!encoded.is_empty());

    // Decode
    let decoded = decoder.decode(&encoded).unwrap();
    assert_eq!(decoded.samples().len(), 960);
    assert_not_silence(decoded.samples(), "separate encoder/decoder roundtrip");
}

#[test]
fn test_separate_encoder_decoder_stereo() {
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
    let decoded = decoder.decode(&encoded).unwrap();

    assert_eq!(decoded.samples().len(), 1920);
    assert_not_silence(decoded.samples(), "stereo separate roundtrip");
}

// ============================================================================
// Multiple frames tests
// ============================================================================

#[test]
fn test_multiple_frames_sequential() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();

    let mut codec = OpusCodec::new(config).unwrap();

    // Encode/decode 10 frames
    for frame_idx in 0..10 {
        let freq = 440.0 + (frame_idx as f32 * 50.0);
        let samples = create_sine_samples(960, freq, 48000.0);
        let input = AudioBuffer::new(samples, format).unwrap();

        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(decoded.samples().len(), 960);
        assert_not_silence(decoded.samples(), &format!("frame {}", frame_idx));
    }
}

#[test]
fn test_multiple_frames_different_configs() {
    let configs = vec![
        OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps64,
            FrameSize::Ms10,
        ),
        OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Mono,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        ),
        OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps256,
            FrameSize::Ms20,
        ),
    ];

    for config in configs {
        let format = config.to_audio_format();
        let mut codec = OpusCodec::new(config).unwrap();

        let count = config.total_samples();
        let samples = if config.channels == ChannelConfig::Stereo {
            create_stereo_samples(count)
        } else {
            create_sine_samples(count, 440.0, 48000.0)
        };

        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(decoded.samples().len(), count);
        assert_not_silence(
            decoded.samples(),
            &format!("config {:?}/{:?}", config.sample_rate, config.channels),
        );
    }
}

// ============================================================================
// Zero-copy decode_into tests
// ============================================================================

#[test]
fn test_decode_into_preallocated_buffer() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();

    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    // Pre-allocate output buffer
    let mut output = vec![0f32; 960];

    // Encode
    let samples = create_sine_samples(960, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();

    // Decode into pre-allocated buffer
    let count = decoder.decode_into(&encoded, &mut output).unwrap();
    assert_eq!(count, 960);
    assert_not_silence(&output, "decode_into pre-allocated buffer");
}

#[test]
fn test_decode_into_stereo_preallocated() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Stereo,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();

    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    let mut output = vec![0f32; 1920];

    let samples = create_stereo_samples(1920);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();

    let count = decoder.decode_into(&encoded, &mut output).unwrap();
    assert_eq!(count, 1920);
    assert_not_silence(&output, "stereo decode_into");
}

// ============================================================================
// Error propagation tests
// ============================================================================

#[test]
fn test_codec_error_display() {
    let err = CodecError::InvalidSampleRate(22050);
    assert!(err.to_string().contains("22050"));

    let err = CodecError::BufferSizeMismatch {
        expected: 960,
        actual: 100,
    };
    assert!(err.to_string().contains("960"));
    assert!(err.to_string().contains("100"));
}

#[test]
fn test_invalid_sample_rate_error() {
    let result = SampleRate::from_u32(22050);
    assert!(result.is_err());
    match result.unwrap_err() {
        CodecError::InvalidSampleRate(22050) => {}
        _ => panic!("Expected InvalidSampleRate error"),
    }
}

#[test]
fn test_encode_buffer_size_mismatch_error() {
    let config = OpusConfig::default();
    let format = config.to_audio_format();
    let mut encoder = OpusEncoderCodec::new(config).unwrap();

    // Wrong size: should be 960, giving 100
    let samples = create_sine_samples(100, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let result = encoder.encode(&input);

    assert!(result.is_err());
    match result.unwrap_err() {
        CodecError::BufferSizeMismatch { expected, actual } => {
            assert_eq!(expected, 960);
            assert_eq!(actual, 100);
        }
        _ => panic!("Expected BufferSizeMismatch error"),
    }
}

#[test]
fn test_decode_into_buffer_too_small_error() {
    let config = OpusConfig::default();
    let format = config.to_audio_format();
    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    let samples = create_sine_samples(960, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();

    // Buffer too small: should be 960, giving 100
    let mut output = vec![0f32; 100];
    let result = decoder.decode_into(&encoded, &mut output);

    assert!(result.is_err());
    match result.unwrap_err() {
        CodecError::BufferSizeMismatch { expected, actual } => {
            assert_eq!(expected, 960);
            assert_eq!(actual, 100);
        }
        _ => panic!("Expected BufferSizeMismatch error"),
    }
}

// ============================================================================
// Config builder pattern tests
// ============================================================================

#[test]
fn test_config_builder_with_application() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps64,
        FrameSize::Ms20,
    )
    .with_application(Application::Voip);

    assert_eq!(config.application, Application::Voip);
    assert_eq!(config.sample_rate, SampleRate::Hz48000);
    assert_eq!(config.channels, ChannelConfig::Mono);
    assert_eq!(config.bitrate, Bitrate::Kbps64);
    assert_eq!(config.frame_size, FrameSize::Ms20);
}

#[test]
fn test_config_builder_lowdelay() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps64,
        FrameSize::Ms10,
    )
    .with_application(Application::LowDelay);

    assert_eq!(config.application, Application::LowDelay);
    assert_eq!(config.frame_size, FrameSize::Ms10);
    assert_eq!(config.frame_size_samples(), 480);
}

// ============================================================================
// AudioFormat conversion tests
// ============================================================================

#[test]
fn test_config_to_audio_format_mono() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();

    assert_eq!(format.sample_rate, 48000);
    assert_eq!(format.channels, 1);
    assert_eq!(format.sample_format, SampleFormat::F32);
}

#[test]
fn test_config_to_audio_format_stereo() {
    let config = OpusConfig::new(
        SampleRate::Hz44100,
        ChannelConfig::Stereo,
        Bitrate::Kbps256,
        FrameSize::Ms40,
    );
    let format = config.to_audio_format();

    assert_eq!(format.sample_rate, 44100);
    assert_eq!(format.channels, 2);
    assert_eq!(format.sample_format, SampleFormat::F32);
}

// ============================================================================
// Encoder bitrate change tests
// ============================================================================

#[test]
fn test_encoder_bitrate_change_with_encoding() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();
    let mut encoder = OpusEncoderCodec::new(config).unwrap();

    // Initial bitrate
    assert_eq!(encoder.bitrate(), Bitrate::Kbps128);

    // Change to 64kbps and encode
    encoder.set_bitrate(Bitrate::Kbps64).unwrap();
    assert_eq!(encoder.bitrate(), Bitrate::Kbps64);

    let samples = create_sine_samples(960, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();
    assert!(!encoded.is_empty());

    // Change to 256kbps and encode
    encoder.set_bitrate(Bitrate::Kbps256).unwrap();
    assert_eq!(encoder.bitrate(), Bitrate::Kbps256);

    let samples = create_sine_samples(960, 880.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = encoder.encode(&input).unwrap();
    assert!(!encoded.is_empty());
}

// ============================================================================
// AudioCodec convenience wrapper tests
// ============================================================================

#[test]
fn test_audio_codec_with_config() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Stereo,
        Bitrate::Kbps256,
        FrameSize::Ms20,
    );
    let format = config.to_audio_format();
    let mut codec = AudioCodec::with_config(config).unwrap();

    let samples = create_stereo_samples(1920);
    let input = AudioBuffer::new(samples, format).unwrap();
    let encoded = codec.encode(&input).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(decoded.samples().len(), 1920);
    assert_not_silence(decoded.samples(), "AudioCodec::with_config");
}

#[test]
fn test_audio_codec_default_roundtrip() {
    let mut codec = AudioCodec::new().unwrap();
    let config = OpusConfig::default();
    let format = config.to_audio_format();

    let samples = create_sine_samples(960, 440.0, 48000.0);
    let input = AudioBuffer::new(samples, format).unwrap();

    let encoded = codec.encode(&input).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(decoded.samples().len(), 960);
    assert_not_silence(decoded.samples(), "AudioCodec::default roundtrip");
}

// ============================================================================
// OpusCodec encode_decode convenience test
// ============================================================================

#[test]
fn test_opus_codec_encode_decode() {
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

    // Use encode_decode convenience method
    let decoded = codec.encode_decode(&input).unwrap();

    assert_eq!(decoded.samples().len(), 960);
    assert_not_silence(decoded.samples(), "OpusCodec::encode_decode");
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_encode_interleaved_into() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let mut encoder = OpusEncoderCodec::new(config).unwrap();

    let samples = create_sine_samples(960, 440.0, 48000.0);
    let mut i16_buf = vec![0i16; 960];
    let mut opus_buf = vec![0u8; 1500];

    let encoded_len = encoder
        .encode_interleaved_into(&samples, &mut i16_buf, &mut opus_buf)
        .unwrap();

    assert!(encoded_len > 0);
    assert!(encoded_len <= 1500);
}

#[test]
fn test_encode_interleaved_into_stereo() {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        ChannelConfig::Stereo,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let mut encoder = OpusEncoderCodec::new(config).unwrap();

    let samples = create_stereo_samples(1920);
    let mut i16_buf = vec![0i16; 1920];
    let mut opus_buf = vec![0u8; 3000];

    let encoded_len = encoder
        .encode_interleaved_into(&samples, &mut i16_buf, &mut opus_buf)
        .unwrap();

    assert!(encoded_len > 0);
    assert!(encoded_len <= 3000);
}
