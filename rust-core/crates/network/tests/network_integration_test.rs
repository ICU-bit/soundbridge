//! Integration tests for network crate.
//!
//! Tests the public API from an external perspective, verifying
//! cross-component interactions and real-world usage patterns.

use network::*;

// ============================================================================
// RawJitterBuffer integration tests
// ============================================================================

#[test]
fn test_raw_jitter_buffer_basic_operations() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    // Push packets
    jb.push(1, 100, vec![0x01, 0x02, 0x03]);
    jb.push(2, 200, vec![0x04, 0x05, 0x06]);
    jb.push(3, 300, vec![0x07, 0x08, 0x09]);

    // Pop in order
    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 1);
    assert_eq!(packet.timestamp, 100);
    assert_eq!(packet.data, vec![0x01, 0x02, 0x03]);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 2);
    assert_eq!(packet.timestamp, 200);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 3);
    assert_eq!(packet.timestamp, 300);

    assert!(jb.pop().is_none());
    assert!(jb.is_empty());
}

#[test]
fn test_raw_jitter_buffer_reorder() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    // Push out of order
    jb.push(3, 300, vec![0x07, 0x08, 0x09]);
    jb.push(1, 100, vec![0x01, 0x02, 0x03]);
    jb.push(2, 200, vec![0x04, 0x05, 0x06]);

    // Pop should be in order
    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 1);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 2);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 3);
}

#[test]
fn test_raw_jitter_buffer_skip_missing() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    // Skip sequence 1, push 2 and 3
    jb.push(2, 200, vec![0x04, 0x05, 0x06]);
    jb.push(3, 300, vec![0x07, 0x08, 0x09]);

    // First pop should skip to 2 (missing 1)
    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 2);
    assert_eq!(jb.next_sequence(), 3);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 3);
}

#[test]
fn test_raw_jitter_buffer_overflow() {
    let config = JitterBufferConfig {
        max_packets: 3,
        ..Default::default()
    };
    let mut jb = RawJitterBuffer::new(config);

    jb.push(1, 100, vec![0x01]);
    jb.push(2, 200, vec![0x02]);
    jb.push(3, 300, vec![0x03]);
    jb.push(4, 400, vec![0x04]); // Should discard oldest

    assert_eq!(jb.len(), 3);
}

#[test]
fn test_raw_jitter_buffer_clear() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    jb.push(1, 100, vec![0x01]);
    jb.push(2, 200, vec![0x02]);

    jb.clear();
    assert!(jb.is_empty());
    assert_eq!(jb.next_sequence(), 0);
}

#[test]
fn test_raw_jitter_buffer_adjust_delay() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    jb.adjust_delay(50);
    assert_eq!(jb.config().target_delay_ms, 50);

    // Test boundary values
    jb.adjust_delay(5); // Below min_delay_ms
    assert_eq!(jb.config().target_delay_ms, 20);

    jb.adjust_delay(300); // Above max_delay_ms
    assert_eq!(jb.config().target_delay_ms, 200);
}

#[test]
fn test_raw_jitter_buffer_stats() {
    let config = JitterBufferConfig::default();
    let mut jb = RawJitterBuffer::new(config);

    jb.push(1, 100, vec![0x01, 0x02]);
    jb.push(2, 200, vec![0x03, 0x04]);

    assert_eq!(jb.len(), 2);
    assert!(!jb.is_empty());

    jb.pop();
    assert_eq!(jb.len(), 1);
}

// ============================================================================
// JitterBuffer (f32) integration tests
// ============================================================================

#[test]
fn test_jitter_buffer_basic_operations() {
    let config = JitterBufferConfig::default();
    let mut jb = JitterBuffer::new(config);

    jb.push(1, vec![1.0f32; 100]);
    jb.push(2, vec![2.0f32; 100]);
    jb.push(3, vec![3.0f32; 100]);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 1);
    assert_eq!(packet.data.len(), 100);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 2);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 3);
}

#[test]
fn test_jitter_buffer_reorder() {
    let config = JitterBufferConfig::default();
    let mut jb = JitterBuffer::new(config);

    jb.push(3, vec![3.0f32; 100]);
    jb.push(1, vec![1.0f32; 100]);
    jb.push(2, vec![2.0f32; 100]);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 1);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 2);

    let packet = jb.pop().unwrap();
    assert_eq!(packet.sequence, 3);
}

#[test]
fn test_jitter_buffer_empty() {
    let config = JitterBufferConfig::default();
    let mut jb = JitterBuffer::new(config);
    assert!(jb.pop().is_none());
}

#[test]
fn test_jitter_buffer_stats() {
    let config = JitterBufferConfig::default();
    let mut jb = JitterBuffer::new(config);

    jb.push(1, vec![1.0f32; 100]);
    jb.push(2, vec![2.0f32; 100]);

    assert_eq!(jb.len(), 2);
    assert!(!jb.is_empty());
}

// ============================================================================
// ConnectionManager integration tests
// ============================================================================

#[test]
fn test_connection_manager_creation() {
    let config = ConnectionConfig::default();
    let manager = ConnectionManager::new(config);

    assert_eq!(manager.state(), ConnectionState::Disconnected);
    assert!(!manager.is_connected());
}

#[test]
fn test_connection_manager_state_transitions() {
    let config = ConnectionConfig::default();
    let manager = ConnectionManager::new(config);

    // Initial state
    assert_eq!(manager.state(), ConnectionState::Disconnected);
    assert!(!manager.is_connected());
}

#[test]
fn test_connection_state_equality() {
    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_eq!(ConnectionState::Connecting, ConnectionState::Connecting);
    assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
}

#[test]
fn test_connection_state_debug() {
    // ConnectionState derives Debug, not Display
    assert_eq!(
        format!("{:?}", ConnectionState::Disconnected),
        "Disconnected"
    );
    assert_eq!(format!("{:?}", ConnectionState::Connecting), "Connecting");
    assert_eq!(format!("{:?}", ConnectionState::Connected), "Connected");
}

// ============================================================================
// ConnectionType tests
// ============================================================================

#[test]
fn test_connection_type_variants() {
    let wifi_lan = ConnectionType::WiFiLan;
    let wifi_direct = ConnectionType::WiFiDirect;
    let usb_adb = ConnectionType::UsbAdb;
    let bluetooth = ConnectionType::Bluetooth;

    assert_ne!(wifi_lan, wifi_direct);
    assert_ne!(usb_adb, bluetooth);
}

#[test]
fn test_connection_type_clone() {
    let ct = ConnectionType::WiFiLan;
    let cloned = ct.clone();
    assert_eq!(ct, cloned);
}

#[test]
fn test_connection_type_copy() {
    let ct = ConnectionType::Bluetooth;
    let copied = ct;
    assert_eq!(ct, copied);
}

#[test]
fn test_connection_type_display() {
    assert_eq!(format!("{}", ConnectionType::WiFiLan), "WiFi LAN");
    assert_eq!(format!("{}", ConnectionType::WiFiDirect), "WiFi Direct");
    assert_eq!(format!("{}", ConnectionType::UsbAdb), "USB/ADB");
    assert_eq!(format!("{}", ConnectionType::Bluetooth), "Bluetooth");
}

// ============================================================================
// ConnectionConfig tests
// ============================================================================

#[test]
fn test_connection_config_default() {
    let config = ConnectionConfig::default();
    assert_eq!(config.heartbeat_interval_ms, 5000);
    assert_eq!(config.heartbeat_timeout_ms, 10000);
    assert_eq!(config.max_reconnect_attempts, 5);
    assert_eq!(config.reconnect_interval_ms, 1000);
}

#[test]
fn test_connection_config_custom() {
    let config = ConnectionConfig {
        heartbeat_interval_ms: 3000,
        heartbeat_timeout_ms: 6000,
        max_reconnect_attempts: 3,
        reconnect_interval_ms: 500,
    };
    assert_eq!(config.heartbeat_interval_ms, 3000);
    assert_eq!(config.heartbeat_timeout_ms, 6000);
    assert_eq!(config.max_reconnect_attempts, 3);
    assert_eq!(config.reconnect_interval_ms, 500);
}

// ============================================================================
// HotspotConfig/State tests
// ============================================================================

#[test]
fn test_hotspot_config_default() {
    let config = HotspotConfig::default();
    assert_eq!(config.ssid, "SoundBridge");
    assert_eq!(config.password, "soundbridge123");
    assert_eq!(config.channel, 6);
    assert_eq!(config.max_clients, 2);
}

#[test]
fn test_hotspot_state_variants() {
    assert_eq!(HotspotState::Idle, HotspotState::Idle);
    assert_eq!(HotspotState::Creating, HotspotState::Creating);
    assert_eq!(HotspotState::Running, HotspotState::Running);
    assert_eq!(HotspotState::Stopped, HotspotState::Stopped);
    assert_eq!(HotspotState::Error, HotspotState::Error);
    assert_ne!(HotspotState::Idle, HotspotState::Running);
}

// ============================================================================
// AdbConfig/State tests
// ============================================================================

#[test]
fn test_adb_config_default() {
    let config = AdbConfig::default();
    assert_eq!(config.adb_port, 5555);
    assert_eq!(config.local_port, 12345);
    assert_eq!(config.remote_port, 12345);
    assert!(config.device_serial.is_empty());
}

#[test]
fn test_adb_state_variants() {
    assert_eq!(AdbState::Disconnected, AdbState::Disconnected);
    assert_eq!(AdbState::DeviceConnected, AdbState::DeviceConnected);
    assert_eq!(AdbState::Forwarding, AdbState::Forwarding);
    assert_eq!(AdbState::Ready, AdbState::Ready);
    assert_eq!(AdbState::Error, AdbState::Error);
    assert_ne!(AdbState::Disconnected, AdbState::Ready);
}

// ============================================================================
// BluetoothConfig/State tests
// ============================================================================

#[test]
fn test_bluetooth_config_default() {
    let config = BluetoothConfig::default();
    assert_eq!(config.device_name, "SoundBridge");
    assert!(config.use_ble);
    assert!(!config.service_uuid.is_empty());
    assert!(!config.audio_char_uuid.is_empty());
}

#[test]
fn test_bluetooth_state_variants() {
    assert_eq!(BluetoothState::Idle, BluetoothState::Idle);
    assert_eq!(BluetoothState::AdapterReady, BluetoothState::AdapterReady);
    assert_eq!(BluetoothState::Scanning, BluetoothState::Scanning);
    assert_eq!(BluetoothState::Connected, BluetoothState::Connected);
    assert_eq!(BluetoothState::Streaming, BluetoothState::Streaming);
    assert_eq!(BluetoothState::Error, BluetoothState::Error);
    assert_ne!(BluetoothState::Idle, BluetoothState::Streaming);
}

// ============================================================================
// TransportConfig tests
// ============================================================================

#[test]
fn test_transport_config_default() {
    let config = TransportConfig::default();
    assert_eq!(config.send_buffer_size, 65536);
    assert_eq!(config.recv_buffer_size, 65536);
    assert_eq!(config.initial_bitrate, 128000);
    assert_eq!(config.min_bitrate, 32000);
    assert_eq!(config.max_bitrate, 256000);
    assert!((config.loss_threshold - 0.05).abs() < f32::EPSILON);
}

#[test]
fn test_transport_config_custom() {
    let config = TransportConfig {
        bind_addr: "127.0.0.1:8080".parse().unwrap(),
        send_buffer_size: 32768,
        recv_buffer_size: 32768,
        initial_bitrate: 64000,
        min_bitrate: 16000,
        max_bitrate: 128000,
        loss_threshold: 0.10,
    };
    assert_eq!(config.send_buffer_size, 32768);
    assert_eq!(config.initial_bitrate, 64000);
    assert!((config.loss_threshold - 0.10).abs() < f32::EPSILON);
}

// ============================================================================
// NetworkError tests
// ============================================================================

#[test]
fn test_network_error_display() {
    let err = NetworkError::ConnectionFailed("test".to_string());
    assert!(err.to_string().contains("test"));

    let err = NetworkError::SendFailed("send error".to_string());
    assert!(err.to_string().contains("send error"));

    let err = NetworkError::ReceiveFailed("recv error".to_string());
    assert!(err.to_string().contains("recv error"));

    let err = NetworkError::BindFailed("bind error".to_string());
    assert!(err.to_string().contains("bind error"));

    let err = NetworkError::Timeout;
    assert!(err.to_string().contains("超时"));

    let err = NetworkError::Disconnected;
    assert!(err.to_string().contains("断开"));
}

#[test]
fn test_network_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let net_err: NetworkError = io_err.into();
    assert!(net_err.to_string().contains("refused"));
}

// ============================================================================
// JitterBufferConfig tests
// ============================================================================

#[test]
fn test_jitter_buffer_config_default() {
    let config = JitterBufferConfig::default();
    assert!(config.max_packets > 0);
    assert!(config.target_delay_ms > 0);
    assert!(config.min_delay_ms > 0);
    assert!(config.max_delay_ms > 0);
    assert!(config.min_delay_ms <= config.target_delay_ms);
    assert!(config.target_delay_ms <= config.max_delay_ms);
}

#[test]
fn test_jitter_buffer_config_custom() {
    let config = JitterBufferConfig {
        max_packets: 50,
        target_delay_ms: 40,
        min_delay_ms: 10,
        max_delay_ms: 100,
    };
    assert_eq!(config.max_packets, 50);
    assert_eq!(config.target_delay_ms, 40);
    assert_eq!(config.min_delay_ms, 10);
    assert_eq!(config.max_delay_ms, 100);
}

// ============================================================================
// RawAudioPacket tests
// ============================================================================

#[test]
fn test_raw_audio_packet_fields() {
    let packet = RawAudioPacket {
        sequence: 42,
        timestamp: 12345,
        data: vec![0x01, 0x02, 0x03],
    };
    assert_eq!(packet.sequence, 42);
    assert_eq!(packet.timestamp, 12345);
    assert_eq!(packet.data, vec![0x01, 0x02, 0x03]);
}
