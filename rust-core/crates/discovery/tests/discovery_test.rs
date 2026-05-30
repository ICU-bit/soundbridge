use discovery::{DeviceDiscovery, DeviceInfo, DiscoveryConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let config = DiscoveryConfig::default();
        let _discovery = DeviceDiscovery::new(config);
    }

    #[test]
    fn test_default_config() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.service_name, "SoundBridge");
        assert_eq!(config.service_type, "_soundbridge._udp.local.");
    }

    #[test]
    fn test_device_info() {
        let info = DeviceInfo {
            name: "Test Device".to_string(),
            address: "192.168.1.100".parse().unwrap(),
            port: 12345,
            hostname: "test.local.".to_string(),
        };

        assert_eq!(info.name, "Test Device");
        assert_eq!(info.port, 12345);
    }
}
