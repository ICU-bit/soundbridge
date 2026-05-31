# SoundBridge — Agent Instructions

跨端音频融合：Windows (C++/C#) ↔ Android (Kotlin/JNI)，Rust 核心引擎。
游戏时不用摘耳机，同时听电脑和手机的声音。

## Project layout

- `rust-core/` — Rust workspace (10 crates). **All dev commands run here, not root.**
- `windows/` — C++20 core + C# WinUI 3 UI. CMake build.
- `android/` — Kotlin + Jetpack Compose + JNI C++. Gradle build.
- `scripts/` — PowerShell helper scripts (`test.ps1`, `bench.ps1`, `build-windows.ps1`, `release.ps1`, `verify-release.ps1`)
- `tools/` — `test-harness.ps1` (full check), `benchmark-runner.ps1`
- `docs/` — `design.md`, `development-plan.md`, `technical-spec.md`

Sub-platform guidance: `rust-core/AGENTS.md`, `windows/AGENTS.md`, `android/AGENTS.md`.
Each Rust crate has its own `AI_GUIDE.md` — **read it before modifying that crate**.

## Commands

All Rust commands require `workdir=rust-core` (workspace root is there, not repo root).

```bash
# Test (CI skips hardware-dependent tests)
cargo test --workspace
cargo test --workspace -- --skip test_capture_device --skip test_playback_device
cargo test -p audio-codec          # single crate

# Quality gates (order matters: fmt → clippy → test)
cargo fmt -- --check
cargo clippy --workspace -- -D warnings

# Benchmarks (audio-codec only, Criterion)
cargo bench -p audio-codec

# Windows C++ (needs vcpkg: opus, spdlog)
cmake -B build -S windows -DCMAKE_TOOLCHAIN_FILE=C:\vcpkg\scripts\buildsystems\vcpkg.cmake
cmake --build build --config Release

# Android
cd android && ./gradlew build
```

PowerShell shortcuts: `.\scripts\test.ps1 -Clippy -Fmt`, `.\scripts\bench.ps1`

## Hard-won conventions

These differ from language defaults — violating them will break things:

**Rust:**
- Dependencies are pinned in `[workspace.dependencies]` in `rust-core/Cargo.toml`. Individual crates use `foo.workspace = true`. Don't add versions in crate `Cargo.toml`.
- Error types: `audio-core` defines `AudioError` / `Result`. All crates use `audio_core::Result`. `audio-codec` has its own `CodecError`.
- Factory pattern: `_private: ()` field + `new() -> Result<Self>`. Prevents external construction.
- Test files: `tests/{name}_test.rs` (project convention, **not** Rust standard `#[cfg(test)]` in src).
- Formatting: `rustfmt.toml` at repo root (edition 2021, max_width=100, 4-space tabs).
- No `unwrap()` in library code — propagate with `Result`.
- `audio-core` is the foundation — changes here break all other crates.

**Windows C++:**
- C++20, MSVC `/permissive-` strict conformance, `/utf-8 /W4 /WX`
- Interface/impl split: pure virtual `IAudioEngine` in `include/soundbridge/`, `AudioEngineImpl final` in `src/`
- Factory functions return `std::unique_ptr`. Copy `= delete` on all impl classes.
- Include public headers as `<soundbridge/...>`, not `"soundbridge/..."`
- `NOMINMAX` is defined — use `<algorithm>` for `std::min`/`std::max`
- Packet magic: `0x53424447` ("SBDG") — packed struct, all platforms must match

**Android:**
- JNI handles: `jlong`, `0L` = invalid. All `native*` methods take `engineHandle: Long` as first param.
- Must call `nativeRelease` for every handle in `onDestroy` — memory leak otherwise.
- `NativeAudioEngine` is Kotlin `object` singleton. `System.loadLibrary` in `init` block only.
- State: `StateFlow` (not `LiveData`). `ConnectionState` enum: `DISCONNECTED`, `CONNECTING`, `CONNECTED`.
- Colors: semantic names from `Color.kt` (`AudioLevelLow`, `ConnectionConnected`, etc). Never raw `Color(0xFF...)`.
- Foreground service: `FOREGROUND_SERVICE_TYPE_MICROPHONE`, `START_STICKY`.

**Cross-platform audio params (hardcoded, don't change):**
- Sample rate: 48000 Hz, Mono, 960 samples/frame (20ms), Float32 internal format

## Network protocol

- Magic: `0x53424447` ("SBDG" = SoundBridge DataGram)
- Audio: UDP low-latency. Control: UDP (QUIC planned but not implemented).
- mDNS service type: `_soundbridge._udp`

## CI

- `.github/workflows/ci.yml`: Rust (fmt → clippy → test → build release), Windows C++ (CMake+vcpkg), Android (Gradle)
- CI runs on `windows-latest` for Rust/Windows, `ubuntu-latest` for Android
- Rust CI skips hardware tests: `--skip test_capture_device --skip test_playback_device`

## Anti-patterns

- Modifying `audio-core` types without considering downstream breakage
- Adding deps to individual crates without checking `[workspace.dependencies]`
- Calling JNI `native*` methods without checking handle `!= 0L`
- Using raw `Color` literals in Android Compose
- Constructing `AudioEngineImpl` directly (use factory)
- Accessing `capture_state_`/`render_state_` without `std::atomic`

## Git remotes

两个远程仓库，提交后都要推送，能推一个就行：
- `origin` → GitHub (`ICU-bit/soundbridge`)
- `gitee` → Gitee (`baigeijiuwanshile/soundbridge`)

推送命令：`git push origin master; git push gitee master`

## Commit style

Conventional Commits: `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:`

## Formatting

`.editorconfig`: UTF-8, CRLF line endings, 4-space indent (YAML 2-space), trailing whitespace trimmed (except .md).
`rustfmt.toml`: edition 2021, max_width 100, 4-space tabs, field/try init shorthand.
