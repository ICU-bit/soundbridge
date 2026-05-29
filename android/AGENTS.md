# ANDROID MODULE

SoundBridge Android app. Kotlin + Jetpack Compose UI, C++ native layer via JNI.

## STRUCTURE

```
app/src/main/java/com/soundbridge/
├── MainActivity.kt          # Entry point, permission handling
├── SoundBridgeApp.kt        # Application class, notification channel
├── native/
│   └── NativeAudioEngine.kt # JNI bridge (object singleton)
├── audio/
│   ├── AudioService.kt      # Foreground service, engine lifecycle, StateFlow
│   ├── AudioCaptureManager.kt
│   └── AudioPlaybackManager.kt
└── ui/
    ├── HomeScreen.kt        # 4 sub-Composables: ConnectionStatusCard, AudioLevelVisualizer, ControlButtons, ServerConfigSection
    ├── SettingsScreen.kt
    ├── SoundBridgeApp.kt    # NavHost
    └── theme/
        ├── Color.kt         # Semantic: AudioLevel*, Connection*, SoundBridge*
        ├── Theme.kt         # Dark priority, dynamic color (Android 12+)
        └── Type.kt

app/src/main/cpp/
├── jni_bridge.cpp           # JNI implementation
├── audio_engine.cpp / include/audio_engine.h
├── opus_codec.cpp / include/opus_codec.h
├── audio_processor.cpp      # AEC/NS/AGC
├── udp_socket.cpp
└── CMakeLists.txt           # C++17, c++_shared STL
```

## WHERE TO LOOK

| Task | File |
|------|------|
| JNI API / native calls | `native/NativeAudioEngine.kt` |
| Engine lifecycle + state | `audio/AudioService.kt` |
| Main UI | `ui/HomeScreen.kt` |
| Navigation | `ui/SoundBridgeApp.kt` |
| Colors / Theme | `ui/theme/Color.kt`, `ui/theme/Theme.kt` |
| Permissions | `MainActivity.kt` |
| NDK build | `app/src/main/cpp/CMakeLists.txt` |

## CONVENTIONS

### JNI Handle Pattern
- Handle type: `jlong`. `0L` = invalid.
- All `native*` methods: first param is `engineHandle: Long`.
- Encoder/decoder use separate handles (`nativeCreateEncoder` / `nativeCreateDecoder`).
- Must call `nativeRelease` (or `nativeReleaseEncoder`/`nativeReleaseDecoder`) on destroy.
- `NativeAudioEngine` is Kotlin `object`. Library loaded in `init` block.

### State Management
- `StateFlow` for reactive state: `_connectionState = MutableStateFlow(...)`, exposed as `val connectionState: StateFlow<...>`.
- `ConnectionState` enum: `DISCONNECTED`, `CONNECTING`, `CONNECTED`.

### UI / Theme
- Material3, dark theme default. Dynamic color on Android 12+.
- Semantic colors in `Color.kt`:
  - `AudioLevelLow` (green), `AudioLevelMedium` (yellow), `AudioLevelHigh` (red)
  - `ConnectionConnected` (green), `ConnectionConnecting` (yellow), `ConnectionDisconnected` (red)
  - Brand: `SoundBridgePrimary`, `SoundBridgeSecondary`, etc.
- Never use raw `Color(0xFF...)`. Always use semantic names.

### Foreground Service
- `AudioService` extends `Service` with `Binder` pattern.
- Actions: `ACTION_START`, `ACTION_STOP` via companion constants.
- `FOREGROUND_SERVICE_TYPE_MICROPHONE` on Android Q+.
- `START_STICKY` for restart on kill.

## ANTI-PATTERNS

- Don't call `native*` without valid handle (`!= 0L`).
- Don't skip `nativeRelease` in `onDestroy`. Every handle must be released.
- Don't use raw `Color` literals. Use semantic names from `Color.kt`.
- Don't use `remember` for state that must survive config changes. Use `StateFlow` in service/viewmodel.
- Don't call `System.loadLibrary` multiple times.
- Don't add network calls in UI layer. Use `AudioService` or dedicated manager.
