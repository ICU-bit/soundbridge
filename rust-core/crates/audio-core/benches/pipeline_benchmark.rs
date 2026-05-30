//! SoundBridge 音频管线性能基准测试

use audio_core::{AudioBuffer, AudioFormat, RingBuffer, SampleFormat};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_ring_buffer_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer");

    for size in [480, 960, 1920] {
        group.bench_with_input(BenchmarkId::new("write", size), &size, |b, &size| {
            let rb = RingBuffer::<f32>::new(size * 4);
            let data = vec![0.5f32; size];
            b.iter(|| {
                rb.write(black_box(&data));
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer");

    for size in [480, 960, 1920] {
        group.bench_with_input(BenchmarkId::new("read", size), &size, |b, &size| {
            let rb = RingBuffer::<f32>::new(size * 4);
            let data = vec![0.5f32; size];
            rb.write(&data);
            let mut output = vec![0.0f32; size];
            b.iter(|| {
                rb.read(black_box(&mut output));
            });
        });
    }

    group.finish();
}

fn bench_audio_buffer_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_buffer");

    for size in [480, 960, 1920] {
        group.bench_with_input(BenchmarkId::new("create", size), &size, |b, &size| {
            let format = AudioFormat {
                sample_rate: 48000,
                channels: 1,
                sample_format: SampleFormat::F32,
            };
            let samples = vec![0.5f32; size];
            b.iter(|| {
                AudioBuffer::new(black_box(samples.clone()), format).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_audio_buffer_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_buffer");

    for size in [480, 960, 1920] {
        group.bench_with_input(BenchmarkId::new("clone", size), &size, |b, &size| {
            let format = AudioFormat {
                sample_rate: 48000,
                channels: 1,
                sample_format: SampleFormat::F32,
            };
            let samples = vec![0.5f32; size];
            let buffer = AudioBuffer::new(samples, format).unwrap();
            b.iter(|| {
                let _ = black_box(&buffer).clone();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_ring_buffer_write,
    bench_ring_buffer_read,
    bench_audio_buffer_creation,
    bench_audio_buffer_clone
);
criterion_main!(benches);
