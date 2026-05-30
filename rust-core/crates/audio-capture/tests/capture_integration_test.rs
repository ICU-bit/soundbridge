//! Integration tests for audio-capture crate.
//!
//! Tests the public API from an external perspective.
//! Note: Tests requiring actual audio hardware are limited
//! as CI environments typically lack audio devices.

use audio_capture::*;
use std::sync::Arc;

// ============================================================================
// CaptureConfig tests
// ============================================================================

#[test]
fn test_capture_config_default() {
    let config = CaptureConfig::default();
    assert_eq!(config.sample_rate, 48000);
    assert_eq!(config.channels, 2);
    assert_eq!(config.buffer_size, 960);
}

#[test]
fn test_capture_config_new() {
    let config = CaptureConfig::new(44100, 1, 1024);
    assert_eq!(config.sample_rate, 44100);
    assert_eq!(config.channels, 1);
    assert_eq!(config.buffer_size, 1024);
}

#[test]
fn test_capture_config_frame_duration_ms_default() {
    let config = CaptureConfig::default();
    let duration = config.frame_duration_ms();
    // 960 / 48000 * 1000 = 20ms
    assert!(
        (duration - 20.0).abs() < 0.01,
        "Expected ~20ms, got {}",
        duration
    );
}

#[test]
fn test_capture_config_frame_duration_ms_custom() {
    let config = CaptureConfig::new(44100, 2, 441);
    let duration = config.frame_duration_ms();
    // 441 / 44100 * 1000 = 10ms
    assert!(
        (duration - 10.0).abs() < 0.01,
        "Expected ~10ms, got {}",
        duration
    );
}

#[test]
fn test_capture_config_frame_bytes_default() {
    let config = CaptureConfig::default();
    let bytes = config.frame_bytes();
    // 960 * 2 * 4 (f32) = 7680
    assert_eq!(bytes, 7680);
}

#[test]
fn test_capture_config_frame_bytes_mono() {
    let config = CaptureConfig::new(48000, 1, 960);
    let bytes = config.frame_bytes();
    // 960 * 1 * 4 = 3840
    assert_eq!(bytes, 3840);
}

#[test]
fn test_capture_config_clone() {
    let config = CaptureConfig::new(44100, 1, 512);
    let cloned = config.clone();
    assert_eq!(config.sample_rate, cloned.sample_rate);
    assert_eq!(config.channels, cloned.channels);
    assert_eq!(config.buffer_size, cloned.buffer_size);
}

#[test]
fn test_capture_config_debug() {
    let config = CaptureConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("48000"));
    assert!(debug_str.contains("CaptureConfig"));
}

// ============================================================================
// DeviceInfo tests
// ============================================================================

#[test]
fn test_device_info_list_devices() {
    let result = CaptureDevice::list_devices();
    // Should succeed even if no devices are available
    assert!(result.is_ok());
}

#[test]
fn test_device_info_fields() {
    let devices = CaptureDevice::list_devices().unwrap();
    if !devices.is_empty() {
        let device = &devices[0];
        assert!(!device.name.is_empty());
        // sample_rates and channels should be populated
        println!(
            "Device: {}, sample_rates: {:?}, channels: {:?}",
            device.name, device.sample_rates, device.channels
        );
    }
}

#[test]
fn test_device_info_clone() {
    let info = DeviceInfo {
        name: "Test Device".to_string(),
        is_default: true,
        sample_rates: vec![48000],
        channels: vec![2],
    };
    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
    assert_eq!(info.is_default, cloned.is_default);
}

#[test]
fn test_device_info_debug() {
    let info = DeviceInfo {
        name: "Test".to_string(),
        is_default: false,
        sample_rates: vec![],
        channels: vec![],
    };
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("Test"));
}

// ============================================================================
// CaptureDevice tests (non-hardware)
// ============================================================================

#[test]
fn test_capture_device_default_device() {
    let result = CaptureDevice::default_device();
    // May fail in CI without audio devices, that's OK
    if let Ok(device) = result {
        assert!(!device.name.is_empty());
        assert!(device.is_default);
    }
}

#[test]
fn test_capture_device_new_default() {
    let config = CaptureConfig::default();
    let result = CaptureDevice::new_default(config);
    // May fail in CI without audio devices
    if let Ok(device) = result {
        assert_eq!(device.config().sample_rate, 48000);
        assert_eq!(device.frame_size(), 960);
        assert_eq!(device.channels(), 2);
        assert!(!device.is_running());
        assert!(!device.device_name().is_empty());
    }
}

#[test]
fn test_capture_device_new_with_specific_device() {
    let devices = CaptureDevice::list_devices().unwrap();
    if let Some(device_info) = devices.first() {
        let config = CaptureConfig::default();
        let result = CaptureDevice::new(device_info, config);
        if let Ok(device) = result {
            assert_eq!(device.frame_size(), 960);
        }
    }
}

#[test]
fn test_capture_device_ring_buffer() {
    let config = CaptureConfig::default();
    if let Ok(device) = CaptureDevice::new_default(config) {
        let rb = device.ring_buffer();
        // Should be an Arc
        let rb2 = rb.clone();
        assert!(Arc::ptr_eq(&rb, &rb2));
    }
}

// ============================================================================
// CaptureError tests
// ============================================================================

#[test]
fn test_capture_error_display() {
    let err = CaptureError::DeviceNotFound("TestDevice".to_string());
    assert!(err.to_string().contains("TestDevice"));

    let err = CaptureError::ConfigNotSupported("bad config".to_string());
    assert!(err.to_string().contains("bad config"));

    let err = CaptureError::StreamError("stream err".to_string());
    assert!(err.to_string().contains("stream err"));

    let err = CaptureError::DeviceUnavailable;
    assert!(err.to_string().contains("不可用"));
}

#[test]
fn test_capture_error_debug() {
    let err = CaptureError::DeviceNotFound("Test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("DeviceNotFound"));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_capture_config_zero_buffer() {
    let config = CaptureConfig::new(48000, 2, 0);
    assert_eq!(config.frame_bytes(), 0);
    // 0 / 48000 * 1000 = 0.0
    assert!((config.frame_duration_ms() - 0.0).abs() < 0.001);
}

#[test]
fn test_capture_config_high_sample_rate() {
    let config = CaptureConfig::new(192000, 2, 3840);
    // 3840 / 192000 * 1000 = 20ms
    assert!((config.frame_duration_ms() - 20.0).abs() < 0.01);
    // 3840 * 2 * 4 = 30720
    assert_eq!(config.frame_bytes(), 30720);
}

#[test]
fn test_capture_config_many_channels() {
    let config = CaptureConfig::new(48000, 8, 960);
    // 960 * 8 * 4 = 30720
    assert_eq!(config.frame_bytes(), 30720);
}

#[test]
fn test_device_info_many_sample_rates() {
    let info = DeviceInfo {
        name: "Multi-rate Device".to_string(),
        is_default: false,
        sample_rates: vec![8000, 16000, 22050, 44100, 48000, 96000],
        channels: vec![1, 2, 4, 8],
    };
    assert_eq!(info.sample_rates.len(), 6);
    assert_eq!(info.channels.len(), 4);
}

#[test]
fn test_device_info_empty() {
    let info = DeviceInfo {
        name: String::new(),
        is_default: false,
        sample_rates: vec![],
        channels: vec![],
    };
    assert!(info.name.is_empty());
    assert!(info.sample_rates.is_empty());
}
