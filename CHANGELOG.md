# Changelog

All notable changes to SoundBridge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.1] - 2026-05-30

### Added
- GitHub Actions CI/CD: Rust tests (clippy+fmt+test) + Windows C++ build + Android build
- .editorconfig: UTF-8, CRLF, consistent indentation across all languages
- rustfmt.toml: edition 2021, max_width 100, field_init_shorthand
- scripts/bench.ps1: benchmark runner utility
- scripts/test.ps1: test runner with clippy/fmt checks
- CHANGELOG.md: Keep a Changelog format
- CONTRIBUTING.md: development environment, code standards, commit conventions
- .github/ISSUE_TEMPLATE/bug_report.md: structured bug report
- .github/ISSUE_TEMPLATE/feature_request.md: feature request template
- .github/pull_request_template.md: PR checklist

### Fixed
- opus_benchmark.rs: add missing rand dev-dependency
- OpusConfig: derive Copy (all fields are Copy enums)
- Remove unnecessary config.clone() calls (audio-codec + ffi-bindings)
- Fix clippy "for loop over a single element" warning
- Workspace-wide cargo fmt formatting cleanup (30 files)

## [0.7.0] - 2026-05-30

### Added
- Windows UI: ProgressRing loading animation on connect button
- Windows UI: Device discovery section with scan button and device list
- Windows UI: 5 new Converters (InvertBool, InvertBoolToVisibility, BoolToScanText, CollectionToVisibility, EmptyCollectionToVisibility)
- Windows UI: Connect button three-state (Connecting... / Connect / Disconnect)
- Android JNI: 11 new connection management functions (hotspot/ADB/BT/exclusive mode)
- Android JNI: jni_bridge.cpp connection management stubs (static state tracking)
- GitHub Actions CI/CD (Rust tests + Windows C++ build + Android build)
- .editorconfig for consistent formatting across editors
- rustfmt.toml for Rust formatting standards
- scripts/bench.ps1: benchmark runner utility
- scripts/test.ps1: test runner with clippy/fmt checks

### Fixed
- opus_benchmark.rs: add missing rand dev-dependency
- OpusConfig: derive Copy (all fields are Copy enums)
- Remove unnecessary config.clone() calls (audio-codec + ffi-bindings)
- Fix clippy "for loop over a single element" warning

## [0.6.0] - 2026-05-30

### Added
- FFI: sb_get_audio_level (real RMS audio level from capture data)
- FFI: sb_set_exclusive_mode (WASAPI exclusive mode latency formula)
- FFI: sb_hotspot_create / sb_hotspot_destroy / sb_hotspot_state (WiFi Direct hotspot)
- FFI: sb_adb_setup_port_forward / sb_adb_state / sb_adb_set_state (USB/ADB port forwarding)
- FFI: sb_bt_init / sb_bt_state / sb_bt_set_state (Bluetooth connection)
- FFI: SharedPipelineStats captured_level_bits + exclusive_mode fields
- Bandwidth adaptation: sender thread adjusts Opus bitrate (64/96/128kbps) based on loss rate
- Windows P/Invoke: sb_get_audio_level / sb_set_exclusive_mode
- Windows UI: real sb_get_audio_level replaces fake data
- Windows UI: ConnectionType selector (ComboBox) + audio mode hot-switch
- Windows: 3 GTest test files (27 tests: Opus codec + audio pipeline + UDP transport)
- Android UI: ConnectionType selector (FilterChip) + audio mode hot-switch
- Network: HotspotConfig / HotspotState / AdbConfig / AdbState / BluetoothConfig / BluetoothState

### Fixed
- FFI: sb_playback_write channels 2→1 (Oracle Bug 1)
- FFI: ConnectionType FFI dead code - sb_set_connection_type / sb_get_connection_type (Oracle Bug 2)
- FFI: sb_set_audio_mode hot-switch - pipeline restart when running (Oracle Bug 3)

## [0.5.0] - 2026-05-29

### Added
- FFI: receiver thread integrates RawJitterBuffer (Opus packets decoded in order)
- Windows: WasapiCapture/WasapiRenderer exclusive mode (10ms buffer, auto-fallback to shared 50ms)
- Windows: IAudioEngine.initialize() exclusive parameter
- Network: RawJitterBuffer (stores raw Opus bytes, 8 tests)

### Fixed
- JNI: nativeConnect uses strtol instead of stoi to prevent crash
- FFI: device discovery JSON escapes special characters

## [0.4.0] - 2026-05-29

### Added
- FFI: sb_set_audio_mode via AudioModeManager
- FFI: receiver thread integrates AudioMixer (local capture + remote decode mixed before playback)
- FFI: sb_set_mix_ratio / sb_get_mix_ratio (PC/phone volume balance)
- Android JNI: nativeGetAudioMode / nativeSetMixRatio / nativeGetMixRatio
- Windows P/Invoke: sb_set_mix_ratio / sb_get_mix_ratio

## [0.3.0] - 2026-05-28

### Added
- FFI: audio mode switching, connection state callbacks, bidirectional control
- Windows: MainWindow + ViewModel + TrayIcon + HotkeyManager + device persistence + auto-start
- Android: SettingsScreen audio mode dropdown + JNI audio mode switching

## [0.2.0] - 2026-05-27

### Added
- Rust core MVP: 10 crates, 181+ tests
- All crates fully implemented (not skeletons)

## [0.1.0] - 2026-05-26

### Added
- Initial design documents
