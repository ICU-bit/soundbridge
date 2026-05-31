use network::bandwidth_pid::*;
use std::thread;
use std::time::Duration;

#[test]
fn test_pid_default_config() {
    let controller = PidBandwidthController::with_default_config();
    assert_eq!(controller.current_bitrate(), (32_000 + 9_216_000) / 2);
}

#[test]
fn test_pid_high_loss_decreases_bitrate() {
    let mut controller = PidBandwidthController::with_default_config();
    let initial = controller.current_bitrate();

    // 等待超过 MIN_DT (10ms)
    thread::sleep(Duration::from_millis(20));

    let metrics = NetworkMetrics {
        loss_rate: 0.10,
        latency_ms: 100.0,
        jitter_ms: 30.0,
    };

    let new_bitrate = controller.update(metrics);
    assert!(new_bitrate < initial);
}

#[test]
fn test_pid_low_loss_increases_bitrate() {
    let mut controller = PidBandwidthController::with_default_config();
    controller.set_bitrate(100_000);
    let initial = controller.current_bitrate();

    // 等待超过 MIN_DT (10ms)
    thread::sleep(Duration::from_millis(20));

    let metrics = NetworkMetrics {
        loss_rate: 0.001,
        latency_ms: 20.0,
        jitter_ms: 5.0,
    };

    let new_bitrate = controller.update(metrics);
    assert!(new_bitrate > initial);
}

#[test]
fn test_pid_respects_bounds() {
    let mut controller = PidBandwidthController::with_default_config();

    controller.set_bitrate(10_000);
    assert_eq!(controller.current_bitrate(), 32_000);

    controller.set_bitrate(10_000_000);
    assert_eq!(controller.current_bitrate(), 9_216_000);
}
