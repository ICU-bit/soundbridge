/**
 * SoundBridge FFI C Header
 *
 * C declarations matching the Rust sb_* functions in ffi-bindings/src/lib.rs.
 * This header is consumed by Windows C++ and Android JNI bridge code.
 *
 * Generated from ffi-bindings v0.8.0
 */

#ifndef SOUNDBRIDGE_H
#define SOUNDBRIDGE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================
 * Types & Enums
 * ============================================================ */

/** FFI error codes (matches Rust SbError) */
typedef enum SbError {
    SB_OK                =  0,
    SB_ERROR             = -1,
    SB_INVALID_ARGUMENT  = -2,
    SB_DEVICE_NOT_FOUND  = -3,
    SB_CONFIG_ERROR      = -4,
    SB_STREAM_ERROR      = -5,
    SB_CODEC_ERROR       = -6,
    SB_NETWORK_ERROR     = -7,
    SB_PIPELINE_NOT_READY= -8,
} SbError;

/** Connection state (matches Rust SbConnectionState) */
typedef enum SbConnectionState {
    SB_DISCONNECTED = 0,
    SB_CONNECTING   = 1,
    SB_CONNECTED    = 2,
    SB_ERROR_STATE  = 3,
} SbConnectionState;

/** Audio mode (matches Rust SbAudioMode) */
typedef enum SbAudioMode {
    SB_BALANCED     = 0,
    SB_HIGH_QUALITY = 1,
    SB_LOW_LATENCY  = 2,
} SbAudioMode;

/** State callback function type */
typedef void (*SbStateCallback)(SbConnectionState state, void* user_data);

/* ============================================================
 * Error handling
 * ============================================================ */

/** Get last error message (thread-local). Returns NULL if no error. */
const char* sb_last_error(void);

/* ============================================================
 * Engine lifecycle
 * ============================================================ */

/** Create engine. Returns opaque handle, or NULL on failure. */
void* sb_engine_create(void);

/** Destroy engine and release all resources. */
void sb_engine_destroy(void* engine);

/* ============================================================
 * State callback
 * ============================================================ */

/** Register state change callback. Pass NULL callback to unregister. */
int sb_set_state_callback(void* engine, SbStateCallback callback, void* user_data);

/** Get current connection state. */
int sb_get_connection_state(void* engine, SbConnectionState* state);

/* ============================================================
 * Network binding
 * ============================================================ */

/** Bind local UDP port. port=0 for auto-assign. */
int sb_bind(void* engine, uint16_t port);

/** Set target address ("ip:port" format). */
int sb_connect(void* engine, const char* addr);

/** Get bound local port. */
int sb_local_port(void* engine, uint16_t* port);

/* ============================================================
 * Audio capture
 * ============================================================ */

/** Start audio capture. device_name=NULL for default device. */
int sb_capture_start(void* engine, const char* device_name);

/** Stop audio capture. */
int sb_capture_stop(void* engine);

/** Read captured audio samples. Returns sample count or negative error. */
int sb_capture_read(void* engine, float* buf, size_t len);

/** Get capture device count. */
int sb_capture_device_count(size_t* count);

/* ============================================================
 * Audio playback
 * ============================================================ */

/** Start audio playback. device_name=NULL for default device. */
int sb_playback_start(void* engine, const char* device_name);

/** Stop audio playback. */
int sb_playback_stop(void* engine);

/** Write audio samples for playback. */
int sb_playback_write(void* engine, const float* buf, size_t len);

/** Get playback device count. */
int sb_playback_device_count(size_t* count);

/* ============================================================
 * Audio mixer
 * ============================================================ */

/** Mix multiple audio inputs with individual volumes. */
int sb_mixer_mix(void* engine,
                 const float** inputs, const size_t* input_lens,
                 const float* volumes, size_t input_count,
                 float* output, size_t output_len);

/* ============================================================
 * Audio processor (AEC/NS/AGC)
 * ============================================================ */

/** Process audio buffer in-place (AEC/NS/AGC). */
int sb_processor_process(void* engine, float* buf, size_t len);

/* ============================================================
 * Audio pipeline
 * ============================================================ */

/** Start audio pipeline (capture → encode → send + recv → decode → play). */
int sb_pipeline_start(void* engine);

/** Stop audio pipeline. */
int sb_pipeline_stop(void* engine);

/** Get pipeline state: 0=Stopped, 1=Running, 2=Error. */
int sb_pipeline_state(void* engine, int* state);

/** Get pipeline statistics. */
int sb_pipeline_stats(void* engine,
                      uint64_t* frames_captured, uint64_t* frames_played,
                      float* latency_ms, float* loss_rate);

/** Get audio level (RMS, 0.0-1.0). */
int sb_get_audio_level(void* engine, float* level);

/** Set WASAPI exclusive mode flag (affects latency calculation). */
int sb_set_exclusive_mode(void* engine, bool exclusive);

/* ============================================================
 * Audio mode
 * ============================================================ */

/** Set audio mode (Balanced/HighQuality/LowLatency). */
int sb_set_audio_mode(void* engine, SbAudioMode mode);

/** Get current audio mode. */
int sb_get_audio_mode(void* engine, SbAudioMode* mode);

/* ============================================================
 * Audio profile & EQ
 * ============================================================ */

/** Audio quality profile (matches Rust SbAudioProfile) */
typedef enum SbAudioProfile {
    SB_PROFILE_BANDWIDTH_SAVING = 0,
    SB_PROFILE_STANDARD         = 1,
    SB_PROFILE_HIGH_QUALITY     = 2,
    SB_PROFILE_LOSSLESS         = 3,
    SB_PROFILE_HIGH_RESOLUTION  = 4,
    SB_PROFILE_STUDIO_MASTER    = 5,
    SB_PROFILE_AUTO             = 6,
    SB_PROFILE_CUSTOM           = 7,
} SbAudioProfile;

/** EQ preset (matches Rust SbEqPreset) */
typedef enum SbEqPreset {
    SB_EQ_PRESET_FLAT    = 0,
    SB_EQ_PRESET_GAMING  = 1,
    SB_EQ_PRESET_MUSIC   = 2,
    SB_EQ_PRESET_VOICE   = 3,
    SB_EQ_PRESET_BASS    = 4,
    SB_EQ_PRESET_TREBLE  = 5,
} SbEqPreset;

/** Audio configuration (matches Rust SbAudioConfig) */
typedef struct SbAudioConfig {
    uint32_t sample_rate;
    uint32_t channels;
    uint32_t bitrate;
    uint32_t frame_size;
    uint32_t complexity;
} SbAudioConfig;

/** Set audio quality profile. Returns 0 on success. */
int sb_set_audio_profile(SbAudioProfile profile);

/** Get current audio quality profile. */
uint32_t sb_get_audio_profile(void);

/** Set channel count (1=Mono, 2=Stereo). Returns 0 on success. */
int sb_set_channels(uint32_t channels);

/** Get current channel count. */
uint32_t sb_get_channels(void);

/** Set single EQ band. band=0-9, gain_db=-12..+12, q=0.1..10. Returns 0 on success. */
int sb_set_eq_band(uint32_t band, float gain_db, float q);

/** Apply EQ preset. Returns 0 on success. */
int sb_set_eq_preset(SbEqPreset preset);

/** Enable/disable EQ (1=enabled, 0=disabled). Returns 0 on success. */
int sb_set_eq_enabled(int enabled);

/** Enable/disable auto profile mode (1=enabled, 0=disabled). Returns 0 on success. */
int sb_set_auto_profile_enabled(int enabled);

/* ============================================================
 * Mix ratio
 * ============================================================ */

/** Set mix ratio (PC volume, phone volume). Range 0.0-1.0. */
int sb_set_mix_ratio(void* engine, float pc_volume, float phone_volume);

/** Get mix ratio. */
int sb_get_mix_ratio(void* engine, float* pc_volume, float* phone_volume);

/* ============================================================
 * Connection type
 * ============================================================ */

/** Set connection type (0=WiFi, 1=WiFiDirect, 2=USB/ADB, 3=Bluetooth). */
int sb_set_connection_type(void* engine, int conn_type);

/** Get connection type. */
int sb_get_connection_type(void* engine, int* conn_type);

/* ============================================================
 * WiFi Direct hotspot
 * ============================================================ */

int sb_hotspot_create(void* engine, const char* ssid, const char* password, uint32_t channel);
int sb_hotspot_destroy(void* engine);
int sb_hotspot_state(void* engine, int32_t* state);
int sb_hotspot_set_state(void* engine, int32_t state);

/* ============================================================
 * USB/ADB connection
 * ============================================================ */

int sb_adb_setup_port_forward(void* engine, uint32_t local_port, uint32_t remote_port,
                              const char* device_serial);
int sb_adb_state(void* engine, int32_t* state);
int sb_adb_set_state(void* engine, int32_t state);

/* ============================================================
 * Bluetooth connection
 * ============================================================ */

int sb_bt_init(void* engine, const char* device_name, bool use_ble);
int sb_bt_state(void* engine, int32_t* state);
int sb_bt_set_state(void* engine, int32_t state);

/* ============================================================
 * Encryption (DTLS/SRTP)
 * ============================================================ */

/** Enable encryption. master_key=16 bytes, master_salt=14 bytes. */
int sb_enable_encryption(void* engine, const uint8_t* master_key, const uint8_t* master_salt);

/** Disable encryption. */
int sb_disable_encryption(void* engine);

/** Query encryption state: 1=enabled, 0=disabled, negative=error. */
int sb_is_encrypted(void* engine);

/* ============================================================
 * Bidirectional control
 * ============================================================ */

int sb_send_volume(void* engine, float volume);
int sb_send_pause(void* engine);
int sb_send_resume(void* engine);
int sb_set_mute(void* engine, int muted);
int sb_get_mute(void* engine);

/* ============================================================
 * Device store (JSON persistence)
 * ============================================================ */

void* sb_device_store_open(const char* path);
void  sb_device_store_close(void* store);
int   sb_device_store_add(void* store, const char* name, const char* address, uint16_t port);
int   sb_device_store_remove(void* store, const char* name);
int   sb_device_store_set_auto_connect(void* store, const char* name, bool auto_connect);
int   sb_device_store_count(void* store, size_t* count);
int   sb_device_store_has(void* store, const char* name);
void  sb_device_store_clear(void* store);
int   sb_device_store_get_address(void* store, const char* name, char* buf, size_t buf_len);
int   sb_device_store_get_port(void* store, const char* name, uint16_t* port);
int   sb_device_store_get_name_at(void* store, size_t index, char* buf, size_t buf_len);

/* ============================================================
 * Device discovery (mDNS)
 * ============================================================ */

void* sb_discovery_create(void);
void  sb_discovery_close(void* discovery);
int   sb_discovery_init(void* discovery);
int   sb_discovery_register(void* discovery, const char* name, uint16_t port);
int   sb_discovery_find_devices(void* discovery, void** devices_buf, size_t buf_size);
void  sb_discovery_free_device_info(void* device_info);

#ifdef __cplusplus
}
#endif

#endif /* SOUNDBRIDGE_H */
