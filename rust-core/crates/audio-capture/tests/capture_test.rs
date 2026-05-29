use audio_capture::{CaptureDevice, CaptureConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let devices = CaptureDevice::list_devices().unwrap();
        println!("Capture devices: {}", devices.len());
        for device in &devices {
            println!("  - {} (default: {})", device.name, device.is_default);
        }
    }

    #[test]
    fn test_default_config() {
        let config = CaptureConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.buffer_size, 960);
    }

    #[test]
    fn test_config_frame_duration() {
        let config = CaptureConfig::default();
        let duration_ms = config.frame_duration_ms();
        assert!((duration_ms - 20.0).abs() < 0.01, "Frame duration should be 20ms, got {}", duration_ms);
    }
}
