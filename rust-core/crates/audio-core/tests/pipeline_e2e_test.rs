//! 端到端音频管线集成测试
//!
//! 测试 RingBuffer → AudioBuffer 完整链路
//! 不需要真实音频设备，使用模拟数据

use audio_core::{AudioBuffer, AudioFormat, SampleFormat, RingBuffer};

/// 测试 Ring Buffer 管线
#[test]
fn test_ring_buffer_pipeline() {
    let rb = RingBuffer::<f32>::new(960 * 4);

    let capture_data = vec![0.5f32; 960];
    let written = rb.write(&capture_data);
    assert_eq!(written, 960);

    let mut processed = vec![0.0f32; 960];
    let read = rb.read(&mut processed);
    assert_eq!(read, 960);

    for (out, &inp) in processed.iter().zip(capture_data.iter()) {
        assert!((out - inp).abs() < 0.001);
    }
}

/// 测试 AudioBuffer 创建和处理
#[test]
fn test_audio_buffer_pipeline() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };

    let samples: Vec<f32> = (0..1920)
        .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 48000.0).sin() * 0.5)
        .collect();

    let buffer = AudioBuffer::new(samples, format).unwrap();
    assert_eq!(buffer.sample_count(), 1920);
    assert_eq!(buffer.frame_count(), 960);

    let has_signal = buffer.samples().iter().any(|&s| s.abs() > 0.001);
    assert!(has_signal, "Buffer should contain signal");
}

/// 测试多帧处理稳定性
#[test]
fn test_multi_frame_stability() {
    let rb = RingBuffer::<f32>::new(1920 * 4);

    for frame in 0..100 {
        let samples: Vec<f32> = (0..1920)
            .map(|i| {
                let t = (frame * 1920 + i) as f32 / 48000.0;
                (t * 2.0 * std::f32::consts::PI * 440.0).sin() * 0.5
            })
            .collect();

        let written = rb.write(&samples);
        assert_eq!(written, 1920);

        let mut output = vec![0.0f32; 1920];
        let read = rb.read(&mut output);
        assert_eq!(read, 1920);

        for (out, &inp) in output.iter().zip(samples.iter()) {
            assert!((out - inp).abs() < 0.001);
        }
    }
}
