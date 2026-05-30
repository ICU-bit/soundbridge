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
- ⚠️ WiFi 直连（热点模式）- 未实现
- ⚠️ USB 有线连接 - 未实现
- ⚠️ 蓝牙连接 - 未实现

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
- ✅ Windows 桌面客户端 - MainWindow + ViewModel + 8 个 Converter
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
│  UDP 音频流                   │       QUIC 控制信令           │
│  • 超低延迟传输               │       • 可靠加密传输          │
│  • 带宽自适应                 │       • 设备发现              │
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
| **网络** | 自定义 UDP/QUIC | 自定义 UDP/QUIC |
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
- ⚠️ 其他连接方式（WiFi直连、USB、蓝牙）- 未实现

### 第三阶段：优化完善 ⏳ 进行中
- ✅ 音频模式动态切换
- ✅ 性能优化（WASAPI 独占模式 + Jitter Buffer）
- [ ] WiFi 直连（热点模式）
- [ ] USB/ADB 连接
- [ ] 蓝牙连接
- [ ] 用户体验完善

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
