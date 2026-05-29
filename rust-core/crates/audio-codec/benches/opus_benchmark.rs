use audio_codec::{
    OpusCodec, OpusConfig, OpusEncoderCodec, OpusDecoderCodec, SampleRate, Channels, Bitrate, FrameSize,
};
use audio_core::AudioBuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;

fn create_random_samples(count: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..count).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn create_sine_wave_samples(count: usize, frequency: f32, sample_rate: u32) -> Vec<f32> {
    (0..count)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin()
        })
        .collect()
}

fn benchmark_opus_encode(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let samples = create_sine_wave_samples(960, 440.0, 48000);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples, format).unwrap();

    let mut codec = OpusCodec::new(config).unwrap();

    c.bench_function("opus_encode_960_samples_mono", |b| {
        b.iter(|| {
            let encoded = codec.encode(black_box(&buffer)).unwrap();
            black_box(encoded);
        })
    });
}

fn benchmark_opus_decode(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let samples = create_sine_wave_samples(960, 440.0, 48000);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();

    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let encoded = encoder.encode(&buffer).unwrap();

    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    c.bench_function("opus_decode_960_samples_mono", |b| {
        b.iter(|| {
            let decoded = decoder.decode(black_box(&encoded)).unwrap();
            black_box(decoded);
        })
    });
}

fn benchmark_opus_encode_stereo(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Stereo,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let samples = create_sine_wave_samples(1920, 440.0, 48000);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples, format).unwrap();

    let mut codec = OpusCodec::new(config).unwrap();

    c.bench_function("opus_encode_1920_samples_stereo", |b| {
        b.iter(|| {
            let encoded = codec.encode(black_box(&buffer)).unwrap();
            black_box(encoded);
        })
    });
}

fn benchmark_opus_decode_stereo(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Stereo,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let samples = create_sine_wave_samples(1920, 440.0, 48000);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples.clone(), format).unwrap();

    let mut encoder = OpusEncoderCodec::new(config).unwrap();
    let encoded = encoder.encode(&buffer).unwrap();

    let mut decoder = OpusDecoderCodec::new(config).unwrap();

    c.bench_function("opus_decode_1920_samples_stereo", |b| {
        b.iter(|| {
            let decoded = decoder.decode(black_box(&encoded)).unwrap();
            black_box(decoded);
        })
    });
}

fn benchmark_opus_different_bitrate(c: &mut Criterion) {
    let sample_rate = SampleRate::Hz48000;
    let channels = Channels::Mono;
    let frame_size = FrameSize::Ms20;
    let samples = create_sine_wave_samples(960, 440.0, 48000);

    let mut group = c.benchmark_group("opus_encode_different_bitrates");

    for bitrate in [Bitrate::Kbps64, Bitrate::Kbps128, Bitrate::Kbps256] {
        let config = OpusConfig::new(sample_rate, channels, bitrate, frame_size);
        let format = config.to_audio_format();
        let buffer = AudioBuffer::new(samples.clone(), format).unwrap();
        let mut codec = OpusCodec::new(config).unwrap();

        group.bench_function(format!("{:?}", bitrate), |b| {
            b.iter(|| {
                let encoded = codec.encode(black_box(&buffer)).unwrap();
                black_box(encoded);
            })
        });
    }

    group.finish();
}

fn benchmark_opus_different_frame_sizes(c: &mut Criterion) {
    let sample_rate = SampleRate::Hz48000;
    let channels = Channels::Mono;
    let bitrate = Bitrate::Kbps128;

    let mut group = c.benchmark_group("opus_encode_different_frame_sizes");

    for (frame_size, sample_count) in [
        (FrameSize::Ms10, 480),
        (FrameSize::Ms20, 960),
        (FrameSize::Ms40, 1920),
    ] {
        let samples = create_sine_wave_samples(sample_count, 440.0, 48000);
        let config = OpusConfig::new(sample_rate, channels, bitrate, frame_size);
        let format = config.to_audio_format();
        let buffer = AudioBuffer::new(samples, format).unwrap();
        let mut codec = OpusCodec::new(config).unwrap();

        group.bench_function(format!("{:?}", frame_size), |b| {
            b.iter(|| {
                let encoded = codec.encode(black_box(&buffer)).unwrap();
                black_box(encoded);
            })
        });
    }

    group.finish();
}

fn benchmark_opus_roundtrip(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Mono,
        Bitrate::Kbps128,
        FrameSize::Ms20,
    );
    let samples = create_random_samples(960);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples, format).unwrap();

    let mut codec = OpusCodec::new(config).unwrap();

    c.bench_function("opus_roundtrip_960_samples", |b| {
        b.iter(|| {
            let encoded = codec.encode(black_box(&buffer)).unwrap();
            let decoded = codec.decode(black_box(&encoded)).unwrap();
            black_box(decoded);
        })
    });
}

fn benchmark_opus_compression_ratio(c: &mut Criterion) {
    let config = OpusConfig::new(
        SampleRate::Hz48000,
        Channels::Mono,
        Bitrate::Kbps64,
        FrameSize::Ms20,
    );
    let samples = create_random_samples(960);
    let format = config.to_audio_format();
    let buffer = AudioBuffer::new(samples, format).unwrap();

    let mut codec = OpusCodec::new(config).unwrap();

    c.bench_function("opus_compression_ratio", |b| {
        b.iter(|| {
            let encoded = codec.encode(black_box(&buffer)).unwrap();
            black_box(encoded.len());
        })
    });
}

criterion_group!(
    benches,
    benchmark_opus_encode,
    benchmark_opus_decode,
    benchmark_opus_encode_stereo,
    benchmark_opus_decode_stereo,
    benchmark_opus_different_bitrate,
    benchmark_opus_different_frame_sizes,
    benchmark_opus_roundtrip,
    benchmark_opus_compression_ratio,
);
criterion_main!(benches);
