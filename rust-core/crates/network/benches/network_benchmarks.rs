//! SoundBridge 网络模块基准测试
//!
//! 测试内容：
//! - SRTP 加密/解密性能（不同包大小）
//! - 会话握手延迟
//! - QUIC 消息序列化/反序列化
//! - Jitter Buffer 推入/弹出性能
//! - 网络监控统计计算

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use network::{
    generate_session_id, AudioConfig, Capability, ControlMessage, CryptoKeys, EcdhPublicKey,
    HandshakeMessage, JitterBuffer, JitterBufferConfig, NetMonitor, NetworkStatsData,
    RawJitterBuffer, Session, SessionConfig, SessionState, SrtpContext,
};
use rand::Rng;

// ────────────────────────── 辅助函数 ──────────────────────────

/// 构造 RTP 数据包（12 字节头 + payload）
fn make_rtp_packet(ssrc: u32, seq: u16, payload: &[u8]) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(12 + payload.len());
    pkt.push(0x80); // V=2, P=0, X=0, CC=0
    pkt.push(0x60); // M=0, PT=96
    pkt.extend_from_slice(&seq.to_be_bytes());
    pkt.extend_from_slice(&0u32.to_be_bytes()); // timestamp
    pkt.extend_from_slice(&ssrc.to_be_bytes());
    pkt.extend_from_slice(payload);
    pkt
}

/// 生成随机字节负载
fn random_payload(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen()).collect()
}

/// 生成随机 f32 PCM 帧
fn random_pcm_frame(size: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

// ────────────────────────── SRTP 加密/解密 ──────────────────────────

fn bench_srtp_protect(c: &mut Criterion) {
    let mut group = c.benchmark_group("srtp_protect");

    for &size in &[64usize, 960, 4096, 16384] {
        let payload = random_payload(size);
        let rtp = make_rtp_packet(0x12345678, 1, &payload);

        group.bench_with_input(BenchmarkId::from_parameter(size), &rtp, |b, rtp| {
            let keys = CryptoKeys::generate();
            let mut ctx = SrtpContext::new(keys, 0x12345678).unwrap();
            b.iter(|| {
                let result = ctx.protect(black_box(rtp)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_srtp_unprotect(c: &mut Criterion) {
    let mut group = c.benchmark_group("srtp_unprotect");

    for &size in &[64usize, 960, 4096, 16384] {
        let keys = CryptoKeys::generate();
        let payload = random_payload(size);
        let rtp = make_rtp_packet(0x12345678, 1, &payload);
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), 0x12345678).unwrap();
        let encrypted = encrypt_ctx.protect(&rtp).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), &encrypted, |b, enc| {
            let mut ctx = SrtpContext::new(keys.clone(), 0x12345678).unwrap();
            b.iter(|| {
                let result = ctx.unprotect(black_box(enc)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_srtp_protect_unprotect_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("srtp_roundtrip");

    for &size in &[64usize, 960, 4096] {
        let payload = random_payload(size);
        let rtp = make_rtp_packet(0xAABBCCDD, 1, &payload);

        group.bench_with_input(BenchmarkId::from_parameter(size), &rtp, |b, rtp| {
            let keys = CryptoKeys::generate();
            let mut enc = SrtpContext::new(keys.clone(), 0xAABBCCDD).unwrap();
            let mut dec = SrtpContext::new(keys, 0xAABBCCDD).unwrap();
            b.iter(|| {
                let encrypted = enc.protect(black_box(rtp)).unwrap();
                let decrypted = dec.unprotect(black_box(&encrypted)).unwrap();
                black_box(decrypted);
            });
        });
    }

    group.finish();
}

// ────────────────────────── 会话握手 ──────────────────────────

fn bench_session_full_handshake(c: &mut Criterion) {
    c.bench_function("session_full_handshake", |b| {
        b.iter(|| {
            let session_id = generate_session_id();
            let config = SessionConfig::default();

            let mut client =
                Session::new_client(session_id.clone(), Capability::default(), config.clone());
            let mut server = Session::new_server(String::new(), Capability::default(), config);

            // 1. Client → ClientHello
            let client_hello = client.initiate_handshake().unwrap();
            // 2. Server → ServerHello
            let server_hello = server.handle_client_hello(&client_hello).unwrap();
            // 3. Client → KeyExchange
            let key_exchange = client.handle_server_hello(&server_hello).unwrap();
            // 4. Server → Finished
            let finished = server.handle_key_exchange(&key_exchange).unwrap();
            // 5. Client processes Finished
            client.handle_finished_client(&finished).unwrap();

            assert_eq!(client.state(), SessionState::Established);
            assert_eq!(server.state(), SessionState::Established);
            black_box((&client, &server));
        });
    });
}

fn bench_session_message_serialize(c: &mut Criterion) {
    let msg = HandshakeMessage::ClientHello {
        session_id: "bench-session-001".into(),
        capabilities: Capability::default(),
        client_public_key: EcdhPublicKey::generate(),
    };

    c.bench_function("session_serialize_client_hello", |b| {
        b.iter(|| {
            let bytes = Session::serialize_message(black_box(&msg)).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_session_message_deserialize(c: &mut Criterion) {
    let msg = HandshakeMessage::ClientHello {
        session_id: "bench-session-001".into(),
        capabilities: Capability::default(),
        client_public_key: EcdhPublicKey::generate(),
    };
    let bytes = Session::serialize_message(&msg).unwrap();

    c.bench_function("session_deserialize_client_hello", |b| {
        b.iter(|| {
            let decoded: HandshakeMessage =
                Session::deserialize_message(black_box(&bytes)).unwrap();
            black_box(decoded);
        });
    });
}

fn bench_session_heartbeat(c: &mut Criterion) {
    let session_id = generate_session_id();
    let config = SessionConfig::default();

    let mut client = Session::new_client(session_id.clone(), Capability::default(), config.clone());
    let mut server = Session::new_server(String::new(), Capability::default(), config);

    // Complete handshake first
    let client_hello = client.initiate_handshake().unwrap();
    let server_hello = server.handle_client_hello(&client_hello).unwrap();
    let key_exchange = client.handle_server_hello(&server_hello).unwrap();
    let finished = server.handle_key_exchange(&key_exchange).unwrap();
    client.handle_finished_client(&finished).unwrap();

    c.bench_function("session_heartbeat_roundtrip", |b| {
        b.iter(|| {
            let heartbeat = client.create_heartbeat().unwrap();
            let ack = server.handle_heartbeat(black_box(&heartbeat)).unwrap();
            client.handle_heartbeat_ack(black_box(&ack)).unwrap();
        });
    });
}

// ────────────────────────── QUIC 控制消息 ──────────────────────────

fn bench_control_message_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_message_serialize");

    let messages: Vec<(&str, ControlMessage)> = vec![
        (
            "session_create",
            ControlMessage::SessionCreate {
                session_id: "s1".into(),
                device_name: "PC".into(),
            },
        ),
        (
            "audio_config",
            ControlMessage::AudioConfigRequest {
                config: AudioConfig::default(),
            },
        ),
        (
            "network_stats",
            ControlMessage::NetworkStatsReport {
                stats: NetworkStatsData {
                    rtt_ms: 12.5,
                    loss_rate: 0.02,
                    bandwidth_bps: 128_000,
                    jitter_ms: 3.0,
                },
            },
        ),
        ("device_query", ControlMessage::DeviceQuery),
    ];

    for (name, msg) in &messages {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let data = bincode::serialize(black_box(msg)).unwrap();
                black_box(data);
            });
        });
    }

    group.finish();
}

fn bench_control_message_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_message_deserialize");

    let messages: Vec<(&str, ControlMessage)> = vec![
        (
            "session_create",
            ControlMessage::SessionCreate {
                session_id: "s1".into(),
                device_name: "PC".into(),
            },
        ),
        (
            "audio_config",
            ControlMessage::AudioConfigRequest {
                config: AudioConfig::default(),
            },
        ),
        (
            "network_stats",
            ControlMessage::NetworkStatsReport {
                stats: NetworkStatsData {
                    rtt_ms: 12.5,
                    loss_rate: 0.02,
                    bandwidth_bps: 128_000,
                    jitter_ms: 3.0,
                },
            },
        ),
        ("device_query", ControlMessage::DeviceQuery),
    ];

    for (name, msg) in &messages {
        let data = bincode::serialize(msg).unwrap();
        group.bench_function(*name, |b| {
            b.iter(|| {
                let decoded: ControlMessage = bincode::deserialize(black_box(&data)).unwrap();
                black_box(decoded);
            });
        });
    }

    group.finish();
}

fn bench_control_message_roundtrip(c: &mut Criterion) {
    c.bench_function("control_message_roundtrip", |b| {
        let msg = ControlMessage::AudioConfigRequest {
            config: AudioConfig::default(),
        };
        b.iter(|| {
            let data = bincode::serialize(black_box(&msg)).unwrap();
            let decoded: ControlMessage = bincode::deserialize(black_box(&data)).unwrap();
            black_box(decoded);
        });
    });
}

// ────────────────────────── Jitter Buffer ──────────────────────────

fn bench_jitter_buffer_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("jitter_buffer_push");

    for &frame_size in &[960usize, 1920] {
        group.bench_with_input(
            BenchmarkId::new("pcm", frame_size),
            &frame_size,
            |b, &size| {
                let config = JitterBufferConfig::default();
                let mut jb = JitterBuffer::new(config);
                let data = random_pcm_frame(size);
                let mut seq = 0u32;
                b.iter(|| {
                    jb.push(seq, black_box(data.clone()));
                    seq += 1;
                });
            },
        );
    }

    group.finish();
}

fn bench_jitter_buffer_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("jitter_buffer_pop");

    for &count in &[100usize, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    // Setup: fill buffer
                    let config = JitterBufferConfig {
                        max_packets: count + 10,
                        ..Default::default()
                    };
                    let mut jb = JitterBuffer::new(config);
                    for i in 0..count as u32 {
                        jb.push(i, random_pcm_frame(960));
                    }
                    jb
                },
                |mut jb| {
                    // Benchmark: pop all
                    while jb.pop().is_some() {
                        // drain
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_jitter_buffer_push_pop_interleaved(c: &mut Criterion) {
    c.bench_function("jitter_buffer_push_pop_interleaved", |b| {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);
        let data = random_pcm_frame(960);
        let mut seq: u32;

        // Pre-fill with a few packets
        for i in 0..5u32 {
            jb.push(i, data.clone());
        }
        seq = 5;

        b.iter(|| {
            jb.push(seq, black_box(data.clone()));
            seq += 1;
            if let Some(pkt) = jb.pop() {
                black_box(pkt);
            }
        });
    });
}

fn bench_raw_jitter_buffer_push(c: &mut Criterion) {
    c.bench_function("raw_jitter_buffer_push", |b| {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);
        let data = random_payload(120); // typical Opus frame
        let mut seq = 0u32;
        b.iter(|| {
            jb.push(seq, seq * 20000, black_box(data.clone()));
            seq += 1;
        });
    });
}

fn bench_raw_jitter_buffer_pop(c: &mut Criterion) {
    c.bench_function("raw_jitter_buffer_pop", |b| {
        b.iter_batched(
            || {
                let config = JitterBufferConfig {
                    max_packets: 1100,
                    ..Default::default()
                };
                let mut jb = RawJitterBuffer::new(config);
                for i in 0..1000u32 {
                    jb.push(i, i * 20000, random_payload(120));
                }
                jb
            },
            |mut jb| {
                while jb.pop().is_some() {}
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ────────────────────────── 网络监控 ──────────────────────────

fn bench_net_monitor_report_rtt(c: &mut Criterion) {
    c.bench_function("net_monitor_report_rtt", |b| {
        let mut monitor = NetMonitor::with_default_config();
        let mut rng = rand::thread_rng();
        b.iter(|| {
            let rtt = rng.gen_range(5.0..200.0);
            monitor.report_rtt(black_box(rtt));
        });
    });
}

fn bench_net_monitor_report_packet(c: &mut Criterion) {
    c.bench_function("net_monitor_report_packet_received", |b| {
        let mut monitor = NetMonitor::with_default_config();
        let mut seq = 0u32;
        b.iter(|| {
            monitor.report_packet_received(black_box(seq));
            seq += 1;
        });
    });
}

fn bench_net_monitor_stats(c: &mut Criterion) {
    c.bench_function("net_monitor_stats_snapshot", |b| {
        let mut monitor = NetMonitor::with_default_config();
        // Pre-populate with data
        for i in 0..100 {
            monitor.report_rtt(10.0 + (i as f32 * 0.5));
            monitor.report_packet_received(i);
            monitor.report_bytes_sent(1200);
        }
        b.iter(|| {
            let stats = monitor.stats();
            black_box(stats);
        });
    });
}

fn bench_net_monitor_quality_score(c: &mut Criterion) {
    c.bench_function("net_monitor_quality_score", |b| {
        let mut monitor = NetMonitor::with_default_config();
        for i in 0..50 {
            monitor.report_rtt(15.0 + (i as f32 * 0.3));
            monitor.report_packet_received(i);
        }
        b.iter(|| {
            let stats = monitor.stats();
            black_box(stats.quality_score);
        });
    });
}

fn bench_net_monitor_bitrate_recommendation(c: &mut Criterion) {
    c.bench_function("net_monitor_bitrate_recommendation", |b| {
        let mut monitor = NetMonitor::with_default_config();
        for i in 0..50 {
            monitor.report_rtt(30.0);
            monitor.report_packet_received(i);
        }
        b.iter(|| {
            let rec = monitor.bitrate_recommendation();
            black_box(rec);
        });
    });
}

// ────────────────────────── HKDF 密钥派生 ──────────────────────────

fn bench_hkdf_key_derivation(c: &mut Criterion) {
    use network::crypto::derive_session_keys;

    c.bench_function("hkdf_derive_session_keys", |b| {
        let master_secret = [0x42u8; 32];
        let salt = [0x69u8; 16];
        b.iter(|| {
            let keys = derive_session_keys(black_box(&master_secret), black_box(&salt)).unwrap();
            black_box(keys);
        });
    });
}

fn bench_crypto_keys_generate(c: &mut Criterion) {
    c.bench_function("crypto_keys_generate", |b| {
        b.iter(|| {
            let keys = CryptoKeys::generate();
            black_box(keys);
        });
    });
}

fn bench_srtp_context_create(c: &mut Criterion) {
    c.bench_function("srtp_context_create", |b| {
        b.iter_batched(
            || CryptoKeys::generate(),
            |keys| {
                let ctx = SrtpContext::new(keys, 0x12345678).unwrap();
                black_box(ctx);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ────────────────────────── Criterion 入口 ──────────────────────────

criterion_group!(
    benches,
    // SRTP 加密/解密
    bench_srtp_protect,
    bench_srtp_unprotect,
    bench_srtp_protect_unprotect_roundtrip,
    // 会话握手
    bench_session_full_handshake,
    bench_session_message_serialize,
    bench_session_message_deserialize,
    bench_session_heartbeat,
    // QUIC 控制消息
    bench_control_message_serialize,
    bench_control_message_deserialize,
    bench_control_message_roundtrip,
    // Jitter Buffer
    bench_jitter_buffer_push,
    bench_jitter_buffer_pop,
    bench_jitter_buffer_push_pop_interleaved,
    bench_raw_jitter_buffer_push,
    bench_raw_jitter_buffer_pop,
    // 网络监控
    bench_net_monitor_report_rtt,
    bench_net_monitor_report_packet,
    bench_net_monitor_stats,
    bench_net_monitor_quality_score,
    bench_net_monitor_bitrate_recommendation,
    // 密钥派生
    bench_hkdf_key_derivation,
    bench_crypto_keys_generate,
    bench_srtp_context_create,
);
criterion_main!(benches);
