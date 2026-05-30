//! Integration tests for audio-playback crate.
//!
//! Tests the public API from an external perspective.
//! Note: Tests requiring actual audio hardware are limited
//! as CI environments typically lack audio devices.

use audio_playback::*;
use std::sync::Arc;

// ============================================================================
// PlaybackConfig tests
// ============================================================================

#[test]
fn test_playback_config_default() {
    let config = PlaybackConfig::default();
    assert_eq!(config.sample_rate, 48000);
    assert_eq!(config.channels, 2);
    assert_eq!(config.buffer_size, 960);
}

#[test]
fn test_playback_config_new() {
    let config = PlaybackConfig::new(44100, 1, 1024);
    assert_eq!(config.sample_rate, 44100);
    assert_eq!(config.channels, 1);
    assert_eq!(config.buffer_size, 1024);
}

#[test]
fn test_playback_config_frame_duration_ms_default() {
    let config = PlaybackConfig::default();
    let duration = config.frame_duration_ms();
    // 960 / 48000 * 1000 = 20ms
    assert!(
        (duration - 20.0).abs() < 0.01,
        "Expected ~20ms, got {}",
        duration
    );
}

#[test]
fn test_playback_config_frame_duration_ms_custom() {
    let config = PlaybackConfig::new(44100, 2, 441);
    let duration = config.frame_duration_ms();
    // 441 / 44100 * 1000 = 10ms
    assert!(
        (duration - 10.0).abs() < 0.01,
        "Expected ~10ms, got {}",
        duration
    );
}

#[test]
fn test_playback_config_frame_bytes_default() {
    let config = PlaybackConfig::default();
    let bytes = config.frame_bytes();
    // 960 * 2 * 4 (f32) = 7680
    assert_eq!(bytes, 7680);
}

#[test]
fn test_playback_config_frame_bytes_mono() {
    let config = PlaybackConfig::new(48000, 1, 960);
    let bytes = config.frame_bytes();
    // 960 * 1 * 4 = 3840
    assert_eq!(bytes, 3840);
}

#[test]
fn test_playback_config_clone() {
    let config = PlaybackConfig::new(44100, 1, 512);
    let cloned = config.clone();
    assert_eq!(config.sample_rate, cloned.sample_rate);
    assert_eq!(config.channels, cloned.channels);
    assert_eq!(config.buffer_size, cloned.buffer_size);
}

#[test]
fn test_playback_config_debug() {
    let config = PlaybackConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("48000"));
    assert!(debug_str.contains("PlaybackConfig"));
}

// ============================================================================
// DeviceInfo tests
// ============================================================================

#[test]
fn test_device_info_list_devices() {
    let result = PlaybackDevice::list_devices();
    // Should succeed even if no devices are available
    assert!(result.is_ok());
}

#[test]
fn test_device_info_fields() {
    let devices = PlaybackDevice::list_devices().unwrap();
    if !devices.is_empty() {
        let device = &devices[0];
        assert!(!device.name.is_empty());
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
// PlaybackDevice tests (non-hardware)
// ============================================================================

#[test]
fn test_playback_device_default_device() {
    let result = PlaybackDevice::default_device();
    // May fail in CI without audio devices
    if let Ok(device) = result {
        assert!(!device.name.is_empty());
        assert!(device.is_default);
    }
}

#[test]
fn test_playback_device_new_default() {
    let config = PlaybackConfig::default();
    let result = PlaybackDevice::new_default(config);
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
fn test_playback_device_new_with_specific_device() {
    let devices = PlaybackDevice::list_devices().unwrap();
    if let Some(device_info) = devices.first() {
        let config = PlaybackConfig::default();
        let result = PlaybackDevice::new(device_info, config);
        if let Ok(device) = result {
            assert_eq!(device.frame_size(), 960);
        }
    }
}

#[test]
fn test_playback_device_ring_buffer() {
    let config = PlaybackConfig::default();
    if let Ok(device) = PlaybackDevice::new_default(config) {
        let rb = device.ring_buffer();
        let rb2 = rb.clone();
        assert!(Arc::ptr_eq(&rb, &rb2));
    }
}

// ============================================================================
// PlaybackError tests
// ============================================================================

#[test]
fn test_playback_error_display() {
    let err = PlaybackError::DeviceNotFound("TestDevice".to_string());
    assert!(err.to_string().contains("TestDevice"));

    let err = PlaybackError::ConfigNotSupported("bad config".to_string());
    assert!(err.to_string().contains("bad config"));

    let err = PlaybackError::StreamError("stream err".to_string());
    assert!(err.to_string().contains("stream err"));

    let err = PlaybackError::DeviceUnavailable;
    assert!(err.to_string().contains("不可用"));
}

#[test]
fn test_playback_error_debug() {
    let err = PlaybackError::DeviceNotFound("Test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("DeviceNotFound"));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_playback_config_zero_buffer() {
    let config = PlaybackConfig::new(48000, 2, 0);
    assert_eq!(config.frame_bytes(), 0);
    assert!((config.frame_duration_ms() - 0.0).abs() < 0.001);
}

#[test]
fn test_playback_config_high_sample_rate() {
    let config = PlaybackConfig::new(192000, 2, 3840);
    assert!((config.frame_duration_ms() - 20.0).abs() < 0.01);
    assert_eq!(config.frame_bytes(), 30720);
}

#[test]
fn test_playback_config_many_channels() {
    let config = PlaybackConfig::new(48000, 8, 960);
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
