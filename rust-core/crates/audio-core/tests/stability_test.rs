//! SoundBridge 稳定性测试
//!
//! 测试长时间运行的稳定性。

use audio_core::{AudioBuffer, AudioFormat, SampleFormat, RingBuffer};
use std::time::{Duration, Instant};

/// 测试 Ring Buffer 长时间运行稳定性
#[test]
fn test_ring_buffer_stability() {
    let rb = RingBuffer::<f32>::new(960 * 4);
    let data = vec![0.5f32; 960];
    let mut output = vec![0.0f32; 960];

    let start = Instant::now();
    let duration = Duration::from_secs(5); // 运行 5 秒
    let mut iterations = 0;

    while start.elapsed() < duration {
        // 写入
        let written = rb.write(&data);
        assert_eq!(written, 960);

        // 读取
        let read = rb.read(&mut output);
        assert_eq!(read, 960);

        // 验证数据
        for (out, &inp) in output.iter().zip(data.iter()) {
            assert!((out - inp).abs() < 0.001);
        }

        iterations += 1;
    }

    println!("Ring buffer stability: {} iterations in {:?}", iterations, start.elapsed());
    assert!(iterations > 1000, "Too few iterations: {}", iterations);
}

/// 测试音频缓冲区创建和销毁稳定性
#[test]
fn test_audio_buffer_stability() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let start = Instant::now();
    let duration = Duration::from_secs(5);
    let mut iterations = 0;

    while start.elapsed() < duration {
        let samples = vec![0.5f32; 960];
        let buffer = AudioBuffer::new(samples, format).unwrap();

        assert_eq!(buffer.sample_count(), 960);
        assert_eq!(buffer.frame_count(), 960);

        let _cloned = buffer.clone();

        iterations += 1;
    }

    println!("Audio buffer stability: {} iterations in {:?}", iterations, start.elapsed());
    assert!(iterations > 1000, "Too few iterations: {}", iterations);
}

/// 测试音频缓冲区格式操作稳定性
#[test]
fn test_audio_format_stability() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    let start = Instant::now();
    let duration = Duration::from_secs(5);
    let mut iterations = 0;

    while start.elapsed() < duration {
        assert_eq!(format.sample_rate, 48000);
        assert_eq!(format.channels, 1);
        assert_eq!(format.sample_format, SampleFormat::F32);
        iterations += 1;
    }

    println!("Audio format stability: {} iterations in {:?}", iterations, start.elapsed());
    assert!(iterations > 1000, "Too few iterations: {}", iterations);
}

/// 测试 Ring Buffer 边界条件稳定性
#[test]
fn test_ring_buffer_edge_cases() {
    let rb = RingBuffer::<f32>::new(4);
    let data = vec![0.5f32; 4];
    let mut output = vec![0.0f32; 4];

    let start = Instant::now();
    let duration = Duration::from_secs(5);
    let mut iterations = 0;

    while start.elapsed() < duration {
        // 写入满
        let written = rb.write(&data);
        assert_eq!(written, 4);
        assert!(rb.is_full());

        // 读取空
        let read = rb.read(&mut output);
        assert_eq!(read, 4);
        assert!(rb.is_empty());

        // 验证数据
        for (out, &inp) in output.iter().zip(data.iter()) {
            assert!((out - inp).abs() < 0.001);
        }

        iterations += 1;
    }

    println!("Ring buffer edge cases: {} iterations in {:?}", iterations, start.elapsed());
    assert!(iterations > 1000, "Too few iterations: {}", iterations);
}
