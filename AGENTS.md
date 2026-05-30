# PROJECT KNOWLEDGE BASE

**Generated:** 2026-05-30
**Commit:** e7522d4
**Branch:** master
**Version:** v0.7.2

## OVERVIEW

SoundBridge 跨端音频融合软件。核心目标：游戏时不用摘耳机，同时听电脑和手机的声音。Rust 核心库 + Windows C++/C# + Android Kotlin/JNI 三端架构。

## STRUCTURE

```
.
├── rust-core/              # Rust workspace（10 个 crate），核心音频引擎
│   └── crates/
│       ├── audio-core/     # 基础类型：Sample trait、AudioBuffer、AudioFormat、AudioMode
│       ├── audio-codec/    # Opus 编解码（完整实现，零拷贝 API）
│       ├── audio-capture/  # 音频采集（cpal 实现，Fixed(960) 帧大小）
│       ├── audio-playback/ # 音频播放（cpal 实现，混音输出）
│       ├── audio-processor/# 音频处理 AEC/NS/AGC（NLMS/SNR/Attack-Release 实现）
│       ├── audio-mixer/    # 混音引擎（完整实现，soft_clip tanh）
│       ├── network/        # UDP 传输（完整实现，零拷贝序列化）
│       ├── discovery/      # 设备发现 mDNS（mdns_sd 实现）
│       ├── protocol/       # 协议序列化（12字节头，零拷贝）
│       └── ffi-bindings/   # FFI 跨语言绑定（完整管线：capture→encode→send + recv→decode→play）
├── windows/                # Windows 原生 C++ 核心 + C# WinUI 3 界面
│   ├── include/soundbridge/# 公共 API 接口（IAudioEngine、types、export）
│   ├── src/audio/          # WASAPI 采集、Opus 编解码、WebRTC APM
│   ├── src/core/           # AudioEngine、AudioPipeline、Session
│   ├── src/network/        # UDP 传输实现
│   ├── src/ui/             # WinUI 3 C# 界面（MainWindow + ViewModel + TrayIcon）
│   ├── cmake/              # 自定义 CMake Find 模块（FindOpus、FindWebRTC）
│   └── tests/              # GTest 测试文件（test_opus_codec、test_audio_pipeline、test_udp_transport）
├── android/                # Android Kotlin + JNI C++
│   ├── app/src/main/java/com/soundbridge/
│   │   ├── native/         # JNI 桥接（NativeAudioEngine.kt - 40+ 函数）
│   │   ├── audio/          # AudioService + DeviceDiscoveryManager
│   │   └── ui/             # Jetpack Compose 界面（Home、Settings、Theme）
│   └── app/src/main/cpp/   # JNI 实现（pipeline + discovery stubs）
├── android-app/            # 已删除（代码在 android/ 目录）
├── windows-app/            # 已删除（代码在 windows/ 目录）
├── docs/                   # 设计文档、开发计划、技术规格
├── scripts/                # 工具脚本（bench.ps1、test.ps1、build-windows.ps1、release.ps1、verify-release.ps1）
└── tools/                  # 开发工具（benchmark-runner.ps1、test-harness.ps1）
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| **Rust 核心开发** | `rust-core/crates/*/` | 先看各 crate 的 AI_GUIDE.md |
| **Opus 编解码** | `rust-core/crates/audio-codec/` | 完整实现，零拷贝 API |
| **Rust 错误类型** | `rust-core/crates/audio-core/src/lib.rs` | AudioError、Result 定义 |
| **Windows 音频引擎** | `windows/src/core/` | AudioEngine、AudioPipeline、Session |
| **Windows 公共 API** | `windows/include/soundbridge/` | IAudioEngine 接口、类型定义 |
| **Windows 界面** | `windows/src/ui/` | WinUI 3 C#，依赖注入模式 |
| **Android JNI 桥接** | `android/app/src/main/java/com/soundbridge/native/` | NativeAudioEngine.kt |
| **Android 设备发现** | `android/app/src/main/java/com/soundbridge/audio/` | DeviceDiscoveryManager (NsdManager) |
| **Android 界面** | `android/app/src/main/java/com/soundbridge/ui/` | Jetpack Compose，Material3 |
| **设计文档** | `docs/design.md` | 核心需求和架构设计 |
| **开发计划** | `docs/development-plan.md` | 分阶段开发任务 |
| **技术规格** | `docs/technical-spec.md` | 详细参数和协议规格 |

## CONVENTIONS

### 跨平台统一参数
- 采样率：48000 Hz（三端一致）
- 默认通道：单声道（Mono）
- 帧大小：960 samples（20ms@48kHz）
- 内部格式：Float32

### Rust crate 约定
- 每个 crate 必须有 `AI_GUIDE.md`（当前状态和下一步）
- 统一错误类型：`audio_core::Result`
- 骨架 crate 使用 `_private: ()` 字段禁止外部构造
- 测试文件命名：`{name}_test.rs`（非 Rust 社区惯例）

### Windows C++ 约定
- 接口/实现分离：`IAudioEngine`（纯虚）vs `AudioEngineImpl`
- 工厂函数返回 `std::unique_ptr`
- 禁止拷贝：`= delete`
- 命名空间：`soundbridge`
- DLL 导出宏：`SOUNDBRIDGE_API`、`SOUNDBRIDGE_CALL`

### Android Kotlin 约定
- 包名：`com.soundbridge`
- JNI 句柄传递：`jlong`（0L = 无效）
- 所有 `native*` 方法第一个参数是 `engineHandle: Long`
- 必须调用 `nativeRelease` 防止内存泄漏
- UI：Jetpack Compose + Material3，深色主题优先

### 网络协议
- 魔术数：`0x53424447`（"SBDG" = SoundBridge DataGram）
- 音频流：UDP（低延迟）
- 控制信令：UDP（当前实现），未来计划 QUIC（可靠加密）

## ANTI-PATTERNS (THIS PROJECT)

- **不要跳过 AI_GUIDE.md**：修改任何 crate 前先读其 AI_GUIDE.md
- **不要破坏跨平台参数**：48kHz、单声道、960 samples 是硬编码的
- **不要直接构造骨架 crate**：使用 `new()` 工厂函数
- **不要忽略 JNI 句柄释放**：必须调用 `nativeRelease`
- **不要在 Rust crate 中使用 `unwrap()`**：使用 `Result` 传播错误

## COMMANDS

```bash
# Rust
cargo test --workspace          # 运行所有测试
cargo bench --workspace         # 运行基准测试
cargo clippy --workspace        # 代码质量检查
cargo fmt -- --check            # 格式检查
cargo test -p audio-core        # 运行特定 crate 测试

# Windows (CMake)
cmake -B build -S windows       # 配置
cmake --build build             # 构建

# Android (Gradle)
./gradlew build                 # 构建
./gradlew test                  # 测试
```

## NOTES

- 所有 10 个 Rust crate 均已完整实现（非骨架），601 测试通过，零 clippy 警告
- Windows C++ 测试文件已创建（test_opus_codec.cpp、test_audio_pipeline.cpp、test_udp_transport.cpp）
- CI/CD 已配置（.github/workflows/ci.yml：Rust 测试 + Windows C++ 构建 + Android 构建）
- 已有 .editorconfig、rustfmt.toml 格式化配置
- `opus_benchmark.rs` 已修复（rand 依赖 + OpusConfig Copy）
- Phase 1-3 全部完成（v0.7.2），所有连接方式已实现
- QUIC 控制信令未实现（文档已移除相关声明，当前仅 UDP 传输）
- Android JNI 连接管理为存根实现（无 Android NDK 环境无法编译验证）
