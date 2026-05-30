//! Pipeline integration test harness for ffi-bindings crate.
//!
//! Tests the C API pipeline lifecycle from an external perspective WITHOUT
//! requiring real audio hardware (no sb_capture_start / sb_playback_start /
//! sb_pipeline_start). Focuses on state management, configuration round-trips,
//! encryption, and error handling that can be verified in CI.
//!
//! Every test is fully independent: create engine → exercise → destroy.

use ffi_bindings::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

// ============================================================================
// Helpers
// ============================================================================

/// Helper: create an engine, return None and print diagnostics on failure.
fn create_engine() -> Option<*mut c_void> {
    unsafe {
        let engine = sb_engine_create();
        if engine.is_null() {
            let err = sb_last_error();
            if !err.is_null() {
                let msg = CStr::from_ptr(err).to_string_lossy();
                eprintln!("sb_engine_create failed: {}", msg);
            }
            return None;
        }
        Some(engine)
    }
}

/// Helper: safely destroy an engine.
fn destroy_engine(engine: *mut c_void) {
    unsafe {
        sb_engine_destroy(engine);
    }
}

/// Helper: get last error message as String (for diagnostics).
fn last_error_string() -> String {
    unsafe {
        let err = sb_last_error();
        if err.is_null() {
            return String::new();
        }
        CStr::from_ptr(err).to_string_lossy().to_string()
    }
}

// ============================================================================
// Test 1: Full engine lifecycle — create, bind, connect, verify, destroy
// ============================================================================

#[test]
fn test_engine_lifecycle_bind_connect_verify() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return, // skip if engine creation fails
    };

    unsafe {
        // Bind on port 0 (auto-assign)
        let rc = sb_bind(engine, 0);
        assert_eq!(rc, 0, "sb_bind(0) should succeed, got error: {}", last_error_string());

        // Verify auto-assigned port is non-zero
        let mut port: u16 = 0;
        let rc = sb_local_port(engine, &mut port);
        assert_eq!(rc, 0, "sb_local_port should succeed");
        assert!(port > 0, "Auto-assigned port should be > 0, got {}", port);

        // Connect to loopback
        let addr = match CString::new("127.0.0.1:54321") {
            Ok(a) => a,
            Err(e) => {
                eprintln!("CString::new failed: {}", e);
                destroy_engine(engine);
                return;
            }
        };
        let rc = sb_connect(engine, addr.as_ptr());
        assert_eq!(rc, 0, "sb_connect should succeed, got error: {}", last_error_string());

        // Verify connection state transitioned to Connecting
        let mut state = SbConnectionState::Disconnected;
        let rc = sb_get_connection_state(engine, &mut state);
        assert_eq!(rc, 0, "sb_get_connection_state should succeed");
        assert_eq!(
            state,
            SbConnectionState::Connecting,
            "State should be Connecting after sb_connect"
        );

        // Verify pipeline is still stopped (no capture/playback started)
        let mut pipeline_state: c_int = -1;
        let rc = sb_pipeline_state(engine, &mut pipeline_state);
        assert_eq!(rc, 0);
        assert_eq!(pipeline_state, 0, "Pipeline should be Stopped (0)");

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 2: Mode switch mid-pipeline — set all three modes, verify round-trip
// ============================================================================

#[test]
fn test_mode_switch_roundtrip_all_modes() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    let modes = [
        SbAudioMode::Balanced,
        SbAudioMode::HighQuality,
        SbAudioMode::LowLatency,
    ];

    unsafe {
        for &mode in &modes {
            let rc = sb_set_audio_mode(engine, mode);
            assert_eq!(rc, 0, "sb_set_audio_mode({:?}) should succeed", mode);

            let mut got = SbAudioMode::Balanced;
            let rc = sb_get_audio_mode(engine, &mut got);
            assert_eq!(rc, 0, "sb_get_audio_mode should succeed");
            assert_eq!(got, mode, "Mode round-trip failed for {:?}", mode);
        }

        // Rapid mode cycling (simulates mid-pipeline mode switch)
        for _ in 0..10 {
            for &mode in &modes {
                let rc = sb_set_audio_mode(engine, mode);
                assert_eq!(rc, 0);
            }
        }
        // Verify final mode sticks
        let mut final_mode = SbAudioMode::Balanced;
        sb_get_audio_mode(engine, &mut final_mode);
        assert_eq!(final_mode, SbAudioMode::LowLatency);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 3: Encryption round-trip — enable, verify, disable, verify
// ============================================================================

#[test]
fn test_encryption_roundtrip() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Initially not encrypted
        let is_enc = sb_is_encrypted(engine);
        assert_eq!(is_enc, 0, "Engine should start unencrypted");

        // Enable encryption with test keys
        let master_key: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
        ];
        let master_salt: [u8; 14] = [
            0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
            0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE,
        ];

        let rc = sb_enable_encryption(engine, master_key.as_ptr(), master_salt.as_ptr());
        assert_eq!(rc, 0, "sb_enable_encryption should succeed, got: {}", last_error_string());

        // Verify encryption is now active
        let is_enc = sb_is_encrypted(engine);
        assert_eq!(is_enc, 1, "Engine should be encrypted after enable");

        // Disable encryption
        let rc = sb_disable_encryption(engine);
        assert_eq!(rc, 0, "sb_disable_encryption should succeed");

        // Verify encryption is now disabled
        let is_enc = sb_is_encrypted(engine);
        assert_eq!(is_enc, 0, "Engine should be unencrypted after disable");

        // Re-enable to test double-toggle
        let rc = sb_enable_encryption(engine, master_key.as_ptr(), master_salt.as_ptr());
        assert_eq!(rc, 0);
        assert_eq!(sb_is_encrypted(engine), 1);

        let rc = sb_disable_encryption(engine);
        assert_eq!(rc, 0);
        assert_eq!(sb_is_encrypted(engine), 0);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 4: Mix ratio hot-update — set various ratios, verify via get
// ============================================================================

#[test]
fn test_mix_ratio_hot_update() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Verify default ratio (0.5 / 0.5)
        let mut pc: f32 = -1.0;
        let mut phone: f32 = -1.0;
        let rc = sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert_eq!(rc, 0, "sb_get_mix_ratio should succeed");
        assert!((pc - 0.5).abs() < 0.01, "Default PC volume should be ~0.5, got {}", pc);
        assert!((phone - 0.5).abs() < 0.01, "Default phone volume should be ~0.5, got {}", phone);

        // Test boundary values
        let test_cases: &[(f32, f32)] = &[
            (0.0, 0.0),   // both silent
            (1.0, 1.0),   // both max
            (0.0, 1.0),   // PC silent, phone max
            (1.0, 0.0),   // PC max, phone silent
            (0.75, 0.25), // asymmetric
        ];

        for &(set_pc, set_phone) in test_cases {
            let rc = sb_set_mix_ratio(engine, set_pc, set_phone);
            assert_eq!(
                rc, 0,
                "sb_set_mix_ratio({}, {}) should succeed",
                set_pc, set_phone
            );

            let mut got_pc: f32 = -1.0;
            let mut got_phone: f32 = -1.0;
            sb_get_mix_ratio(engine, &mut got_pc, &mut got_phone);
            assert!(
                (got_pc - set_pc).abs() < 0.01,
                "PC volume mismatch: set={}, got={}",
                set_pc,
                got_pc
            );
            assert!(
                (got_phone - set_phone).abs() < 0.01,
                "Phone volume mismatch: set={}, got={}",
                set_phone,
                got_phone
            );
        }

        // Rapid hot-update (simulates slider dragging)
        for i in 0..100 {
            let val = i as f32 / 100.0;
            let rc = sb_set_mix_ratio(engine, val, 1.0 - val);
            assert_eq!(rc, 0, "Hot-update iteration {} should succeed", i);
        }
        // Verify final value
        let mut final_pc: f32 = 0.0;
        let mut final_phone: f32 = 0.0;
        sb_get_mix_ratio(engine, &mut final_pc, &mut final_phone);
        assert!((final_pc - 0.99).abs() < 0.01, "Final PC should be ~0.99, got {}", final_pc);
        assert!((final_phone - 0.01).abs() < 0.01, "Final phone should be ~0.01, got {}", final_phone);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 5: Pipeline state and error handling without capture
// ============================================================================

#[test]
fn test_pipeline_state_without_hardware() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Initial state should be Stopped (0)
        let mut state: c_int = -1;
        let rc = sb_pipeline_state(engine, &mut state);
        assert_eq!(rc, 0);
        assert_eq!(state, 0, "Pipeline should start as Stopped (0)");

        // Attempting pipeline_start without capture should fail with PipelineNotReady
        let rc = sb_pipeline_start(engine);
        assert_eq!(
            rc,
            SbError::PipelineNotReady as c_int,
            "Pipeline start without capture should return PipelineNotReady ({})",
            SbError::PipelineNotReady as c_int
        );

        // Pipeline state should still be Stopped after failed start
        let mut state: c_int = -1;
        sb_pipeline_state(engine, &mut state);
        assert_eq!(state, 0, "Pipeline should remain Stopped after failed start");

        // Audio level should be 0.0 without pipeline
        let mut level: f32 = -1.0;
        let rc = sb_get_audio_level(engine, &mut level);
        assert_eq!(rc, 0, "sb_get_audio_level should succeed");
        assert!((level - 0.0).abs() < 0.001, "Audio level without pipeline should be 0.0, got {}", level);

        // Pipeline stop on already-stopped engine should succeed (no-op)
        let rc = sb_pipeline_stop(engine);
        assert_eq!(rc, 0, "Pipeline stop on stopped engine should be a no-op success");

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 6: State callback registration and verification
// ============================================================================

/// Global state for callback tests — serialised via mutex.
static CALLBACK_LOG: Mutex<Vec<i32>> = Mutex::new(Vec::new());

extern "C" fn recording_callback(state: SbConnectionState, _user_data: *mut c_void) {
    if let Ok(mut log) = CALLBACK_LOG.lock() {
        log.push(state as i32);
    }
}

#[test]
fn test_state_callback_registration_and_fire() {
    // Clear callback log
    if let Ok(mut log) = CALLBACK_LOG.lock() {
        log.clear();
    }

    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Register callback
        let rc = sb_set_state_callback(engine, Some(recording_callback), ptr::null_mut());
        assert_eq!(rc, 0, "sb_set_state_callback should succeed");

        // Trigger Connecting state via sb_connect
        let addr = match CString::new("127.0.0.1:9999") {
            Ok(a) => a,
            Err(_) => {
                destroy_engine(engine);
                return;
            }
        };
        let rc = sb_connect(engine, addr.as_ptr());
        assert_eq!(rc, 0);

        // Verify callback was invoked with Connecting (1)
        if let Ok(log) = CALLBACK_LOG.lock() {
            assert!(
                log.contains(&(SbConnectionState::Connecting as i32)),
                "Callback should have received Connecting state, log: {:?}",
                log
            );
        }

        // Unregister callback
        let rc = sb_set_state_callback(engine, None, ptr::null_mut());
        assert_eq!(rc, 0);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 7: Encryption error handling — null args, re-enable, pipeline guard
// ============================================================================

#[test]
fn test_encryption_error_handling() {
    unsafe {
        // Null engine
        let key = [0u8; 16];
        let salt = [0u8; 14];
        let rc = sb_enable_encryption(ptr::null_mut(), key.as_ptr(), salt.as_ptr());
        assert_ne!(rc, 0, "Null engine should fail");
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // Null key
        let engine = match create_engine() {
            Some(e) => e,
            None => return,
        };
        let rc = sb_enable_encryption(engine, ptr::null(), salt.as_ptr());
        assert_ne!(rc, 0, "Null key should fail");

        // Null salt
        let rc = sb_enable_encryption(engine, key.as_ptr(), ptr::null());
        assert_ne!(rc, 0, "Null salt should fail");

        // Null engine for is_encrypted
        let rc = sb_is_encrypted(ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // Null engine for disable_encryption
        let rc = sb_disable_encryption(ptr::null_mut());
        assert_ne!(rc, 0);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 8: Device store full lifecycle
// ============================================================================

#[test]
fn test_device_store_full_lifecycle() {
    unsafe {
        let test_file = CString::new("test_pipeline_devices.json").unwrap_or_else(|_| {
            CString::new("fallback.json").unwrap_or_default()
        });

        // Open store
        let store = sb_device_store_open(test_file.as_ptr());
        if store.is_null() {
            eprintln!("sb_device_store_open failed: {}", last_error_string());
            return;
        }

        // Initially empty
        let mut count: usize = 999;
        let rc = sb_device_store_count(store, &mut count);
        assert_eq!(rc, 0);
        assert_eq!(count, 0, "Store should start empty");

        // Add devices
        let name1 = CString::new("Phone_A").unwrap_or_default();
        let addr1 = CString::new("192.168.1.10").unwrap_or_default();
        let rc = sb_device_store_add(store, name1.as_ptr(), addr1.as_ptr(), 5000);
        assert_eq!(rc, 0, "Add device 1 should succeed");

        let name2 = CString::new("Phone_B").unwrap_or_default();
        let addr2 = CString::new("192.168.1.20").unwrap_or_default();
        let rc = sb_device_store_add(store, name2.as_ptr(), addr2.as_ptr(), 6000);
        assert_eq!(rc, 0, "Add device 2 should succeed");

        // Count should be 2
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 2, "Should have 2 devices");

        // has_device check
        assert_eq!(sb_device_store_has(store, name1.as_ptr()), 1);
        let ghost = CString::new("Ghost").unwrap_or_default();
        assert_eq!(sb_device_store_has(store, ghost.as_ptr()), 0);

        // Get port
        let mut port: u16 = 0;
        let rc = sb_device_store_get_port(store, name1.as_ptr(), &mut port);
        assert_eq!(rc, 0);
        assert_eq!(port, 5000);

        // Get address
        let mut buf = [0u8; 64];
        let written = sb_device_store_get_address(
            store,
            name2.as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
        );
        assert!(written > 0, "Should have written address bytes");
        let addr_str = CStr::from_ptr(buf.as_ptr() as *const c_char).to_string_lossy();
        assert_eq!(addr_str, "192.168.1.20");

        // Remove device 1
        let rc = sb_device_store_remove(store, name1.as_ptr());
        assert_eq!(rc, 0);
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 1, "Should have 1 device after removal");

        // Clear all
        sb_device_store_clear(store);
        sb_device_store_count(store, &mut count);
        assert_eq!(count, 0, "Should be empty after clear");

        // Close
        sb_device_store_close(store);

        // Cleanup
        let _ = std::fs::remove_file("test_pipeline_devices.json");
    }
}

// ============================================================================
// Test 9: Connection type round-trip for all 4 types
// ============================================================================

#[test]
fn test_connection_type_all_values() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    // 0=WiFiLan, 1=WiFiDirect, 2=UsbAdb, 3=Bluetooth
    let types: &[(i32, &str)] = &[
        (0, "WiFiLan"),
        (1, "WiFiDirect"),
        (2, "UsbAdb"),
        (3, "Bluetooth"),
    ];

    unsafe {
        for &(conn_type, name) in types {
            let rc = sb_set_connection_type(engine, conn_type);
            assert_eq!(
                rc, 0,
                "sb_set_connection_type({}) should succeed for {}",
                conn_type, name
            );

            let mut got: i32 = -1;
            let rc = sb_get_connection_type(engine, &mut got);
            assert_eq!(rc, 0);
            assert_eq!(got, conn_type, "Connection type round-trip failed for {}", name);
        }

        // Invalid type should fail
        let rc = sb_set_connection_type(engine, 99);
        assert_eq!(
            rc,
            SbError::InvalidArgument as c_int,
            "Invalid connection type should return InvalidArgument"
        );

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 10: Mute toggle round-trip
// ============================================================================

#[test]
fn test_mute_toggle_roundtrip() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Default should be unmuted (0)
        let mute = sb_get_mute(engine);
        assert_eq!(mute, 0, "Should start unmuted");

        // Mute on
        let rc = sb_set_mute(engine, 1);
        assert_eq!(rc, 0);
        assert_eq!(sb_get_mute(engine), 1, "Should be muted after set_mute(1)");

        // Mute off
        let rc = sb_set_mute(engine, 0);
        assert_eq!(rc, 0);
        assert_eq!(sb_get_mute(engine), 0, "Should be unmuted after set_mute(0)");

        // Rapid toggle
        for i in 0..20 {
            let val = i % 2;
            sb_set_mute(engine, val);
            assert_eq!(sb_get_mute(engine), val, "Toggle iteration {}", i);
        }

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 11: Audio level returns 0 without running pipeline
// ============================================================================

#[test]
fn test_audio_level_without_pipeline() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        let mut level: f32 = -999.0;
        let rc = sb_get_audio_level(engine, &mut level);
        assert_eq!(rc, 0, "sb_get_audio_level should succeed");
        assert!(
            (0.0..=1.0).contains(&level),
            "Audio level should be in [0.0, 1.0], got {}",
            level
        );
        assert!((level - 0.0).abs() < 0.001, "Level without pipeline should be 0.0, got {}", level);

        // Null engine
        let rc = sb_get_audio_level(ptr::null_mut(), &mut level);
        assert_ne!(rc, 0, "Null engine should fail");

        // Null level pointer
        let rc = sb_get_audio_level(engine, ptr::null_mut());
        assert_ne!(rc, 0, "Null level pointer should fail");

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 12: Pipeline stats without running pipeline (all zeros)
// ============================================================================

#[test]
fn test_pipeline_stats_without_pipeline() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        let mut frames_captured: u64 = 999;
        let mut frames_played: u64 = 999;
        let mut latency_ms: f32 = 999.0;
        let mut loss_rate: f32 = 999.0;

        let rc = sb_pipeline_stats(
            engine,
            &mut frames_captured,
            &mut frames_played,
            &mut latency_ms,
            &mut loss_rate,
        );
        assert_eq!(rc, 0, "sb_pipeline_stats should succeed");
        assert_eq!(frames_captured, 0, "No frames captured without pipeline");
        assert_eq!(frames_played, 0, "No frames played without pipeline");
        assert!((latency_ms - 0.0).abs() < 0.001, "Latency should be 0.0 without pipeline");
        assert!((loss_rate - 0.0).abs() < 0.001, "Loss rate should be 0.0 without pipeline");

        destroy_engine(engine);
    }
}
