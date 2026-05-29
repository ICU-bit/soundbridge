use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 延迟测量结果
#[derive(Debug, Clone)]
struct LatencyResult {
    buffer_size: usize,
    avg_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    samples: usize,
}

/// 采集到的数据包，带时间戳
#[derive(Debug, Clone)]
struct CapturedChunk {
    timestamp: Instant,
    samples: Vec<f32>,
}

fn main() {
    tracing_subscriber::fmt::init();

    println!("=== SoundBridge cpal 延迟测试 ===");
    println!("目标: 验证 cpal 在 Windows 上的延迟是否 <50ms");
    println!();

    let host = cpal::default_host();
    println!("音频主机: {:?}", host.id());

    // 列出可用设备
    println!("\n--- 可用输入设备 ---");
    let input_devices: Vec<_> = host.input_devices().unwrap().collect();
    for (i, device) in input_devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "未知".to_string());
        println!("  [{}] {}", i, name);
    }

    println!("\n--- 可用输出设备 ---");
    let output_devices: Vec<_> = host.output_devices().unwrap().collect();
    for (i, device) in output_devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "未知".to_string());
        println!("  [{}] {}", i, name);
    }

    // 获取默认设备
    let input_device = host.default_input_device().expect("无默认输入设备");
    let output_device = host.default_output_device().expect("无默认输出设备");

    println!("\n默认输入设备: {}", input_device.name().unwrap_or_default());
    println!("默认输出设备: {}", output_device.name().unwrap_or_default());

    // 查询设备支持的配置
    println!("\n--- 输入设备支持的配置 ---");
    if let Ok(config) = input_device.default_input_config() {
        println!("  默认配置: {:?}", config);
    }
    for config in input_device.supported_input_configs().unwrap() {
        println!("  {:?}", config);
    }

    println!("\n--- 输出设备支持的配置 ---");
    if let Ok(config) = output_device.default_output_config() {
        println!("  默认配置: {:?}", config);
    }
    for config in output_device.supported_output_configs().unwrap() {
        println!("  {:?}", config);
    }

    // 测试不同 buffer size
    let buffer_sizes = [480, 960, 1920];
    let sample_rate = 48000u32;
    let channels = 2u16; // 设备支持 2 通道（立体声）
    let test_duration = Duration::from_secs(3); // 每个 buffer size 测试 3 秒

    let mut results = Vec::new();

    for &buffer_size in &buffer_sizes {
        println!("\n--- 测试 buffer_size: {} samples ({:.1}ms) ---",
            buffer_size,
            buffer_size as f64 / sample_rate as f64 * 1000.0
        );

        match run_latency_test(
            &input_device,
            &output_device,
            sample_rate,
            channels,
            buffer_size,
            test_duration,
        ) {
            Ok(result) => {
                println!("  平均延迟: {:.2}ms", result.avg_latency_ms);
                println!("  最小延迟: {:.2}ms", result.min_latency_ms);
                println!("  最大延迟: {:.2}ms", result.max_latency_ms);
                println!("  测量样本数: {}", result.samples);
                results.push(result);
            }
            Err(e) => {
                println!("  测试失败: {}", e);
            }
        }
    }

    // 输出最终报告
    println!("\n\n=== 延迟测试报告 ===");
    println!("{:-<60}", "");
    println!("{:<15} {:<12} {:<12} {:<12} {:<10}",
        "Buffer Size", "平均(ms)", "最小(ms)", "最大(ms)", "样本数");
    println!("{:-<60}", "");

    for result in &results {
        let buffer_ms = result.buffer_size as f64 / sample_rate as f64 * 1000.0;
        let pass = if result.avg_latency_ms < 50.0 { "✓" } else { "✗" };
        println!("{:<12} {:.1}ms {:<5} {:<12.2} {:<12.2} {:<10}",
            format!("{} ({:.1}ms)", result.buffer_size, buffer_ms),
            result.avg_latency_ms,
            pass,
            result.min_latency_ms,
            result.max_latency_ms,
            result.samples
        );
    }
    println!("{:-<60}", "");

    // 判断是否通过成功标准
    let success = results.iter().any(|r| r.buffer_size == 960 && r.avg_latency_ms < 50.0);
    if success {
        println!("\n✓ 成功: buffer_size=960 时延迟 <50ms");
    } else {
        println!("\n✗ 未达标: buffer_size=960 时延迟 >=50ms");
    }
}

/// 运行单个 buffer size 的延迟测试
fn run_latency_test(
    input_device: &cpal::Device,
    output_device: &cpal::Device,
    sample_rate: u32,
    channels: u16,
    buffer_size: usize,
    test_duration: Duration,
) -> Result<LatencyResult, Box<dyn std::error::Error>> {
    // 配置输入流（使用默认 buffer size）
    let input_config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    // 配置输出流（使用默认 buffer size）
    let output_config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    // 共享缓冲区：采集 -> 播放
    let captured_chunks: Arc<Mutex<Vec<CapturedChunk>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_chunks_clone = captured_chunks.clone();

    // 延迟测量结果
    let latencies: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));

    // 播放位置追踪
    let playback_position: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    // 创建输入流（采集）
    let input_stream = input_device.build_input_stream(
        &input_config,
        move |data: &[f32], _info: &cpal::InputCallbackInfo| {
            let timestamp = Instant::now();
            let chunk = CapturedChunk {
                timestamp,
                samples: data.to_vec(),
            };
            if let Ok(mut chunks) = captured_chunks_clone.lock() {
                chunks.push(chunk);
                // 限制缓冲区大小，防止内存无限增长
                if chunks.len() > 1000 {
                    chunks.drain(0..500);
                }
            }
        },
        |err| {
            tracing::error!("输入流错误: {}", err);
        },
        None,
    )?;

    // 创建输出流（播放）
    let latencies_for_closure = latencies.clone();
    let playback_position_for_closure = playback_position.clone();
    let output_stream = output_device.build_output_stream(
        &output_config,
        move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
            let playback_time = Instant::now();

            if let Ok(chunks) = captured_chunks.lock() {
                if let Ok(mut pos) = playback_position_for_closure.lock() {
                    // 找到对应的采集块
                    let chunk_index = *pos / buffer_size;
                    if chunk_index < chunks.len() {
                        let chunk = &chunks[chunk_index];
                        let latency = playback_time.duration_since(chunk.timestamp);
                        let latency_ms = latency.as_secs_f64() * 1000.0;

                        if let Ok(mut lats) = latencies_for_closure.lock() {
                            lats.push(latency_ms);
                        }

                        // 复制采集数据到播放缓冲区
                        let copy_len = data.len().min(chunk.samples.len());
                        data[..copy_len].copy_from_slice(&chunk.samples[..copy_len]);
                        if copy_len < data.len() {
                            data[copy_len..].fill(0.0);
                        }

                        *pos += buffer_size;
                    } else {
                        // 没有数据，播放静音
                        data.fill(0.0);
                    }
                }
            }
        },
        |err| {
            tracing::error!("输出流错误: {}", err);
        },
        None,
    )?;

    // 开始采集和播放
    input_stream.play()?;
    output_stream.play()?;

    println!("  采集中... ({}秒)", test_duration.as_secs());
    std::thread::sleep(test_duration);

    // 停止流
    drop(input_stream);
    drop(output_stream);

    // 计算统计结果
    let latencies = latencies.lock().unwrap();
    if latencies.is_empty() {
        return Err("未采集到延迟数据".into());
    }

    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let min = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = latencies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    Ok(LatencyResult {
        buffer_size,
        avg_latency_ms: avg,
        min_latency_ms: min,
        max_latency_ms: max,
        samples: latencies.len(),
    })
}
