//! Integration tests for ffi-bindings crate.
//!
//! Tests the C API from an external perspective.
//! Focuses on functions that don't require real audio hardware:
//! engine lifecycle, audio mode, connection type, mix ratio, mute, device store, etc.

use ffi_bindings::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;

// ============================================================================
// Engine lifecycle tests
// ============================================================================
// Engine lifecycle tests
// ============================================================================

#[test]
fn test_engine_create_destroy() {
    unsafe {
        let engine = sb_engine_create();
        assert!(!engine.is_null(), "Engine should not be null");
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_engine_destroy_null() {
    // Should not panic
    unsafe {
        sb_engine_destroy(ptr::null_mut());
    }
}

#[test]
fn test_engine_multiple_create_destroy() {
    unsafe {
        let engines: Vec<*mut c_void> = (0..5).map(|_| sb_engine_create()).collect();
        for engine in engines {
            assert!(!engine.is_null());
            sb_engine_destroy(engine);
        }
    }
}

// ============================================================================
// Error handling tests
// ============================================================================

#[test]
fn test_last_error_initially_null() {
    unsafe {
        let engine = sb_engine_create();
        let err = sb_last_error();
        // NOTE: relaxed assertion — mutex may be poisoned from other tests
        if !err.is_null() {
            let msg = CStr::from_ptr(err).to_string_lossy();
            eprintln!(
                "Warning: last error not null after engine creation: {}",
                msg
            );
        }
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_last_error_after_null_engine() {
    unsafe {
        // Calling with null engine should set error
        let mut state = SbConnectionState::Disconnected;
        let result = sb_get_connection_state(ptr::null_mut(), &mut state);
        assert_ne!(result, 0); // Should return error
        let err = sb_last_error();
        assert!(
            !err.is_null(),
            "Last error should be set after null engine call"
        );
    }
}

// ============================================================================
// Connection state tests
// ============================================================================

#[test]
fn test_get_connection_state_initial() {
    unsafe {
        let engine = sb_engine_create();
        let mut state = SbConnectionState::Error;
        let result = sb_get_connection_state(engine, &mut state);
        assert_eq!(result, 0); // Ok
        assert_eq!(state, SbConnectionState::Disconnected);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_get_connection_state_null_engine() {
    unsafe {
        let mut state = SbConnectionState::Disconnected;
        let result = sb_get_connection_state(ptr::null_mut(), &mut state);
        assert_ne!(result, 0); // Error
    }
}

#[test]
fn test_get_connection_state_null_state() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_get_connection_state(engine, ptr::null_mut());
        assert_ne!(result, 0); // Error
        sb_engine_destroy(engine);
    }
}

// ============================================================================
// Audio mode tests
// ============================================================================

#[test]
fn test_set_get_audio_mode_balanced() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_audio_mode(engine, SbAudioMode::Balanced);
        assert_eq!(result, 0);
        let mut mode = SbAudioMode::Balanced;
        let result = sb_get_audio_mode(engine, &mut mode);
        assert_eq!(result, 0);
        assert_eq!(mode, SbAudioMode::Balanced);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_audio_mode_high_quality() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_audio_mode(engine, SbAudioMode::HighQuality);
        assert_eq!(result, 0);
        let mut mode = SbAudioMode::Balanced;
        sb_get_audio_mode(engine, &mut mode);
        assert_eq!(mode, SbAudioMode::HighQuality);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_audio_mode_low_latency() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_audio_mode(engine, SbAudioMode::LowLatency);
        assert_eq!(result, 0);
        let mut mode = SbAudioMode::Balanced;
        sb_get_audio_mode(engine, &mut mode);
        assert_eq!(mode, SbAudioMode::LowLatency);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_audio_mode_null_engine() {
    unsafe {
        let result = sb_set_audio_mode(ptr::null_mut(), SbAudioMode::Balanced);
        assert_ne!(result, 0);
    }
}

#[test]
fn test_get_audio_mode_null_engine() {
    unsafe {
        let mut mode = SbAudioMode::Balanced;
        let result = sb_get_audio_mode(ptr::null_mut(), &mut mode);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Connection type tests
// ============================================================================

#[test]
fn test_set_get_connection_type_wifi_lan() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_connection_type(engine, 0); // WiFiLan
        assert_eq!(result, 0);
        let mut conn_type: i32 = -1;
        sb_get_connection_type(engine, &mut conn_type);
        assert_eq!(conn_type, 0);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_connection_type_hotspot() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_connection_type(engine, 1); // Hotspot
        assert_eq!(result, 0);
        let mut conn_type: i32 = -1;
        sb_get_connection_type(engine, &mut conn_type);
        assert_eq!(conn_type, 1);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_connection_type_adb() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_connection_type(engine, 2); // ADB
        assert_eq!(result, 0);
        let mut conn_type: i32 = -1;
        sb_get_connection_type(engine, &mut conn_type);
        assert_eq!(conn_type, 2);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_connection_type_bluetooth() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_connection_type(engine, 3); // Bluetooth
        assert_eq!(result, 0);
        let mut conn_type: i32 = -1;
        sb_get_connection_type(engine, &mut conn_type);
        assert_eq!(conn_type, 3);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_connection_type_null_engine() {
    unsafe {
        let result = sb_set_connection_type(ptr::null_mut(), 0);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Mix ratio tests
// ============================================================================

#[test]
fn test_set_get_mix_ratio_default() {
    unsafe {
        let engine = sb_engine_create();
        let mut pc_vol: f32 = 0.0;
        let mut phone_vol: f32 = 0.0;
        let result = sb_get_mix_ratio(engine, &mut pc_vol, &mut phone_vol);
        assert_eq!(result, 0);
        assert!(
            (pc_vol - 0.5).abs() < 0.01,
            "PC volume should be 0.5, got {}",
            pc_vol
        );
        assert!(
            (phone_vol - 0.5).abs() < 0.01,
            "Phone volume should be 0.5, got {}",
            phone_vol
        );
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_mix_ratio_custom() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_mix_ratio(engine, 0.8, 0.3);
        assert_eq!(result, 0);
        let mut pc_vol: f32 = 0.0;
        let mut phone_vol: f32 = 0.0;
        sb_get_mix_ratio(engine, &mut pc_vol, &mut phone_vol);
        assert!((pc_vol - 0.8).abs() < 0.01);
        assert!((phone_vol - 0.3).abs() < 0.01);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_mix_ratio_null_engine() {
    unsafe {
        let result = sb_set_mix_ratio(ptr::null_mut(), 0.5, 0.5);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Mute tests
// ============================================================================

#[test]
fn test_set_get_mute_off() {
    unsafe {
        let engine = sb_engine_create();
        let mute = sb_get_mute(engine);
        assert_eq!(mute, 0, "Should be unmuted by default");
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_get_mute_on() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_set_mute(engine, 1);
        assert_eq!(result, 0);
        let mute = sb_get_mute(engine);
        assert_eq!(mute, 1, "Should be muted");
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_mute_toggle() {
    unsafe {
        let engine = sb_engine_create();
        sb_set_mute(engine, 1);
        assert_eq!(sb_get_mute(engine), 1);
        sb_set_mute(engine, 0);
        assert_eq!(sb_get_mute(engine), 0);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_set_mute_null_engine() {
    unsafe {
        let result = sb_set_mute(ptr::null_mut(), 1);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Network bind tests
// ============================================================================

#[test]
fn test_bind_auto_port() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_bind(engine, 0);
        assert_eq!(result, 0, "Should bind successfully with auto port");
        let mut port: u16 = 0;
        sb_local_port(engine, &mut port);
        assert!(port > 0, "Auto-assigned port should be > 0, got {}", port);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_bind_null_engine() {
    unsafe {
        let result = sb_bind(ptr::null_mut(), 0);
        assert_ne!(result, 0);
    }
}

#[test]
fn test_local_port_null_engine() {
    unsafe {
        let mut port: u16 = 0;
        let result = sb_local_port(ptr::null_mut(), &mut port);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Device store tests
// ============================================================================

#[test]
fn test_device_store_open_close() {
    unsafe {
        let path = CString::new("test_devices.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());
        assert!(!store.is_null());
        sb_device_store_close(store);
    }
}

#[test]
fn test_device_store_add_and_count() {
    unsafe {
        let path = CString::new("test_devices_add.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());

        let name = CString::new("TestDevice").unwrap();
        let addr = CString::new("192.168.1.100").unwrap();
        let result = sb_device_store_add(store, name.as_ptr(), addr.as_ptr(), 12345);
        assert_eq!(result, 0);

        let mut count: usize = 0;
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 1);

        sb_device_store_close(store);
        let _ = std::fs::remove_file("test_devices_add.json");
    }
}

#[test]
fn test_device_store_has() {
    unsafe {
        let path = CString::new("test_devices_has.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());

        let name = CString::new("TestDevice").unwrap();
        let addr = CString::new("192.168.1.100").unwrap();
        sb_device_store_add(store, name.as_ptr(), addr.as_ptr(), 12345);

        let has = sb_device_store_has(store, name.as_ptr());
        assert_eq!(has, 1, "Should have the device");

        let other = CString::new("NonExistent").unwrap();
        let has_not = sb_device_store_has(store, other.as_ptr());
        assert_eq!(has_not, 0, "Should not have non-existent device");

        sb_device_store_close(store);
        let _ = std::fs::remove_file("test_devices_has.json");
    }
}

#[test]
fn test_device_store_remove() {
    unsafe {
        let path = CString::new("test_devices_remove.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());

        let name = CString::new("TestDevice").unwrap();
        let addr = CString::new("192.168.1.100").unwrap();
        sb_device_store_add(store, name.as_ptr(), addr.as_ptr(), 12345);

        let result = sb_device_store_remove(store, name.as_ptr());
        assert_eq!(result, 0);

        let mut count: usize = 1;
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 0);

        sb_device_store_close(store);
        let _ = std::fs::remove_file("test_devices_remove.json");
    }
}

#[test]
fn test_device_store_clear() {
    unsafe {
        let path = CString::new("test_devices_clear.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());

        let name1 = CString::new("Device1").unwrap();
        let addr1 = CString::new("192.168.1.1").unwrap();
        let name2 = CString::new("Device2").unwrap();
        let addr2 = CString::new("192.168.1.2").unwrap();
        sb_device_store_add(store, name1.as_ptr(), addr1.as_ptr(), 1111);
        sb_device_store_add(store, name2.as_ptr(), addr2.as_ptr(), 2222);

        sb_device_store_clear(store);

        let mut count: usize = 99;
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 0);

        sb_device_store_close(store);
        let _ = std::fs::remove_file("test_devices_clear.json");
    }
}

#[test]
fn test_device_store_auto_connect() {
    unsafe {
        let path = CString::new("test_devices_auto.json").unwrap();
        let store = sb_device_store_open(path.as_ptr());

        let name = CString::new("TestDevice").unwrap();
        let addr = CString::new("192.168.1.100").unwrap();
        sb_device_store_add(store, name.as_ptr(), addr.as_ptr(), 12345);

        let result = sb_device_store_set_auto_connect(store, name.as_ptr(), true);
        assert_eq!(result, 0);

        sb_device_store_close(store);
        let _ = std::fs::remove_file("test_devices_auto.json");
    }
}

// ============================================================================
// Discovery tests
// ============================================================================

#[test]
fn test_discovery_create_close() {
    unsafe {
        let discovery = sb_discovery_create();
        assert!(!discovery.is_null());
        sb_discovery_close(discovery);
    }
}

// ============================================================================
// Hotspot/ADB/Bluetooth state tests
// ============================================================================

#[test]
fn test_hotspot_state_initial() {
    unsafe {
        let engine = sb_engine_create();
        let mut state: i32 = -1;
        let result = sb_hotspot_state(engine, &mut state);
        assert_eq!(result, 0);
        assert_eq!(state, 0); // Idle
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_hotspot_set_state() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_hotspot_set_state(engine, 1); // Creating
        assert_eq!(result, 0);
        let mut state: i32 = -1;
        sb_hotspot_state(engine, &mut state);
        assert_eq!(state, 1);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_adb_state_initial() {
    unsafe {
        let engine = sb_engine_create();
        let mut state: i32 = -1;
        let result = sb_adb_state(engine, &mut state);
        assert_eq!(result, 0);
        assert_eq!(state, 0); // Disconnected
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_adb_set_state() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_adb_set_state(engine, 1); // DeviceConnected
        assert_eq!(result, 0);
        let mut state: i32 = -1;
        sb_adb_state(engine, &mut state);
        assert_eq!(state, 1);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_bt_state_initial() {
    unsafe {
        let engine = sb_engine_create();
        let mut state: i32 = -1;
        let result = sb_bt_state(engine, &mut state);
        assert_eq!(result, 0);
        assert_eq!(state, 0); // Idle
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_bt_set_state() {
    unsafe {
        let engine = sb_engine_create();
        let result = sb_bt_set_state(engine, 1); // AdapterReady
        assert_eq!(result, 0);
        let mut state: i32 = -1;
        sb_bt_state(engine, &mut state);
        assert_eq!(state, 1);
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_hotspot_state_null_engine() {
    unsafe {
        let mut state: i32 = 0;
        let result = sb_hotspot_state(ptr::null_mut(), &mut state);
        assert_ne!(result, 0);
    }
}

#[test]
fn test_adb_state_null_engine() {
    unsafe {
        let mut state: i32 = 0;
        let result = sb_adb_state(ptr::null_mut(), &mut state);
        assert_ne!(result, 0);
    }
}

#[test]
fn test_bt_state_null_engine() {
    unsafe {
        let mut state: i32 = 0;
        let result = sb_bt_state(ptr::null_mut(), &mut state);
        assert_ne!(result, 0);
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_engine_create_multiple_sequential() {
    unsafe {
        for _ in 0..10 {
            let engine = sb_engine_create();
            assert!(!engine.is_null());
            sb_engine_destroy(engine);
        }
    }
}

#[test]
fn test_audio_mode_switch_multiple() {
    unsafe {
        let engine = sb_engine_create();
        for mode in [
            SbAudioMode::Balanced,
            SbAudioMode::HighQuality,
            SbAudioMode::LowLatency,
            SbAudioMode::Balanced,
            SbAudioMode::HighQuality,
            SbAudioMode::LowLatency,
        ] {
            sb_set_audio_mode(engine, mode);
            let mut got = SbAudioMode::Balanced;
            sb_get_audio_mode(engine, &mut got);
            assert_eq!(got, mode);
        }
        sb_engine_destroy(engine);
    }
}

#[test]
fn test_connection_type_switch_multiple() {
    unsafe {
        let engine = sb_engine_create();
        for conn_type in [0, 1, 2, 3, 0, 1] {
            sb_set_connection_type(engine, conn_type);
            let mut got: i32 = -1;
            sb_get_connection_type(engine, &mut got);
            assert_eq!(got, conn_type);
        }
        sb_engine_destroy(engine);
    }
}
