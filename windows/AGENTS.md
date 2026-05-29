# WINDOWS MODULE

Windows native C++ core engine + C# WinUI 3 UI for SoundBridge.

## STRUCTURE

```
windows/
├── include/soundbridge/    # Public API headers (IAudioEngine, ISession, types, export macros)
├── src/
│   ├── audio/              # WASAPI capture/renderer, Opus codec, WebRTC APM wrappers
│   ├── core/               # AudioEngineImpl, AudioPipeline, Session implementations
│   ├── network/            # UDP/QUIC transports, PacketBuilder (magic: 0x53424447)
│   └── ui/                 # WinUI 3 C# app (SoundBridge.csproj, App.xaml.cs)
├── cmake/                  # FindOpus.cmake, FindWebRTC.cmake
└── tests/                  # GTest test .cpp files (NOT yet created)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Public API interfaces | `include/soundbridge/audio_engine.h` | `IAudioEngine`, `ISession` pure virtual classes |
| Type definitions | `include/soundbridge/types.h` | `AudioFormat`, `SessionConfig`, `AudioStreamState` enum |
| DLL export macros | `include/soundbridge/export.h` | `SOUNDBRIDGE_API`, `SOUNDBRIDGE_CALL` |
| Engine implementation | `src/core/audio_engine.h` | `AudioEngineImpl final : public IAudioEngine` |
| Audio pipeline | `src/core/audio_pipeline.h` | Capture → APM → Encode → Send chain |
| Session management | `src/core/session.h` | Connection lifecycle, state machine |
| Audio types (internal) | `src/core/audio_types.h` | `AudioRingBuffer` and internal types |
| WASAPI capture | `src/audio/wasapi_capture.h` | Windows audio device capture |
| Opus encoding | `src/audio/opus_codec.h` | Opus encoder/decoder wrapper |
| WebRTC APM | `src/audio/webrtc_apm.h` | AEC, NS, AGC processing |
| Packet format | `src/network/packet.h` | `PacketHeader`, `PacketBuilder`, checksum |
| UDP transport | `src/network/udp_transport.h` | Low-latency audio streaming |
| QUIC transport | `src/network/quic_transport.h` | Reliable control signaling |
| Transport interface | `src/network/transport_interface.h` | Abstract transport base |
| WinUI 3 app | `src/ui/App.xaml.cs` | C# entry point, DI container setup |
| Build config | `CMakeLists.txt` | C++20, MSVC /utf-8 /W4 /WX |
| Test config | `tests/CMakeLists.txt` | GTest, expects `test_*.cpp` files |

## CONVENTIONS

- C++20 standard, MSVC with `/permissive-` (strict conformance mode)
- Interface/impl separation: pure virtual `IAudioEngine` in public headers, `AudioEngineImpl final` in src/
- Factory functions: `create_audio_engine()` and `create_session()` return `std::unique_ptr`
- Copy deleted on all impl classes: `= delete` on copy ctor and copy assignment
- State machine: `AudioStreamState` enum (Idle → Starting → Running → Pausing → Paused → Stopping → Stopped, or Error)
- Atomic state variables: `std::atomic<AudioStreamState>` for thread-safe state queries
- Callbacks are `std::function` types: `AudioFrameCallback`, `StateChangeCallback`, `ErrorCallback`
- Packet magic: `0x53424447` ("SBDG") in `PacketHeader`, validated on parse
- DLL export: define `SOUNDBRIDGE_EXPORTS` when building the library, consumers get `dllimport`

## ANTI-PATTERNS

- Do NOT include `soundbridge/` headers in implementation files; use `<soundbridge/...>` include path
- Do NOT construct `AudioEngineImpl` directly; always use `create_audio_engine()` factory
- Do NOT use raw pointers for engine/session ownership; always `std::unique_ptr`
- Do NOT access `capture_state_`/`render_state_` without `std::atomic` operations
- Do NOT modify `PacketHeader` layout without updating all platform implementations (packed struct)
- Do NOT skip `tests/` directory; test .cpp files are declared in CMakeLists.txt but not yet created
- Do NOT use `std::min`/`std::max` directly; `NOMINMAX` is defined, use `<algorithm>`
