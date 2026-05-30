//! Integration tests for audio-core crate.
//!
//! Tests all public API types: SampleFormat, AudioFormat, AudioError,
//! Sample trait, AudioBuffer<T>, RingBuffer<T>.
//! Covers multiple sample types (i16, i32, f32, f64), edge cases, and cross-component interactions.

use audio_core::*;

// ============================================================================
// SampleFormat tests
// ============================================================================

#[test]
fn test_sample_format_variants() {
    assert_eq!(SampleFormat::I16, SampleFormat::I16);
    assert_eq!(SampleFormat::I32, SampleFormat::I32);
    assert_eq!(SampleFormat::F32, SampleFormat::F32);
    assert_eq!(SampleFormat::F64, SampleFormat::F64);
    assert_ne!(SampleFormat::I16, SampleFormat::F32);
}

#[test]
fn test_sample_format_debug() {
    let debug = format!("{:?}", SampleFormat::F32);
    assert!(debug.contains("F32"));
}

#[test]
fn test_sample_format_clone() {
    let fmt = SampleFormat::F32;
    let cloned = fmt;
    assert_eq!(fmt, cloned);
}

#[test]
fn test_sample_format_hash() {
    // SampleFormat implements Hash, verify it works in a set
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(SampleFormat::I16);
    set.insert(SampleFormat::F32);
    set.insert(SampleFormat::I16); // duplicate
    assert_eq!(set.len(), 2);
}

// ============================================================================
// AudioFormat tests
// ============================================================================

#[test]
fn test_audio_format_creation() {
    let fmt = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    assert_eq!(fmt.sample_rate, 48000);
    assert_eq!(fmt.channels, 1);
    assert_eq!(fmt.sample_format, SampleFormat::F32);
}

#[test]
fn test_audio_format_stereo() {
    let fmt = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };
    assert_eq!(fmt.channels, 2);
}

#[test]
fn test_audio_format_clone() {
    let fmt = AudioFormat {
        sample_rate: 44100,
        channels: 2,
        sample_format: SampleFormat::I16,
    };
    let cloned = fmt;
    assert_eq!(fmt.sample_rate, cloned.sample_rate);
    assert_eq!(fmt.channels, cloned.channels);
    assert_eq!(fmt.sample_format, cloned.sample_format);
}

#[test]
fn test_audio_format_debug() {
    let fmt = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let debug = format!("{:?}", fmt);
    assert!(debug.contains("48000"));
    assert!(debug.contains("AudioFormat"));
}

#[test]
fn test_audio_format_equality() {
    let fmt1 = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let fmt2 = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    assert_eq!(fmt1.sample_rate, fmt2.sample_rate);
    assert_eq!(fmt1.channels, fmt2.channels);
    assert_eq!(fmt1.sample_format, fmt2.sample_format);
}

// ============================================================================
// AudioError tests
// ============================================================================

#[test]
fn test_audio_error_display() {
    let err = AudioError::InvalidBufferSize;
    assert!(err.to_string().contains("invalid buffer size"));

    let err = AudioError::FormatMismatch;
    assert!(err.to_string().contains("format mismatch"));
}

#[test]
fn test_audio_error_debug() {
    let err = AudioError::InvalidBufferSize;
    let debug = format!("{:?}", err);
    assert!(debug.contains("InvalidBufferSize"));
}

// ============================================================================
// Sample trait tests
// ============================================================================

#[test]
fn test_sample_i16() {
    assert_eq!(i16::FORMAT, SampleFormat::I16);
}

#[test]
fn test_sample_i32() {
    assert_eq!(i32::FORMAT, SampleFormat::I32);
}

#[test]
fn test_sample_f32() {
    assert_eq!(f32::FORMAT, SampleFormat::F32);
}

#[test]
fn test_sample_f64() {
    assert_eq!(f64::FORMAT, SampleFormat::F64);
}

// ============================================================================
// AudioBuffer<f32> tests
// ============================================================================

#[test]
fn test_audio_buffer_f32_creation() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let samples = vec![0.5f32; 960];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.frame_count(), 960);
    assert_eq!(buffer.samples(), &samples[..]);
}

#[test]
fn test_audio_buffer_f32_format() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let buffer = AudioBuffer::new(vec![0.0f32; 960], format).unwrap();
    assert_eq!(buffer.format().sample_rate, 48000);
    assert_eq!(buffer.format().channels, 1);
    assert_eq!(buffer.format().sample_format, SampleFormat::F32);
}

#[test]
fn test_audio_buffer_f32_bytes() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let samples = vec![1.0f32, 2.0, 3.0, 4.0];
    let buffer = AudioBuffer::new(samples, format).unwrap();
    // f32 = 4 bytes, 4 samples = 16 bytes
    assert_eq!(buffer.as_bytes().len(), 16);
}

#[test]
fn test_audio_buffer_f32_clone() {
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

#[test]
fn test_audio_buffer_f32_stereo() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };
    // 960 frames * 2 channels = 1920 samples
    let samples = vec![0.5f32; 1920];
    let buffer = AudioBuffer::new(samples, format).unwrap();
    assert_eq!(buffer.sample_count(), 1920);
    assert_eq!(buffer.frame_count(), 960);
}

#[test]
fn test_audio_buffer_f32_empty() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let buffer: AudioBuffer<f32> = AudioBuffer::new(vec![], format).unwrap();
    assert_eq!(buffer.sample_count(), 0);
    assert_eq!(buffer.frame_count(), 0);
    assert_eq!(buffer.as_bytes().len(), 0);
}

#[test]
fn test_audio_buffer_f32_single_sample() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let buffer = AudioBuffer::new(vec![0.5f32], format).unwrap();
    assert_eq!(buffer.sample_count(), 1);
    assert_eq!(buffer.samples()[0], 0.5);
}

// ============================================================================
// AudioBuffer<i16> tests
// ============================================================================

#[test]
fn test_audio_buffer_i16_creation() {
    let format = AudioFormat {
        sample_rate: 44100,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
    let samples = vec![1000i16; 960];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.samples(), &samples[..]);
    assert_eq!(buffer.as_bytes().len(), 960 * 2); // i16 = 2 bytes
}

// ============================================================================
// AudioBuffer<i32> tests
// ============================================================================

#[test]
fn test_audio_buffer_i32_creation() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::I32,
    };
    let samples = vec![100000i32; 960];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.samples(), &samples[..]);
    assert_eq!(buffer.as_bytes().len(), 960 * 4); // i32 = 4 bytes
}

// ============================================================================
// AudioBuffer<f64> tests
// ============================================================================

#[test]
fn test_audio_buffer_f64_creation() {
    let format = AudioFormat {
        sample_rate: 96000,
        channels: 1,
        sample_format: SampleFormat::F64,
    };
    let samples = vec![0.5f64; 960];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.samples(), &samples[..]);
    assert_eq!(buffer.as_bytes().len(), 960 * 8); // f64 = 8 bytes
}

// ============================================================================
// RingBuffer tests
// ============================================================================

#[test]
fn test_ring_buffer_creation() {
    let rb = RingBuffer::<f32>::new(1024);
    assert_eq!(rb.capacity(), 1024);
    assert!(rb.is_empty());
    assert!(!rb.is_full());
}

#[test]
fn test_ring_buffer_power_of_two() {
    let rb = RingBuffer::<f32>::new(10);
    assert_eq!(rb.capacity(), 16); // rounds up to next power of 2
}

#[test]
fn test_ring_buffer_write_read() {
    let rb = RingBuffer::<f32>::new(16);
    let data = [1.0, 2.0, 3.0, 4.0];
    let written = rb.write(&data);
    assert_eq!(written, 4);
    assert_eq!(rb.available_read(), 4);

    let mut output = [0.0f32; 4];
    let read = rb.read(&mut output);
    assert_eq!(read, 4);
    assert_eq!(output, data);
    assert_eq!(rb.available_read(), 0);
}

#[test]
fn test_ring_buffer_full() {
    let rb = RingBuffer::<f32>::new(4);
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let written = rb.write(&data);
    assert_eq!(written, 4);
    assert!(rb.is_full());
}

#[test]
fn test_ring_buffer_empty_read() {
    let rb = RingBuffer::<f32>::new(16);
    let mut output = [0.0f32; 4];
    let read = rb.read(&mut output);
    assert_eq!(read, 0);
    assert!(rb.is_empty());
}

#[test]
fn test_ring_buffer_wrap_around() {
    let rb = RingBuffer::<f32>::new(4);
    rb.write(&[1.0, 2.0, 3.0, 4.0]);

    let mut output = [0.0f32; 2];
    rb.read(&mut output);
    assert_eq!(output, [1.0, 2.0]);

    rb.write(&[5.0, 6.0]);

    let mut output = [0.0f32; 4];
    let read = rb.read(&mut output);
    assert_eq!(read, 4);
    assert_eq!(output, [3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_ring_buffer_clear() {
    let rb = RingBuffer::<f32>::new(16);
    rb.write(&[1.0, 2.0, 3.0]);
    assert_eq!(rb.available_read(), 3);

    rb.clear();
    assert_eq!(rb.available_read(), 0);
    assert!(rb.is_empty());
}

#[test]
fn test_ring_buffer_available_write() {
    let rb = RingBuffer::<f32>::new(8);
    assert_eq!(rb.available_write(), 8);

    rb.write(&[1.0, 2.0, 3.0]);
    assert_eq!(rb.available_write(), 5);

    rb.write(&[4.0, 5.0]);
    assert_eq!(rb.available_write(), 3);
}

#[test]
fn test_ring_buffer_960_frames() {
    let rb = RingBuffer::<f32>::new(960 * 4);
    let data = vec![0.5f32; 960];
    let written = rb.write(&data);
    assert_eq!(written, 960);

    let mut output = vec![0.0f32; 960];
    let read = rb.read(&mut output);
    assert_eq!(read, 960);

    for (out, &inp) in output.iter().zip(data.iter()) {
        assert!((out - inp).abs() < 0.001);
    }
}

#[test]
fn test_ring_buffer_i16() {
    let rb = RingBuffer::<i16>::new(16);
    let data = [100i16, 200, 300];
    rb.write(&data);

    let mut output = [0i16; 3];
    rb.read(&mut output);
    assert_eq!(output, data);
}

// ============================================================================
// RingBuffer + AudioBuffer integration tests
// ============================================================================

#[test]
fn test_ring_buffer_with_audio_buffer() {
    let rb = RingBuffer::<f32>::new(960 * 4);
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };

    // Write 960 samples
    let samples = vec![0.5f32; 960];
    rb.write(&samples);

    // Read into AudioBuffer
    let mut output = vec![0.0f32; 960];
    rb.read(&mut output);
    let buffer = AudioBuffer::new(output, format).unwrap();

    assert_eq!(buffer.sample_count(), 960);
    assert_eq!(buffer.frame_count(), 960);
    for sample in buffer.samples() {
        assert!((sample - 0.5).abs() < 0.001);
    }
}

#[test]
fn test_multi_frame_pipeline() {
    let rb = RingBuffer::<f32>::new(1920 * 4);
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };

    for frame in 0..10 {
        let samples: Vec<f32> = (0..1920)
            .map(|i| {
                let t = (frame * 1920 + i) as f32 / 48000.0;
                (t * 2.0 * std::f32::consts::PI * 440.0).sin() * 0.5
            })
            .collect();

        rb.write(&samples);

        let mut output = vec![0.0f32; 1920];
        rb.read(&mut output);
        let buffer = AudioBuffer::new(output, format).unwrap();

        assert_eq!(buffer.sample_count(), 1920);
        assert_eq!(buffer.frame_count(), 960);
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_audio_buffer_44100_mono() {
    let format = AudioFormat {
        sample_rate: 44100,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let samples = vec![0.1f32; 441]; // 10ms @ 44.1kHz
    let buffer = AudioBuffer::new(samples, format).unwrap();
    assert_eq!(buffer.sample_count(), 441);
    assert_eq!(buffer.frame_count(), 441);
}

#[test]
fn test_audio_buffer_96000_stereo() {
    let format = AudioFormat {
        sample_rate: 96000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };
    let samples = vec![0.1f32; 3840]; // 96000/1000 * 2 = 192 frames * 2 channels
    let buffer = AudioBuffer::new(samples, format).unwrap();
    assert_eq!(buffer.sample_count(), 3840);
    assert_eq!(buffer.frame_count(), 1920);
}

#[test]
fn test_ring_buffer_large_capacity() {
    let rb = RingBuffer::<f32>::new(65536);
    assert_eq!(rb.capacity(), 65536);
    assert!(rb.is_empty());
}

#[test]
fn test_ring_buffer_minimum_capacity() {
    let rb = RingBuffer::<f32>::new(1);
    assert_eq!(rb.capacity(), 1); // next_power_of_two(1) = 1
}

#[test]
fn test_audio_buffer_negative_samples() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let samples = vec![-0.5f32, -1.0, 0.0, 0.5, 1.0];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.samples(), &samples[..]);
}

#[test]
fn test_audio_buffer_i16_min_max() {
    let format = AudioFormat {
        sample_rate: 48000,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
    let samples = vec![i16::MIN, i16::MAX, 0];
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
    assert_eq!(buffer.samples(), &samples[..]);
}
