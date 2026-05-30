//! Integration tests for discovery crate.
//!
//! Tests the public API from an external perspective, verifying
//! cross-component interactions and real-world usage patterns.

use discovery::*;
use std::net::IpAddr;

// ============================================================================
// DiscoveryConfig tests
// ============================================================================

#[test]
fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();
    assert_eq!(config.service_name, "SoundBridge");
    assert_eq!(config.service_type, "_soundbridge._udp.local.");
    assert_eq!(config.port, 0);
    assert_eq!(config.timeout_ms, 3000);
}

#[test]
fn test_discovery_config_custom() {
    let config = DiscoveryConfig {
        service_name: "MyDevice".to_string(),
        service_type: "_myapp._tcp.local.".to_string(),
        port: 8080,
        timeout_ms: 5000,
    };
    assert_eq!(config.service_name, "MyDevice");
    assert_eq!(config.service_type, "_myapp._tcp.local.");
    assert_eq!(config.port, 8080);
    assert_eq!(config.timeout_ms, 5000);
}

#[test]
fn test_discovery_config_clone() {
    let config = DiscoveryConfig::default();
    let cloned = config.clone();
    assert_eq!(config.service_name, cloned.service_name);
    assert_eq!(config.service_type, cloned.service_type);
}

// ============================================================================
// DeviceInfo tests
// ============================================================================

#[test]
fn test_device_info_creation() {
    let info = DeviceInfo {
        name: "Test Device".to_string(),
        address: "192.168.1.100".parse().unwrap(),
        port: 12345,
        hostname: "test.local.".to_string(),
    };

    assert_eq!(info.name, "Test Device");
    assert_eq!(info.address, "192.168.1.100".parse::<IpAddr>().unwrap());
    assert_eq!(info.port, 12345);
    assert_eq!(info.hostname, "test.local.");
}

#[test]
fn test_device_info_clone() {
    let info = DeviceInfo {
        name: "Device".to_string(),
        address: "10.0.0.1".parse().unwrap(),
        port: 8080,
        hostname: "device.local.".to_string(),
    };

    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
    assert_eq!(info.address, cloned.address);
    assert_eq!(info.port, cloned.port);
}

#[test]
fn test_device_info_ipv6() {
    let info = DeviceInfo {
        name: "IPv6 Device".to_string(),
        address: "::1".parse().unwrap(),
        port: 9090,
        hostname: "ipv6.local.".to_string(),
    };

    assert_eq!(info.address, "::1".parse::<IpAddr>().unwrap());
}

// ============================================================================
// DeviceDiscovery tests
// ============================================================================

#[test]
fn test_device_discovery_creation() {
    let config = DiscoveryConfig::default();
    let discovery = DeviceDiscovery::new(config);
    let config_ref = discovery.config();
    assert_eq!(config_ref.service_name, "SoundBridge");
}

#[test]
fn test_device_discovery_with_default_config() {
    let discovery = DeviceDiscovery::with_default_config();
    let config_ref = discovery.config();
    assert_eq!(config_ref.service_name, "SoundBridge");
    assert_eq!(config_ref.service_type, "_soundbridge._udp.local.");
}

#[test]
fn test_device_discovery_config_accessor() {
    let config = DiscoveryConfig {
        service_name: "Custom".to_string(),
        service_type: "_custom._tcp.local.".to_string(),
        port: 5555,
        timeout_ms: 1000,
    };
    let discovery = DeviceDiscovery::new(config);
    let config_ref = discovery.config();
    assert_eq!(config_ref.service_name, "Custom");
    assert_eq!(config_ref.port, 5555);
}

// ============================================================================
// DeviceStore tests
// ============================================================================

#[test]
fn test_device_store_creation() {
    let store = DeviceStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn test_device_store_default() {
    let store = DeviceStore::default();
    assert!(store.is_empty());
}

#[test]
fn test_device_store_add_device() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Test Device", addr, 12345);

    assert_eq!(store.len(), 1);
    assert!(store.has_device("Test Device"));

    let device = store.get_device("Test Device").unwrap();
    assert_eq!(device.name, "Test Device");
    assert_eq!(device.address, "192.168.1.100");
    assert_eq!(device.port, 12345);
    assert_eq!(device.connection_count, 1);
}

#[test]
fn test_device_store_update_device() {
    let mut store = DeviceStore::new();
    let addr1: IpAddr = "192.168.1.100".parse().unwrap();
    let addr2: IpAddr = "192.168.1.200".parse().unwrap();

    store.add_device("Test Device", addr1, 12345);
    store.add_device("Test Device", addr2, 54321);

    assert_eq!(store.len(), 1);

    let device = store.get_device("Test Device").unwrap();
    assert_eq!(device.address, "192.168.1.200");
    assert_eq!(device.port, 54321);
    assert_eq!(device.connection_count, 2);
}

#[test]
fn test_device_store_multiple_devices() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Device 1", addr, 1111);
    store.add_device("Device 2", addr, 2222);
    store.add_device("Device 3", addr, 3333);

    assert_eq!(store.len(), 3);
    assert!(store.has_device("Device 1"));
    assert!(store.has_device("Device 2"));
    assert!(store.has_device("Device 3"));
}

#[test]
fn test_device_store_get_device_not_found() {
    let store = DeviceStore::new();
    assert!(store.get_device("NonExistent").is_none());
}

#[test]
fn test_device_store_get_all_devices() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Device 1", addr, 1111);
    store.add_device("Device 2", addr, 2222);

    let all = store.get_all_devices();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_device_store_auto_connect() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Test Device", addr, 12345);
    store.set_auto_connect("Test Device", true);

    let device = store.get_device("Test Device").unwrap();
    assert!(device.auto_connect);

    let auto_devices = store.get_auto_connect_devices();
    assert_eq!(auto_devices.len(), 1);
    assert_eq!(auto_devices[0].name, "Test Device");
}

#[test]
fn test_device_store_auto_connect_off() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Test Device", addr, 12345);
    store.set_auto_connect("Test Device", true);
    store.set_auto_connect("Test Device", false);

    let device = store.get_device("Test Device").unwrap();
    assert!(!device.auto_connect);

    let auto_devices = store.get_auto_connect_devices();
    assert_eq!(auto_devices.len(), 0);
}

#[test]
fn test_device_store_auto_connect_nonexistent() {
    let mut store = DeviceStore::new();
    // Should not panic
    store.set_auto_connect("NonExistent", true);
}

#[test]
fn test_device_store_remove_device() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Test Device", addr, 12345);
    assert!(store.has_device("Test Device"));

    assert!(store.remove_device("Test Device"));
    assert!(!store.has_device("Test Device"));
    assert_eq!(store.len(), 0);
}

#[test]
fn test_device_store_remove_nonexistent() {
    let mut store = DeviceStore::new();
    assert!(!store.remove_device("NonExistent"));
}

#[test]
fn test_device_store_clear() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Device 1", addr, 1111);
    store.add_device("Device 2", addr, 2222);
    assert_eq!(store.len(), 2);

    store.clear();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn test_device_store_connection_count() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Test Device", addr, 12345);
    assert_eq!(store.get_device("Test Device").unwrap().connection_count, 1);

    store.add_device("Test Device", addr, 12345);
    assert_eq!(store.get_device("Test Device").unwrap().connection_count, 2);

    store.add_device("Test Device", addr, 12345);
    assert_eq!(store.get_device("Test Device").unwrap().connection_count, 3);
}

#[test]
fn test_device_store_persistence() {
    let temp_dir = std::env::temp_dir().join("soundbridge_test_integration");
    let file_path = temp_dir.join("devices.json");

    // Clean up
    let _ = std::fs::remove_file(&file_path);

    // Create and save
    {
        let mut store = DeviceStore::with_file(&file_path);
        let addr: IpAddr = "192.168.1.100".parse().unwrap();
        store.add_device("Persisted Device", addr, 12345);
        store.set_auto_connect("Persisted Device", true);
    }

    // Reload and verify
    {
        let store = DeviceStore::with_file(&file_path);
        assert_eq!(store.len(), 1);
        let device = store.get_device("Persisted Device").unwrap();
        assert_eq!(device.address, "192.168.1.100");
        assert_eq!(device.port, 12345);
        assert!(device.auto_connect);
    }

    // Clean up
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_device_store_with_file_nonexistent() {
    let temp_dir = std::env::temp_dir().join("soundbridge_test_no_file");
    let file_path = temp_dir.join("nonexistent.json");

    // Should not panic when file doesn't exist
    let store = DeviceStore::with_file(&file_path);
    assert!(store.is_empty());

    // Clean up
    let _ = std::fs::remove_dir(&temp_dir);
}

// ============================================================================
// DiscoveryError tests
// ============================================================================

#[test]
fn test_discovery_error_display() {
    let err = DiscoveryError::DiscoveryFailed("test error".to_string());
    assert!(err.to_string().contains("test error"));

    let err = DiscoveryError::RegistrationFailed("reg error".to_string());
    assert!(err.to_string().contains("reg error"));
}

#[test]
fn test_discovery_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let err: DiscoveryError = io_err.into();
    assert!(err.to_string().contains("not found"));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_device_store_empty_name() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("", addr, 12345);
    assert!(store.has_device(""));
    assert_eq!(store.len(), 1);
}

#[test]
fn test_device_store_long_name() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();
    let long_name = "A".repeat(1000);

    store.add_device(&long_name, addr, 12345);
    assert!(store.has_device(&long_name));
}

#[test]
fn test_device_store_special_characters() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Device with spaces & special chars!@#", addr, 12345);
    assert!(store.has_device("Device with spaces & special chars!@#"));
}

#[test]
fn test_device_store_port_zero() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Zero Port", addr, 0);
    let device = store.get_device("Zero Port").unwrap();
    assert_eq!(device.port, 0);
}

#[test]
fn test_device_store_port_max() {
    let mut store = DeviceStore::new();
    let addr: IpAddr = "192.168.1.100".parse().unwrap();

    store.add_device("Max Port", addr, u16::MAX);
    let device = store.get_device("Max Port").unwrap();
    assert_eq!(device.port, u16::MAX);
}
