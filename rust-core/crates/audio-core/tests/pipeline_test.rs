//! SoundBridge 端到端音频管线测试
//!
//! 测试完整的音频处理流程。

use audio_core::{AudioBuffer, AudioFormat, SampleFormat, RingBuffer};

/// 测试 Ring Buffer 在音频管线中的使用
#[test]
fn test_ring_buffer_pipeline() {
    let rb = RingBuffer::<f32>::new(960 * 4);

    // 模拟采集线程写入
    let capture_data = vec![0.5f32; 960];
    let written = rb.write(&capture_data);
    assert_eq!(written, 960);

    // 模拟处理线程读取
    let mut processed = vec![0.0f32; 960];
    let read = rb.read(&mut processed);
    assert_eq!(read, 960);

    // 验证数据一致性
    for (out, &inp) in processed.iter().zip(capture_data.iter()) {
        assert!((out - inp).abs() < 0.001);
    }

    // 模拟混音输出写入
    let mixed_data = vec![0.8f32; 960];
    let written = rb.write(&mixed_data);
    assert_eq!(written, 960);

    // 模拟播放线程读取
    let mut playback = vec![0.0f32; 960];
    let read = rb.read(&mut playback);
    assert_eq!(read, 960);
}

/// 测试音频格式转换
#[test]
fn test_audio_format_conversion() {
    let format_mono = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![0.5f32; 960];
    let buffer = AudioBuffer::new(samples, format_mono).unwrap();

    assert_eq!(buffer.format().sample_rate, 48000);
    assert_eq!(buffer.format().channels, 1);
    assert_eq!(buffer.format().sample_format, SampleFormat::F32);
    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.frame_count(), 960);
}

/// 测试多路音频数据处理
#[test]
fn test_multi_stream_processing() {
    let rb1 = RingBuffer::<f32>::new(960 * 4);
    let rb2 = RingBuffer::<f32>::new(960 * 4);

    // 写入两路音频
    let stream1 = vec![0.5f32; 960];
    let stream2 = vec![0.3f32; 960];

    rb1.write(&stream1);
    rb2.write(&stream2);

    // 读取并混音
    let mut buf1 = vec![0.0f32; 960];
    let mut buf2 = vec![0.0f32; 960];

    rb1.read(&mut buf1);
    rb2.read(&mut buf2);

    // 简单混音：加权求和
    let mut mixed = vec![0.0f32; 960];
    for i in 0..960 {
        mixed[i] = buf1[i] * 0.7 + buf2[i] * 0.5;
    }

    // 验证混音结果
    for sample in &mixed {
        assert!(*sample > 0.0, "Mixed output should not be silent");
        assert!(*sample < 2.0, "Mixed output should not clip");
    }
}

/// 测试音频缓冲区生命周期
#[test]
fn test_audio_buffer_lifecycle() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let samples = vec![0.5f32; 960];
    let buffer = AudioBuffer::new(samples, format).unwrap();

    let cloned = buffer.clone();
    assert_eq!(buffer.samples(), cloned.samples());

    let bytes = buffer.as_bytes();
    assert_eq!(bytes.len(), 960 * 4);

    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.frame_count(), 960);
}

/// 测试 Ring Buffer 边界条件
#[test]
fn test_ring_buffer_boundary() {
    let rb = RingBuffer::<f32>::new(4);
    let data = vec![0.5f32; 4];
    let mut output = vec![0.0f32; 4];

    let written = rb.write(&data);
    assert_eq!(written, 4);
    assert!(rb.is_full());

    let read = rb.read(&mut output);
    assert_eq!(read, 4);
    assert!(rb.is_empty());

    for (out, &inp) in output.iter().zip(data.iter()) {
        assert!((out - inp).abs() < 0.001);
    }
}
