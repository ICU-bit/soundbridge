//! 端到端音频管线测试
//!
//! 测试 capture → encode → decode → playback 完整链路

use audio_capture::{CaptureDevice, CaptureConfig};
use audio_playback::{PlaybackDevice, PlaybackConfig};
use audio_codec::{OpusEncoderCodec, OpusDecoderCodec, OpusConfig, SampleRate, ChannelConfig, Bitrate, FrameSize};
use audio_core::{AudioBuffer, AudioFormat, SampleFormat};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_encode_decode_loopback() {
        // 创建配置
        let opus_config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            sample_format: SampleFormat::F32,
        };

        // 创建编码器和解码器
        let mut encoder = OpusEncoderCodec::new(opus_config.clone()).unwrap();
        let mut decoder = OpusDecoderCodec::new(opus_config).unwrap();

        // 模拟采集数据（正弦波）
        let samples: Vec<f32> = (0..1920)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 48000.0).sin() * 0.5)
            .collect();

        // 编码
        let input = AudioBuffer::new(samples, format).unwrap();
        let encoded = encoder.encode(&input).unwrap();
        assert!(!encoded.is_empty(), "Encoded data should not be empty");

        // 解码
        let decoded = decoder.decode(&encoded).unwrap();
        assert_eq!(decoded.sample_count(), 1920, "Decoded sample count should match");

        // 验证解码后的数据不全为零
        let decoded_samples = decoded.samples();
        let has_signal = decoded_samples.iter().any(|&s| s.abs() > 0.001);
        assert!(has_signal, "Decoded audio should contain signal");
    }

    #[test]
    fn test_multiple_frames_pipeline() {
        let opus_config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            sample_format: SampleFormat::F32,
        };

        let mut encoder = OpusEncoderCodec::new(opus_config.clone()).unwrap();
        let mut decoder = OpusDecoderCodec::new(opus_config).unwrap();

        // 处理 10 帧音频
        for frame in 0..10 {
            let samples: Vec<f32> = (0..1920)
                .map(|i| {
                    let t = (frame * 1920 + i) as f32 / 48000.0;
                    (t * 2.0 * std::f32::consts::PI * 440.0).sin() * 0.5
                })
                .collect();

            let input = AudioBuffer::new(samples, format).unwrap();
            let encoded = encoder.encode(&input).unwrap();
            let decoded = decoder.decode(&encoded).unwrap();

            assert_eq!(decoded.sample_count(), 1920);
            let has_signal = decoded.samples().iter().any(|&s| s.abs() > 0.001);
            assert!(has_signal, "Frame {} should contain signal", frame);
        }
    }
}
