# Rust FFI Migration Guide for Android

This document explains how to build the SoundBridge Android app with the Rust FFI static library instead of the local C++ AudioEngine.

## Overview

The Android JNI bridge (`jni_bridge.cpp`) supports two build modes:

| Mode | Flag | Engine | Status |
|------|------|--------|--------|
| **Legacy** (default) | `SOUNDBRIDGE_USE_RUST_FFI=OFF` | Local C++ `AudioEngine` | Current production |
| **Rust FFI** | `SOUNDBRIDGE_USE_RUST_FFI=ON` | Rust `ffi-bindings` crate | Migration target |

When Rust FFI is enabled, the following JNI functions delegate to Rust `sb_*` functions:
- `nativeInit` → `sb_engine_create()` + `sb_bind(0)` + `sb_local_port()`
- `nativeRelease` → `sb_engine_destroy()`
- `nativeBind` → `sb_bind()`
- `nativeConnect` → `sb_connect()`
- `nativeGetLocalPort` → `sb_local_port()`
- `nativePipelineStart` → `sb_pipeline_start()`
- `nativePipelineStop` → `sb_pipeline_stop()`
- `nativePipelineState` → `sb_pipeline_state()`
- `nativeSetEncryptionEnabled` → `sb_enable_encryption()` / `sb_disable_encryption()`
- `nativeIsEncryptionEnabled` → `sb_is_encrypted()`

Stub functions (hotspot/ADB/Bluetooth/AEC) remain unchanged.

## Prerequisites

1. **Rust toolchain** with Android targets:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   ```

2. **cargo-ndk** (Rust cross-compilation for Android):
   ```bash
   cargo install cargo-ndk
   ```

3. **Android NDK** (r25+ recommended):
   - Install via Android Studio SDK Manager, or download directly
   - Set `ANDROID_NDK_HOME` environment variable

4. **Android SDK** (API level 26+):
   - Set `ANDROID_HOME` environment variable

## Building the Rust Static Library

### Step 1: Build for each target architecture

```bash
# From the workspace root (跨端音媒体融合/)
cd rust-core

# Build for arm64-v8a (most modern devices)
cargo ndk -t arm64-v8a --manifest-path Cargo.toml -p ffi-bindings --release

# Build for armeabi-v7a (older 32-bit devices)
cargo ndk -t armeabi-v7a --manifest-path Cargo.toml -p ffi-bindings --release

# Build for x86_64 (emulator)
cargo ndk -t x86_64 --manifest-path Cargo.toml -p ffi-bindings --release
```

### Step 2: Verify the static libraries

After building, static libraries are at:
```
rust-core/target/aarch64-linux-android/release/libsoundbridge_bindings.a
rust-core/target/armv7-linux-androideabi/release/libsoundbridge_bindings.a
rust-core/target/x86_64-linux-android/release/libsoundbridge_bindings.a
```

## Building the Android App with Rust FFI

### Option A: Gradle with CMake arguments

In `android/app/build.gradle.kts`, add a CMake argument:

```kotlin
android {
    defaultConfig {
        externalNativeBuild {
            cmake {
                arguments(
                    "-DSOUNDBRIDGE_USE_RUST_FFI=ON",
                    "-DSOUNDBRIDGE_RUST_FFI_LIB_DIR=${rootProject.projectDir}/../rust-core/target/aarch64-linux-android/release",
                    "-DSOUNDBRIDGE_RUST_FFI_INCLUDE_DIR=${rootProject.projectDir}/../rust-core/crates/ffi-bindings/include"
                )
            }
        }
    }
}
```

### Option B: Direct CMake build

```bash
cd android/app/src/main/cpp

cmake -B build \
    -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake \
    -DANDROID_ABI=arm64-v8a \
    -DANDROID_PLATFORM=android-26 \
    -DSOUNDBRIDGE_USE_RUST_FFI=ON \
    -DSOUNDBRIDGE_RUST_FFI_LIB_DIR=../../../../rust-core/target/aarch64-linux-android/release \
    -DSOUNDBRIDGE_RUST_FFI_INCLUDE_DIR=../../../../rust-core/crates/ffi-bindings/include

cmake --build build
```

### Option C: Environment variable

```bash
export SOUNDBRIDGE_USE_RUST_FFI=ON
export SOUNDBRIDGE_RUST_FFI_LIB_DIR=/path/to/rust-core/target/aarch64-linux-android/release
export SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR=/path/to/rust-core/crates/ffi-bindings/include
```

## CMake Options Reference

| Option | Default | Description |
|--------|---------|-------------|
| `SOUNDBRIDGE_USE_RUST_FFI` | `OFF` | Enable Rust FFI integration |
| `SOUNDBRIDGE_RUST_FFI_LIB_DIR` | Auto-detected | Directory containing `libsoundbridge_bindings.a` |
| `SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR` | Auto-detected | Directory containing `soundbridge.h` |

## Architecture Mapping

| ABI | Rust Target | NDK Toolchain |
|-----|-------------|---------------|
| `arm64-v8a` | `aarch64-linux-android` | `aarch64-linux-android26-clang` |
| `armeabi-v7a` | `armv7-linux-androideabi` | `armv7a-linux-androideabi26-clang` |
| `x86_64` | `x86_64-linux-android` | `x86_64-linux-android26-clang` |

## Verification

### Without Rust FFI (default build)
```bash
cd android
./gradlew assembleDebug
# Should succeed with local C++ AudioEngine
```

### With Rust FFI
```bash
# 1. Build Rust static library
cd rust-core && cargo ndk -t arm64-v8a -p ffi-bindings --release && cd ..

# 2. Build Android app
cd android
./gradlew assembleDebug \
    -PcmakeArgs="-DSOUNDBRIDGE_USE_RUST_FFI=ON"
```

## Troubleshooting

### "libsoundbridge_bindings.a not found"
- Verify the Rust build completed: `ls rust-core/target/aarch64-linux-android/release/libsoundbridge_bindings.a`
- Check `SOUNDBRIDGE_RUST_FFI_LIB_DIR` points to the correct directory

### "soundbridge.h not found"
- The header is at `rust-core/crates/ffi-bindings/include/soundbridge.h`
- Check `SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR` points to the `include/` directory

### Linker errors (undefined symbols)
- Ensure you're linking `dl` and `m` libraries (CMakeLists.txt handles this)
- Verify the Rust target matches the Android ABI

### "cargo ndk" not found
- Install it: `cargo install cargo-ndk`
- Ensure `~/.cargo/bin` is in your `PATH`

## Migration Status

- [x] C header (`soundbridge.h`) — all 65 `sb_*` functions declared
- [x] CMakeLists.txt — conditional Rust FFI linking
- [x] jni_bridge.cpp — `#ifdef SOUNDBRIDGE_USE_RUST_FFI` guards
- [ ] Rust static library built for Android (requires NDK)
- [ ] End-to-end test on Android device
- [ ] NativeAudioEngine.kt signatures updated (not needed — JNI layer handles translation)
