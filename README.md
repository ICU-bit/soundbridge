# SoundBridge

> 🎧 声音的桥梁 - 跨端音频融合软件

## 核心目标

**游戏时不用摘耳机，同时听电脑和手机的声音**

---

## 📖 文档导航

| 文档 | 说明 | 查看 |
|------|------|------|
| **[设计文档](docs/design.md)** | 核心需求、架构设计、UI方案 | 👈 从这里开始 |
| **[开发计划](docs/development-plan.md)** | 分阶段开发、每周任务、代码示例 | 👈 实施指南 |
| **[技术规格](docs/technical-spec.md)** | 详细参数、协议规格、性能指标 | 👈 技术参考 |

---

## ✨ 功能特性

### 🔗 连接方式
- ✅ WiFi 局域网（自动发现 - mDNS _soundbridge._udp）
- ✅ WiFi 直连（热点模式）- sb_hotspot_create / sb_hotspot_destroy / sb_hotspot_state
- ✅ USB 有线连接（ADB）- sb_adb_setup_port_forward / sb_adb_state
- ✅ 蓝牙连接 - sb_bt_init / sb_bt_state（BLE + 经典蓝牙）

### 🎵 音频模式
- ✅ 均衡模式（50-100ms 延迟）
- ✅ 高音质模式（48kHz/24bit）
- ✅ 超低延迟模式（<30ms）
- ✅ 动态切换 - UI 已接入（Windows ComboBox + Android SettingsScreen）
- ✅ 混音比例控制 - sb_set_mix_ratio / sb_get_mix_ratio

### 🔄 传输方式
- ✅ 双向同时传输（手机↔电脑）
- ✅ 混音模式（两边音频混合播放）

### 🎛️ 控制方式
- ✅ Windows 桌面客户端 - MainWindow + ViewModel + 19 个 Converter
- ✅ Android App - Jetpack Compose UI + NativeAudioEngine JNI
- ✅ 系统托盘常驻 - TrayIcon.cs（Shell_NotifyIcon P/Invoke）
- ✅ 全局快捷键 - HotkeyManager.cs（RegisterHotKey, Ctrl+Alt+T/M/S）
- ✅ 双向控制 - sb_send_volume / sb_send_pause / sb_send_resume

### ✨ 附加功能
- ✅ 设备自动记忆 - DeviceStore + JSON 持久化（%LocalAppData%/SoundBridge/devices.json）
- ✅ 设备自动发现 - mDNS（Windows sb_discovery_* FFI + Android NsdManager）
- ✅ 启动自启 - Windows Registry（HKCU\...\Run）
- ✅ 连接状态通知 - ConnectionNotificationService + Toast 通知
- ✅ 回声消除（AEC）- NLMS 自适应滤波器
- ✅ 噪声抑制（NS）- SNR 估计
- ✅ 自动增益控制（AGC）- 攻击/释放时间平滑
- ✅ 电平指示器

---

## 🏗️ 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                        用户界面层                            │
├─────────────────────────────────────────────────────────────┤
│  Windows 桌面客户端         │       Android App              │
│  • 托盘图标 + 右键菜单       │       • Material Design 3      │
│  • 电平指示器                │       • 电平指示器              │
│  • 全局快捷键                │       • 快捷操作               │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       服务核心层                              │
├─────────────────────────────────────────────────────────────┤
│  Windows 服务                │       Android 前台服务          │
│  • 开机自启                  │       • 保活运行               │
│  • 后台常驻                  │       • 音频采集               │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       音频引擎层                              │
├─────────────────────────────────────────────────────────────┤
│  • Opus 编解码器 (libopus)                                  │
│  • 低延迟混音引擎                                           │
│  • 回声消除 (WebRTC APM)                                    │
│  • 噪声抑制 (WebRTC APM)                                    │
│  • 自动增益控制 (WebRTC APM)                                 │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       网络传输层                              │
├─────────────────────────────────────────────────────────────┤
│  UDP 音频流                                                   │
│  • 超低延迟传输                                               │
│  • 带宽自适应（64/96/128 kbps 动态码率）                      │
│  • RawJitterBuffer（乱序容忍）                                │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      连接管理层                               │
├─────────────────────────────────────────────────────────────┤
│  WiFi 局域网  │  WiFi 直连  │  USB  │  蓝牙                  │
│  • 自动发现   │  • 热点连接  │ • ADB │  • BLE音频            │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       平台抽象层                              │
├─────────────────────────────────────────────────────────────┤
│  Windows 音频 API            │       Android Audio API       │
│  • WASAPI (低延迟)           │       • AAudio (低延迟)       │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ 技术栈

| 组件 | Windows | Android |
|------|---------|---------|
| **语言** | C++ (核心) + C# (UI) | Kotlin + C++ (JNI) |
| **UI框架** | WinUI 3 | Jetpack Compose |
| **音频采集** | WASAPI | AAudio |
| **编解码** | libopus | libopus |
| **网络** | 自定义 UDP 传输 | 自定义 UDP 传输 |
| **回声消除** | WebRTC APM | WebRTC APM |
| **构建** | CMake + MSBuild | Gradle + NDK |

---

## 📅 开发阶段

### 第一阶段：MVP ✅ 已完成
- ✅ WiFi 局域网连接
- ✅ 双向音频传输
- ✅ Windows 托盘 + Android App
- ✅ 基础混音
- ✅ 电平指示器

### 第二阶段：完整功能 ✅ 已完成
- ✅ 回声消除（NLMS）、噪声抑制（SNR）、自动增益控制（AGC）
- ✅ 设备记忆（JSON 持久化）、启动自启（Registry）
- ✅ 全局快捷键（Ctrl+Alt+T/M/S）
- ✅ 系统托盘 + 连接状态通知（Toast）
- ✅ 音频模式动态切换（均衡/高音质/超低延迟）
- ✅ 其他连接方式（WiFi直连、USB、蓝牙）- FFI 已实现

### 第三阶段：优化完善 ✅ 已完成
- ✅ 音频模式动态切换
- ✅ 性能优化（WASAPI 独占模式 + Jitter Buffer）
- ✅ WiFi 直连（热点模式）- sb_hotspot_* FFI
- ✅ USB/ADB 连接 - sb_adb_* FFI
- ✅ 蓝牙连接 - sb_bt_* FFI
- ✅ 用户体验完善（UI 动画、设备发现、连接状态）
- ✅ CI/CD 配置（GitHub Actions）
- ✅ 开发工具（benchmark-runner、test-harness）
- ✅ 文档同步（AI_GUIDE.md、CONTRIBUTING.md、CHANGELOG.md）

---

## 🎨 UI 设计

### Windows 托盘菜单
```
┌─────────────────────────────┐
│ 🎧 SoundBridge              │
├─────────────────────────────┤
│ 状态: 已连接 (WiFi)          │
│ 手机→电脑: ████████░░ 80%   │
│ 电脑→手机: ░░░░░░░░░░ 0%    │
├─────────────────────────────┤
│ ⚙️ 打开设置                  │
│ 📱 设备管理                  │
│ 🎵 音频模式 > 均衡           │
│ 🔌 连接方式 > WiFi           │
├─────────────────────────────┤
│ 🚪 退出                      │
└─────────────────────────────┘
```

### 快捷键
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Alt+M` | 切换混音模式 |
| `Ctrl+Alt+T` | 切换传输方向 |
| `Ctrl+Alt+P` | 暂停/恢复 |
| `Ctrl+Alt+S` | 打开设置 |

---

## 📊 性能指标

| 指标 | 当前值 | 目标 |
|------|--------|------|
| **延迟（均衡模式）** | ~120 ms（共享）/~40 ms（独占） | <100 ms |
| **延迟（超低延迟）** | ~90 ms（共享）/~30 ms（独占） | <30 ms |
| **CPU占用（空闲）** | <2% | <2% |
| **CPU占用（传输中）** | <15% | <15% |
| **内存占用** | <50 MB | <100 MB |
| **带宽（每通道）** | 64-128 kbps | 64-128 kbps |

> **延迟说明**：延迟由以下部分组成：
>
> | 组件 | 共享模式 | 独占模式 |
> |------|---------|---------|
> | WASAPI 缓冲区 | 50ms | 10ms |
> | Ring buffer | 2×帧大小（均衡40ms/超低20ms） | 2×帧大小（均衡40ms/超低20ms） |
> | cpal 缓冲区 | 1×帧大小（均衡20ms/超低10ms） | 1×帧大小（均衡20ms/超低10ms） |
> | Jitter buffer | 按序直接通过（~0ms） | 按序直接通过（~0ms） |
> | 编解码 | ~5ms | ~5ms |
> | 网络 | ~5ms | ~5ms |
>
> 独占模式通过 `exclusive = true` 参数启用，自动回退共享模式。

---

## 🔧 开发环境

### Windows
- Visual Studio 2022
- Windows 10 SDK
- CMake 3.20+
- WinUI 3

### Android
- Android Studio Hedgehog+
- Android SDK 33+
- NDK 25+
- Kotlin 1.9+

---

## 📝 版本历史

- **v0.7.2** - 开发工具与自动化
  - tools: 新增 benchmark-runner.ps1（基准测试运行器，生成 markdown 报告）
  - tools: 新增 test-harness.ps1（测试运行器，含 clippy/fmt 检查）
  - scripts: 新增 build-windows.ps1（本地 CMake 构建）
  - scripts: 新增 release.ps1（自动化 GitHub Release 创建）
  - scripts: 新增 verify-release.ps1（发布前验证检查器）
  - docs: 新增 CONTRIBUTING.md（开发环境、代码规范、提交约定）
  - docs: 新增 CHANGELOG.md（Keep a Changelog 格式，v0.1.0-v0.7.2）
  - github: 新增 Issue/PR 模板（bug_report、feature_request、PR template）
  - ci: GitHub Actions CI/CD（Rust 测试 + Windows C++ 构建 + Android 构建）
  - config: .editorconfig（UTF-8、CRLF、4 空格缩进）+ rustfmt.toml
  - fix: opus_benchmark.rs 修复（rand dev-dep + OpusConfig Copy derive）
  - style: cargo fmt 全量格式化（30 个文件）
- **v0.7.1** - CI/CD + 项目基础设施
  - ci: GitHub Actions ci.yml 配置（Rust 测试 + Windows C++ 构建 + Android 构建）
  - config: .editorconfig + rustfmt.toml 格式化配置
  - scripts: bench.ps1 + test.ps1 工具脚本
  - docs: Phase 3 完成文档
- **v0.7.0** - UI 动画优化 + Android JNI 连接管理 + 最终测试
  - Windows: ProgressRing 加载动画（连接中状态）
  - Windows: 设备发现 UI（扫描按钮 + 设备列表 + ListView 选择）
  - Windows: 新增 5 个 Converters（InvertBool、InvertBoolToVisibility、BoolToScanText、CollectionToVisibility、EmptyCollectionToVisibility）
  - Windows: 连接按钮三态（Connecting... / Connect / Disconnect）
  - Android: NativeAudioEngine 新增 11 个 JNI 函数（热点/ADB/蓝牙/独占模式）
  - Android: jni_bridge.cpp 连接管理存根实现（静态状态跟踪）
  - 测试: 267 测试全部通过，零 clippy 警告
- **v0.6.0** - 多连接方式 + Oracle Bug 修复 + 音频电平
  - FFI: 新增 sb_get_audio_level（真实 RMS 电平，从采集数据计算）
  - FFI: 新增 sb_set_exclusive_mode（WASAPI 独占模式延迟公式自适应）
  - FFI: 新增 sb_hotspot_create/sb_hotspot_destroy/sb_hotspot_state（WiFi Direct 热点管理）
  - FFI: 新增 sb_adb_setup_port_forward/sb_adb_state/sb_adb_set_state（USB/ADB 端口转发）
  - FFI: 新增 sb_bt_init/sb_bt_state/sb_bt_set_state（蓝牙连接管理）
  - FFI: 修复 sb_playback_write channels 2→1（Oracle Bug 1）
  - FFI: 修复 ConnectionType FFI 死代码 - sb_set_connection_type/sb_get_connection_type（Oracle Bug 2）
  - FFI: 修复 sb_set_audio_mode 热切换 - 管线运行时自动重启（Oracle Bug 3）
  - FFI: SharedPipelineStats 新增 captured_level_bits + exclusive_mode 字段
  - FFI: 带宽自适应 - 发送线程根据丢包率动态调整 Opus 码率（64/96/128kbps）
  - Windows: P/Invoke 新增 sb_get_audio_level / sb_set_exclusive_mode
  - Windows: UI 使用真实 sb_get_audio_level 替代假数据
  - Windows: ConnectionType 选择器（ComboBox）+ 音频模式热切换
  - Windows: 创建 3 个 GTest 测试文件（27 个测试：Opus 编解码 + 音频管线 + UDP 传输）
  - Android: ConnectionType 选择器（FilterChip）+ 音频模式热切换
  - Network: 新增 HotspotConfig/HotspotState/AdbConfig/AdbState/BluetoothConfig/BluetoothState
  - 测试: 76 个 FFI 测试，零 clippy 警告
- **v0.5.0** - Jitter Buffer + WASAPI 独占模式
  - FFI: 接收线程集成 RawJitterBuffer，Opus 包按序解码（乱序容忍）
  - Windows: WasapiCapture/WasapiRenderer 支持独占模式（10ms 缓冲区，自动回退共享模式）
  - Windows: IAudioEngine.initialize() 新增 exclusive 参数
  - Network: 新增 RawJitterBuffer（存储原始 Opus 字节，8 个测试）
  - JNI: nativeConnect 使用 strtol 替代 stoi 防止异常崩溃
  - FFI: 设备发现 JSON 转义特殊字符防止注入
  - 独占模式延迟：均衡~40ms / 超低延迟~30ms（共享模式：~120ms/~90ms）
- **v0.4.0** - AudioModeManager + 混音器集成到管线
  - FFI: sb_set_audio_mode 现在通过 AudioModeManager 切换编解码参数
  - FFI: 接收线程集成 AudioMixer，本地采集 + 远端解码混音后播放
  - FFI: 新增 sb_set_mix_ratio / sb_get_mix_ratio 控制 PC/手机音量平衡
  - Android: JNI 新增 nativeGetAudioMode / nativeSetMixRatio / nativeGetMixRatio
  - Windows: P/Invoke 新增 sb_set_mix_ratio / sb_get_mix_ratio
  - AudioMixer 新增 Clone trait
  - 新增 6 个 FFI 测试（混音比例边界值、无效值、null 指针）
- **v0.3.0** - FFI bindings + Windows/Android UI 完整实现
  - FFI: 音频模式切换、连接状态回调、双向控制
  - Windows: MainWindow + ViewModel + TrayIcon + HotkeyManager + 设备持久化 + 自启
  - Android: SettingsScreen 音频模式下拉 + JNI 音频模式切换
- **v0.2.0** - Rust 核心 MVP（10 个 crate，181+ 测试）
- **v0.1.0** - 初始设计文档完成

---

## 📄 许可证

待定

---

## 👥 贡献者

- SoundBridge Team

---

## 📧 联系方式

待定
