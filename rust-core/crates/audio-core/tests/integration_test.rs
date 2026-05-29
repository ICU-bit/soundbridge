//! SoundBridge 集成测试
//!
//! 测试端到端音频管线。

use audio_core::{AudioBuffer, AudioFormat, SampleFormat};

/// 测试音频格式
#[test]
fn test_audio_format() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    assert_eq!(format.sample_rate, 48000);
    assert_eq!(format.channels, 1);
    assert_eq!(format.sample_format, SampleFormat::F32);
}

/// 测试音频缓冲区创建
#[test]
fn test_audio_buffer_creation() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![0.5f32; 960]; // 1 通道，960 帧
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();

    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.frame_count(), 960);
    assert_eq!(buffer.samples(), &samples[..]);
}

/// 测试音频缓冲区格式
#[test]
fn test_audio_buffer_format() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![0.0f32; 960];
    let buffer = AudioBuffer::new(samples, format).unwrap();

    assert_eq!(buffer.format().sample_rate, 48000);
    assert_eq!(buffer.format().channels, 1);
    assert_eq!(buffer.format().sample_format, SampleFormat::F32);
}

/// 测试音频缓冲区字节表示
#[test]
fn test_audio_buffer_bytes() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![1.0f32, 2.0, 3.0, 4.0];
    let buffer = AudioBuffer::new(samples, format).unwrap();

    // f32 = 4 字节，4 个样本 = 16 字节
    assert_eq!(buffer.as_bytes().len(), 16);
}

/// 测试音频缓冲区克隆
#[test]
fn test_audio_buffer_clone() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![1.0f32, 2.0, 3.0];
    let buffer = AudioBuffer::new(samples, format).unwrap();
    let cloned = buffer.clone();

    assert_eq!(buffer.samples(), cloned.samples());
    assert_eq!(buffer.sample_count(), cloned.sample_count());
}
