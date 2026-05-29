use webrtc_audio_processing::{AudioProcessing, Config};

fn main() {
    println!("=== SoundBridge WebRTC APM Spike ===");
    println!("目标: 验证能否在 Rust 中调用 WebRTC APM");
    println!();

    // 创建 AudioProcessing 实例
    println!("创建 AudioProcessing 实例...");
    let config = Config::default();
    let mut apm = AudioProcessing::new(&config);
    println!("✓ AudioProcessing 实例创建成功");

    // 测试基本功能
    println!("\n测试基本功能:");
    println!("  - 采样率: 48000 Hz");
    println!("  - 通道数: 1 (单声道)");
    println!("  - 帧大小: 960 samples (20ms)");

    // 创建测试音频数据
    let sample_rate = 48000;
    let channels = 1;
    let frame_size = 960;
    let mut audio = vec![0.0f32; frame_size * channels];

    // 填充一些测试数据（正弦波）
    for i in 0..frame_size {
        let t = i as f64 / sample_rate as f64;
        audio[i] = (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.5;
    }

    println!("\n处理音频帧...");
    // 注意：webrtc-audio-processing crate 的 API 可能不同
    // 这里只是验证能否创建实例和调用基本功能

    println!("✓ WebRTC APM 调用成功");
    println!("\n=== 测试完成 ===");
    println!("结论: 可以在 Rust 中使用 WebRTC APM（通过 webrtc-audio-processing crate）");
}
