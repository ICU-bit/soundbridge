//! FFI unit tests for SoundBridge C API.
//!
//! Tests the non-hardware-dependent C API functions: engine lifecycle,
//! error handling, null pointer safety, configuration round-trips, and
//! boundary values. Every test is fully independent: create → exercise → destroy.

use ffi_bindings::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::ptr;

// ============================================================================
// Helpers
// ============================================================================

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

fn destroy_engine(engine: *mut c_void) {
    unsafe { sb_engine_destroy(engine) }
}

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
// Test 1: Engine create/destroy lifecycle
// ============================================================================

#[test]
fn test_engine_create_destroy_lifecycle() {
    unsafe {
        let engine = sb_engine_create();
        assert!(!engine.is_null(), "sb_engine_create should return non-null");

        // No error should be set after successful creation
        let err = sb_last_error();
        assert!(err.is_null(), "No error after successful create");

        // Destroy should not panic
        sb_engine_destroy(engine);
    }
}

// ============================================================================
// Test 2: sb_last_error behavior
// ============================================================================

#[test]
fn test_last_error_behavior() {
    unsafe {
        // After successful creation, last_error should be null
        let engine = match create_engine() {
            Some(e) => e,
            None => return,
        };
        let err = sb_last_error();
        assert!(err.is_null(), "No error after successful engine creation");

        // Trigger an error: null engine to sb_set_mute
        let rc = sb_set_mute(ptr::null_mut(), 1);
        assert_eq!(rc, SbError::InvalidArgument as c_int);
        let msg = last_error_string();
        assert!(
            !msg.is_empty(),
            "Error message should be set after null engine call"
        );

        // After a successful call, error should be cleared
        let rc = sb_set_mute(engine, 0);
        assert_eq!(rc, 0, "Valid call should succeed");
        let err = sb_last_error();
        assert!(
            err.is_null(),
            "Error should be cleared after successful call"
        );

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 3: Null pointer handling across functions
// ============================================================================

#[test]
fn test_null_pointer_handling() {
    unsafe {
        // Note: sb_engine_destroy(null) is NOT tested — the underlying C impl
        // dereferences the pointer directly, causing UB. This is intentional:
        // callers must not pass null to destroy.

        // sb_set_mute(null, ...) should return InvalidArgument
        let rc = sb_set_mute(ptr::null_mut(), 0);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_mute(null) should return InvalidArgument
        let rc = sb_get_mute(ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_set_state_callback with null engine
        let rc = sb_set_state_callback(ptr::null_mut(), None, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_connection_state with null engine
        let mut state = SbConnectionState::Disconnected;
        let rc = sb_get_connection_state(ptr::null_mut(), &mut state);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_connection_state with null output pointer
        let engine = match create_engine() {
            Some(e) => e,
            None => return,
        };
        let rc = sb_get_connection_state(engine, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_bind with null engine
        let rc = sb_bind(ptr::null_mut(), 0);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_connect with null engine
        let addr = CString::new("127.0.0.1:12345").unwrap();
        let rc = sb_connect(ptr::null_mut(), addr.as_ptr());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_connect with null address
        let rc = sb_connect(engine, ptr::null());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_local_port with null engine
        let mut port: u16 = 0;
        let rc = sb_local_port(ptr::null_mut(), &mut port);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_local_port with null output
        let rc = sb_local_port(engine, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_set_audio_mode with null engine
        let rc = sb_set_audio_mode(ptr::null_mut(), SbAudioMode::Balanced);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_audio_mode with null engine
        let mut mode = SbAudioMode::Balanced;
        let rc = sb_get_audio_mode(ptr::null_mut(), &mut mode);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_audio_mode with null output
        let rc = sb_get_audio_mode(engine, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_set_connection_type with null engine
        let rc = sb_set_connection_type(ptr::null_mut(), 0);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_connection_type with null engine
        let mut conn_type: i32 = -1;
        let rc = sb_get_connection_type(ptr::null_mut(), &mut conn_type);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_connection_type with null output
        let rc = sb_get_connection_type(engine, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_set_mix_ratio with null engine
        let rc = sb_set_mix_ratio(ptr::null_mut(), 0.5, 0.5);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_mix_ratio with null engine
        let mut pc: f32 = 0.0;
        let mut phone: f32 = 0.0;
        let rc = sb_get_mix_ratio(ptr::null_mut(), &mut pc, &mut phone);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_mix_ratio with null pc pointer
        let rc = sb_get_mix_ratio(engine, ptr::null_mut(), &mut phone);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_get_mix_ratio with null phone pointer
        let rc = sb_get_mix_ratio(engine, &mut pc, ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_pipeline_state with null engine
        let mut pstate: c_int = -1;
        let rc = sb_pipeline_state(ptr::null_mut(), &mut pstate);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_pipeline_start with null engine
        let rc = sb_pipeline_start(ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_pipeline_stop with null engine
        let rc = sb_pipeline_stop(ptr::null_mut());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_processor_process with null engine
        let mut buf = [0.0f32; 960];
        let rc = sb_processor_process(ptr::null_mut(), buf.as_mut_ptr(), buf.len());
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_processor_process with null buffer
        let rc = sb_processor_process(engine, ptr::null_mut(), 960);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // sb_set_exclusive_mode with null engine
        let rc = sb_set_exclusive_mode(ptr::null_mut(), false);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 4: Mix ratio boundary values
// ============================================================================

#[test]
fn test_mix_ratio_boundary_values() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Default should be 0.5 / 0.5
        let mut pc: f32 = -1.0;
        let mut phone: f32 = -1.0;
        let rc = sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert_eq!(rc, 0);
        assert!((pc - 0.5).abs() < 0.01, "Default PC ~0.5, got {}", pc);
        assert!(
            (phone - 0.5).abs() < 0.01,
            "Default phone ~0.5, got {}",
            phone
        );

        // Both silent
        let rc = sb_set_mix_ratio(engine, 0.0, 0.0);
        assert_eq!(rc, 0);
        sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert!((pc - 0.0).abs() < 0.01);
        assert!((phone - 0.0).abs() < 0.01);

        // Both max
        let rc = sb_set_mix_ratio(engine, 1.0, 1.0);
        assert_eq!(rc, 0);
        sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert!((pc - 1.0).abs() < 0.01);
        assert!((phone - 1.0).abs() < 0.01);

        // PC silent, phone max
        let rc = sb_set_mix_ratio(engine, 0.0, 1.0);
        assert_eq!(rc, 0);
        sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert!((pc - 0.0).abs() < 0.01);
        assert!((phone - 1.0).abs() < 0.01);

        // PC max, phone silent
        let rc = sb_set_mix_ratio(engine, 1.0, 0.0);
        assert_eq!(rc, 0);
        sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert!((pc - 1.0).abs() < 0.01);
        assert!((phone - 0.0).abs() < 0.01);

        // Asymmetric
        let rc = sb_set_mix_ratio(engine, 0.75, 0.25);
        assert_eq!(rc, 0);
        sb_get_mix_ratio(engine, &mut pc, &mut phone);
        assert!((pc - 0.75).abs() < 0.01);
        assert!((phone - 0.25).abs() < 0.01);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 5: Audio mode round-trip for all 3 modes
// ============================================================================

#[test]
fn test_audio_mode_roundtrip() {
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
            assert_eq!(rc, 0);
            assert_eq!(got, mode, "Mode round-trip failed for {:?}", mode);
        }

        // Rapid cycling
        for _ in 0..10 {
            for &mode in &modes {
                sb_set_audio_mode(engine, mode);
            }
        }
        let mut final_mode = SbAudioMode::Balanced;
        sb_get_audio_mode(engine, &mut final_mode);
        assert_eq!(final_mode, SbAudioMode::LowLatency);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 6: Connection type round-trip for all 4 types + invalid
// ============================================================================

#[test]
fn test_connection_type_roundtrip() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

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
                "set_connection_type({}) should succeed for {}",
                conn_type, name
            );

            let mut got: i32 = -1;
            let rc = sb_get_connection_type(engine, &mut got);
            assert_eq!(rc, 0);
            assert_eq!(
                got, conn_type,
                "Connection type round-trip failed for {}",
                name
            );
        }

        // Invalid type should fail
        let rc = sb_set_connection_type(engine, 99);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        // Invalid negative type should fail
        let rc = sb_set_connection_type(engine, -1);
        assert_eq!(rc, SbError::InvalidArgument as c_int);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 7: Mute toggle round-trip
// ============================================================================

#[test]
fn test_mute_toggle_roundtrip() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Default unmuted
        assert_eq!(sb_get_mute(engine), 0, "Should start unmuted");

        // Mute on
        let rc = sb_set_mute(engine, 1);
        assert_eq!(rc, 0);
        assert_eq!(sb_get_mute(engine), 1);

        // Mute off
        let rc = sb_set_mute(engine, 0);
        assert_eq!(rc, 0);
        assert_eq!(sb_get_mute(engine), 0);

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
// Test 8: Pipeline state without hardware
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
        assert_eq!(state, 0, "Pipeline should start as Stopped");

        // Pipeline start without capture should fail
        let rc = sb_pipeline_start(engine);
        assert_eq!(rc, SbError::PipelineNotReady as c_int);

        // State should still be Stopped after failed start
        sb_pipeline_state(engine, &mut state);
        assert_eq!(
            state, 0,
            "Pipeline should remain Stopped after failed start"
        );

        // Audio level without pipeline
        let mut level: f32 = -1.0;
        let rc = sb_get_audio_level(engine, &mut level);
        assert_eq!(rc, 0);
        assert!(
            (level - 0.0).abs() < 0.001,
            "Level without pipeline ~0.0, got {}",
            level
        );

        // Pipeline stop on stopped engine is a no-op
        let rc = sb_pipeline_stop(engine);
        assert_eq!(rc, 0);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 9: Connection state lifecycle
// ============================================================================

#[test]
fn test_connection_state_lifecycle() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        // Initial state should be Disconnected
        let mut state = SbConnectionState::Disconnected;
        let rc = sb_get_connection_state(engine, &mut state);
        assert_eq!(rc, 0);
        assert_eq!(state, SbConnectionState::Disconnected);

        // After sb_bind, state should still be Disconnected
        let rc = sb_bind(engine, 0);
        assert_eq!(rc, 0);
        sb_get_connection_state(engine, &mut state);
        assert_eq!(state, SbConnectionState::Disconnected);

        // After sb_connect, state should be Connecting
        let addr = CString::new("127.0.0.1:54321").unwrap();
        let rc = sb_connect(engine, addr.as_ptr());
        assert_eq!(rc, 0);
        sb_get_connection_state(engine, &mut state);
        assert_eq!(state, SbConnectionState::Connecting);

        destroy_engine(engine);
    }
}

// ============================================================================
// Test 10: Processor process without null args
// ============================================================================

#[test]
fn test_processor_process_valid() {
    let engine = match create_engine() {
        Some(e) => e,
        None => return,
    };

    unsafe {
        let mut buf = [0.0f32; 960];
        let rc = sb_processor_process(engine, buf.as_mut_ptr(), buf.len());
        assert_eq!(
            rc,
            0,
            "sb_processor_process should succeed, got: {}",
            last_error_string()
        );

        destroy_engine(engine);
    }
}
