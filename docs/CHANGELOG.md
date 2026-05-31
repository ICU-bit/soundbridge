# Changelog

All notable changes to SoundBridge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.10.0] - 2026-05-31

### Added
- PID bandwidth controller for adaptive bitrate (Kp=0.5, Ki=0.1, Kd=0.2)
- AudioProfile system with 8 quality tiers (Bandwidth Saving → Studio Master, up to 192kHz/Stereo)
- AutoProfileManager with network scoring (bandwidth 0.4 + latency 0.3 + loss 0.3)
- StereoMixer for mono-to-stereo upmix and stereo-to-mono downmix
- 10-band ParametricEq with 6 presets (Flat, Bass Boost, Vocal, Gaming, Cinema, Studio)
- 8 new FFI functions: sb_set_audio_profile, sb_get_audio_profile, sb_set_auto_profile, sb_set_eq_enabled, sb_set_eq_preset, sb_set_eq_band, sb_get_eq_preset, sb_get_eq_enabled
- Windows UI: AudioProfile ComboBox + Equalizer preset selector
- Android UI: AudioProfile section + Equalizer section with band sliders
- Android i18n: 30+ strings extracted to strings.xml (Chinese/English bilingual)
- Android English locale (values-en/strings.xml)
- CHANGELOG.md

### Changed
- Version bumped to 0.10.0
- development-plan.md updated with Phase 7 complete + Phase 8 progress

### Fixed
- C++ unused parameter warnings suppressed via #pragma in jni_bridge.cpp
- Android MissingPermission lint suppressed in BluetoothManager
- Android Compose API compatibility (LinearProgressIndicator)
- Windows bcrypt.h include order (windows.h before bcrypt.h)
- Android i18n: restored sp import, fixed guidePages reference
- Android i18n: use stringResource for GuidePage titleResId/descriptionResId

## [0.9.0] - 2026-05-30

### Added
- FEC (Forward Error Correction) encoder/decoder using Opus inband FEC
- panic_hook + tracing logging system
- ReconnectManager with exponential backoff (1s→30s, max 10 retries)
- Manual test guide (docs/manual-test-guide.md)
- Network latency measurement script (tools/latency-measure.ps1)
- FeedbackManager for user feedback collection
- FirstRunGuideScreen (4-step onboarding wizard)
- Android UncaughtExceptionHandler with user-friendly crash dialog
- Android auto-reconnect with exponential backoff
- Android reconnect UI (progress indicator + cancel button)

## [0.8.0] - 2026-05-29

### Added
- DTLS/SRTP encryption (AES-128-CM + HMAC-SHA1-80, end-to-end)
- QUIC control signaling (quinn 0.10, session management/audio negotiation/device discovery)
- Session handshake protocol (state machine, capability negotiation, heartbeat, graceful disconnect)
- Audio mode dynamic switching (Balanced/Low Latency/High Quality)
- RawJitterBuffer for out-of-order packet tolerance
- WASAPI exclusive mode support (10ms buffer, auto-fallback)
- Zero-allocation hot path for audio pipeline
- ECDH key exchange (x25519-dalek)
- Device memory with JSON persistence
- Global hotkeys (Ctrl+Alt+P/M/S)
- Mute flag as Arc<AtomicBool>
- Auto-start on Windows (Registry HKCU\...\Run)
- Hotspot mode (WiFi Direct)
- ADB connection (USB port forwarding)
- Bluetooth connection (RFCOMM↔UDP bridge)

## [0.7.0] - 2026-05-28

### Added
- ProgressRing loading animation for connection states
- Device discovery UI (scan button + device list + ListView)
- 5 new Converters (InvertBool, InvertBoolToVisibility, BoolToScanText, CollectionToVisibility, EmptyCollectionToVisibility)
- Connection button three states (Connecting.../Connect/Disconnect)
- 11 new JNI functions (hotspot/ADB/bluetooth/exclusive mode)
- jni_bridge.cpp connection management stubs

## [0.6.0] - 2026-05-27

### Added
- sb_get_audio_level (real RMS audio level from capture data)
- sb_set_exclusive_mode (WASAPI exclusive mode with latency formula)
- sb_hotspot_create/destroy/state (WiFi Direct hotspot management)
- sb_adb_setup_port_forward/state/set_state (USB/ADB port forwarding)
- sb_bt_init/state/set_state (Bluetooth connection management)
- AudioProfile system with network requirements display
- Auto profile mode with network scoring

### Fixed
- sb_playback_write channels 2→1 (Oracle Bug 1)
- ConnectionType FFI dead code - sb_set_connection_type/sb_get_connection_type (Oracle Bug 2)
- sb_set_audio_mode hot-restart - pipeline auto-restart on mode change (Oracle Bug 3)
- Device discovery JSON escaping for special characters

## [0.5.0] - 2026-05-26

### Added
- RawJitterBuffer for raw Opus byte storage with out-of-order tolerance
- WASAPI exclusive mode support (10ms buffer, auto-fallback to shared)
- AudioModeManager for codec parameter switching
- AudioMixer integration in receive pipeline
- sb_set_mix_ratio / sb_get_mix_ratio for PC/phone volume balance

## [0.4.0] - 2026-05-25

### Added
- FFI bindings: audio mode switching, connection state callback, bidirectional control
- Windows: MainWindow + ViewModel + TrayIcon + HotkeyManager + device persistence + auto-start
- Android: SettingsScreen audio mode dropdown + JNI audio mode switching

## [0.3.0] - 2026-05-24

### Added
- UDP audio transport with low latency
- Opus encode/decode pipeline
- Basic audio mixing
- Audio level indicator
- mDNS device discovery (_soundbridge._udp)

## [0.2.0] - 2026-05-23

### Added
- Rust core workspace (10 crates, 181+ tests)
- Audio capture/playback via cpal
- Opus codec integration
- Network transport layer

## [0.1.0] - 2026-05-22

### Added
- Initial design document
- Project structure
- Basic architecture
