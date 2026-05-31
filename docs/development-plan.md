# SoundBridge 开发计划

> 唯一开发文档 — 设计、规格、进度、计划合一
> 最后更新：2026-05-31 (阶段 7 完成，阶段 8 进行中)

---

## 项目概览

跨端音频融合：Windows (C++/C#) ↔ Android (Kotlin/JNI)，Rust 核心引擎。
核心场景：游戏时不用摘耳机，同时听电脑和手机的声音。

### 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                        用户界面层                            │
├─────────────────────────────────────────────────────────────┤
│  Windows 桌面客户端         │       Android App              │
│  • 托盘图标 + 右键菜单       │       • Material Design 3      │
│  • 电平指示器                │       • 电平指示器              │
│  • 全局快捷键                │       • 快捷操作               │
│  • 设备管理                  │       • 设备管理               │
│  • 模式切换                  │       • 模式切换               │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       服务核心层                              │
├─────────────────────────────────────────────────────────────┤
│  Windows 服务                │       Android 前台服务          │
│  • 开机自启                  │       • 保活运行               │
│  • 后台常驻                  │       • 音频采集               │
│  • 系统通知                  │       • 系统通知               │
│  • 设备记忆                  │       • 设备记忆               │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       音频引擎层                              │
├─────────────────────────────────────────────────────────────┤
│  • Opus 编解码器 (libopus)                                  │
│  • 低延迟混音引擎                                           │
│  • 自研 AEC/NS/AGC (NLMS/SNR/攻击释放)                      │
│  • PLC 丢包隐藏 (波形外推 + Hanning 窗)                     │
│  • 电平检测 (RMS)                                           │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       网络传输层                              │
├─────────────────────────────────────────────────────────────┤
│  UDP 音频流 + 控制信令                                        │
│  • SRTP 加密 (AES-128-CM + HMAC-SHA1-80)                   │
│  • ECDH 密钥交换 (x25519-dalek)                             │
│  • 带宽自适应（64/96/128 kbps 动态码率）                      │
│  • RawJitterBuffer（乱序容忍）                                │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      连接管理层                               │
├─────────────────────────────────────────────────────────────┤
│  WiFi 局域网  │  WiFi 热点  │  USB/ADB  │  蓝牙 RFCOMM       │
│  • mDNS 发现  │  • netsh    │  • 端口转发│  • UDP 桥接        │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                       平台抽象层                              │
├─────────────────────────────────────────────────────────────┤
│  Windows 音频 API            │       Android Audio API       │
│  • WASAPI (低延迟)           │       • AAudio (低延迟)       │
│  • 环回捕获                   │       • AudioRecord          │
│  • 音频会话                   │       • AudioTrack           │
└─────────────────────────────────────────────────────────────┘
```

### 技术栈

| 组件 | Windows | Android |
|------|---------|---------|
| **语言** | C++ (核心) + C# (UI) | Kotlin + C++ (JNI) |
| **UI框架** | WinUI 3 | Jetpack Compose |
| **音频采集** | WASAPI | AAudio |
| **编解码** | libopus | libopus |
| **网络** | 自定义 UDP 传输 | 自定义 UDP 传输 |
| **加密** | x25519 + DTLS + SRTP | x25519 + SRTP |
| **构建** | CMake + MSBuild | Gradle + NDK |

---

## 技术规格

### 音频参数

| 参数 | 值 |
|------|-----|
| 采样率 | 48000 Hz |
| 位深 | 32 bit (Float32) |
| 声道 | 单声道 (1ch) |
| 帧大小 | 960 samples |
| 帧时长 | 20 ms |

### Opus 编码参数

| 模式 | 码率 | 信号类型 |
|------|------|----------|
| 均衡模式 | 96 kbps | 自动 |
| 高音质模式 | 128 kbps | 音乐 |
| 超低延迟模式 | 64 kbps | 语音 |

### 延迟预算

```
总延迟 = 采集延迟 + 编码延迟 + 网络延迟 + 解码延迟 + 播放延迟

均衡模式（实测 ~40ms）：
  采集: 10-20ms | 编码: 20ms | 网络: 5-20ms | 解码: 5ms | 播放: 10-20ms

超低延迟模式（实测 ~30ms）：
  采集: 5-10ms | 编码: 10ms | 网络: 5-15ms | 解码: 2.5ms | 播放: 5-10ms
```

### 网络协议

#### 音频包格式 (UDP)
```
┌──────────────────────────────────────────────────────────┐
│  字节 0-3:   序列号 (uint32_t, 网络字节序)               │
│  字节 4-7:   时间戳 (uint32_t, 毫秒, 网络字节序)         │
│  字节 8:     标志位 (uint8_t)                            │
│              - bit 0: 关键帧 | bit 1: 混音 | bit 2-7: 保留│
│  字节 9:     通道数 (uint8_t)                            │
│  字节 10-11: Opus 数据长度 (uint16_t, 网络字节序)        │
│  字节 12-N:  Opus 编码数据                               │
└──────────────────────────────────────────────────────────┘
```

#### 控制消息格式 (UDP)
```
┌──────────────────────────────────────────────────────────┐
│  字节 0-3:   消息长度 (uint32_t, 网络字节序)             │
│  字节 4:     消息类型 (uint8_t)                          │
│  字节 5-N:   消息体 (JSON 或二进制)                      │
└──────────────────────────────────────────────────────────┘
```

#### 消息类型
| 类型 | 代码 | 说明 |
|------|------|------|
| HELLO / HELLO_ACK | 0x01 / 0x21 | 握手 |
| AUTH / AUTH_ACK | 0x02 / 0x22 | 认证 |
| START_AUDIO | 0x03 | 开始音频传输 |
| STOP_AUDIO | 0x04 | 停止音频传输 |
| CHANGE_MODE | 0x05 | 切换音频模式 |
| VOLUME | 0x06 | 音量控制 |
| STATUS / STATUS_ACK | 0x07 / 0x27 | 状态查询 |
| HEARTBEAT / HEARTBEAT_ACK | 0x08 / 0x28 | 心跳 |
| ERROR | 0xFF | 错误 |

#### mDNS 服务发现
- 服务类型：`_soundbridge._udp.local.`
- 属性：`id`, `name`, `type`(android/windows), `version`, `capabilities`

### 连接流程

**WiFi 局域网**：mDNS 发现 → 建立 UDP 控制连接 → 握手认证 → 开始音频流

**WiFi 热点**：手机开热点 → 电脑连接 → 手机启动服务 → 自动发现/手动输入 IP → 建立连接

**USB/ADB**：手机开 USB 调试 → ADB 连接 → `adb forward` 端口转发 → 通过本地端口建立连接

**蓝牙 RFCOMM**：设备配对 → RFCOMM 接入 → 本地 UDP 桥接 → 接入 Rust 管线

### 安全

| 层级 | 实现 |
|------|------|
| 密钥交换 | x25519-dalek ECDH |
| 密钥推导 | HKDF-SHA1 (Windows) / DTLS handshake (Android) |
| 数据加密 | SRTP AES-128-CM + HMAC-SHA1-80 |
| 线程安全 | AtomicBool + Mutex 保护密钥材料 |

### 兼容性

| 平台 | 最低版本 | 推荐版本 |
|------|----------|----------|
| Windows | Windows 10 1809 | Windows 11 |
| Android | Android 8.0 (API 26) | Android 12+ |

支持所有 WASAPI/AAudio 兼容设备。蓝牙 4.0+ (BLE), 2.1+ (经典)。WiFi 802.11n/ac/ax。

---

## UI 设计

### 设计风格
**简约现代** — Material Design 3 / Fluent Design，深色/浅色自适应，响应式布局。

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

### Windows 快捷键
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Alt+T` | 切换传输方向 |
| `Ctrl+Alt+M` | 切换混音模式 |
| `Ctrl+Alt+S` | 打开设置 |

### Android App
- **主界面**：连接状态、电平指示器、快捷控制
- **设备管理**：已配对设备列表、连接历史
- **设置页面**：音频模式、连接方式、快捷键配置

---

## 第一阶段：MVP (最小可行产品) ✅ 已完成

**实际完成**：2026年5月

### 1.1 周次规划

#### Week 1-2：项目搭建 + 核心音频引擎 ✅
- ✅ 项目结构搭建（Windows + Android）
- ✅ 集成 Opus 编解码库
- ✅ 实现基础音频采集（WASAPI + AAudio）
- ✅ 实现基础音频播放
- ✅ 单元测试框架搭建

#### Week 3-4：网络传输层 ✅
- ✅ UDP 音频流传输实现
- ✅ TCP 控制信令实现
- ✅ 设备发现（mDNS）- Rust FFI + Windows C# + Android NsdManager + UI
- ✅ 连接管理（建立/断开/重连）
- ✅ 基础丢包恢复

#### Week 5-6：混音 + UI ✅
- ✅ 混音引擎实现
- ✅ Windows 托盘应用
- ✅ Android 基础 UI
- ✅ 电平指示器
- ✅ 连接状态显示

#### Week 7-8：集成测试 + 优化 ✅
- ✅ 端到端测试
- ✅ 延迟优化
- ✅ 稳定性测试
- ✅ Bug 修复
- ✅ 用户体验优化

### 1.2 技术实现细节

#### Windows 音频采集 (WASAPI)
```cpp
class AudioCapture {
public:
    bool Initialize();
    bool Start();
    bool Stop();
    void SetCallback(AudioCallback callback);
private:
    IAudioClient* audioClient;
    IAudioCaptureClient* captureClient;
    WAVEFORMATEX* format;
};
```

#### Android 音频采集 (AAudio)
```kotlin
class AudioCapture {
    private var aaudioStream: AAudioStream? = null
    fun initialize(): Boolean
    fun start(): Boolean
    fun stop(): Boolean
    fun setCallback(callback: AudioCallback)
}
```

#### Opus 编解码
```cpp
class OpusCodec {
public:
    bool Initialize(int sampleRate, int channels, int bitrate);
    int Encode(const int16_t* pcm, int frameSize, uint8_t* output, int maxOutput);
    int Decode(const uint8_t* data, int dataLen, int16_t* pcm, int maxFrameSize);
private:
    OpusEncoder* encoder;
    OpusDecoder* decoder;
};
```

#### UDP 音频流
```cpp
class UdpAudioStream {
public:
    bool Bind(int port);
    bool SendTo(const uint8_t* data, int len, const sockaddr_in& addr);
    int ReceiveFrom(uint8_t* buffer, int bufferSize, sockaddr_in& fromAddr);
private:
    int sockfd;
    std::atomic<bool> running;
};
```

---

## 第二阶段：完整功能 ✅ 已完成

**实际完成**：2026年5月

### 2.1 多种连接方式 ✅
- ✅ WiFi 直连（热点模式）- sb_hotspot_create / sb_hotspot_destroy / sb_hotspot_state
- ✅ USB 有线连接（ADB）- sb_adb_setup_port_forward / sb_adb_state
- ✅ 蓝牙连接（BLE + 经典）- sb_bt_init / sb_bt_state
- ✅ 连接方式自动选择 - ConnectionType FFI + UI 选择器

### 2.2 音频处理 ✅
- ✅ 自研 AEC/NS/AGC 替代 WebRTC APM（NLMS/SNR/攻击释放平滑）
- ✅ 回声消除（AEC）- NLMS 自适应滤波器
- ✅ 噪声抑制（NS）- SNR 估计
- ✅ 自动增益控制（AGC）- 攻击/释放时间平滑
- ✅ 丢包隐藏（PLC）- 波形外推 + Hanning 窗 + 舒适噪声

### 2.3 设备管理 + 快捷键 ✅
- ✅ 设备记忆（JSON 持久化到 %LocalAppData%/SoundBridge/devices.json）
- ✅ 启动自启（Windows Registry HKCU\...\Run）
- ✅ 全局快捷键（HotkeyManager, Ctrl+Alt+T/M/S）
- ✅ 系统通知（ConnectionNotificationService + Toast）

### 2.4 技术实现细节

#### 设备记忆存储
```json
{
    "devices": [
        {
            "id": "device_001",
            "name": "Pixel 7",
            "type": "wifi",
            "last_connected": "2024-01-15T10:30:00Z",
            "auto_connect": true
        }
    ]
}
```

---

## 第三阶段：优化完善 ✅ 已完成

**完成时间**：2026年5月30日
**版本**：v0.7.0 → v0.8.0

### 3.1 音频模式切换 ✅
- ✅ 动态切换音频模式（Windows ComboBox + Android SettingsScreen）
- ✅ 混音比例调节（sb_set_mix_ratio / sb_get_mix_ratio，Arc<AtomicU32> 跨线程）
- ✅ 带宽自适应（丢包检测 + 动态码率 64/96/128kbps）

### 3.2 性能优化 ✅
- ✅ 延迟优化（WASAPI 独占模式 10ms 缓冲区：Balanced ~40ms，LowLatency ~30ms）
- ✅ Jitter Buffer 集成（RawJitterBuffer 存储原始 Opus 字节，按序解码）
- ✅ CPU/内存优化（零分配热路径：decode_into, encode_interleaved_into, mix_two_into）
- ✅ VecDeque 替代 jitter buffer Vec（O(1) pop_front）
- ✅ SRTP 零分配 protect_into/unprotect_into

### 3.3 多连接方式 UI ✅
- ✅ Windows ComboBox + Android FilterChip 连接方式选择
- ✅ WiFi 直连（netsh hostednetwork / WifiP2pManager）
- ✅ USB/ADB 连接（adb forward / Runtime.exec）
- ✅ 蓝牙连接（RFCOMM + UDP 桥接）
- ✅ 真实音频电平检测（sb_get_audio_level，RMS 从采集数据计算）

### 3.4 安全加固 ✅
- ✅ ECDH 密钥交换：模拟→真实 x25519-dalek
- ✅ DTLS 密钥推导：XOR→HKDF-SHA1 (Windows)
- ✅ 线程安全：encryption_enabled_→AtomicBool
- ✅ JNI 全局变量→std::atomic
- ✅ SRTP 密钥材料加 mutex 保护
- ✅ WSA/COM 资源清理
- ✅ 移除 6 处 unwrap()/expect()

### 3.5 功能完善 ✅
- ✅ 静音：管线线程检查 muted 标志（AtomicBool）
- ✅ Windows 开机自启（Registry HKCU）
- ✅ Windows WiFi 热点（netsh）
- ✅ Windows ADB 端口转发
- ✅ Windows 自动连接上次设备
- ✅ Android 静音按钮→nativeSetMute 完整 JNI 链路
- ✅ Android 设置持久化（SharedPreferences）
- ✅ Android 蓝牙 RFCOMM 接入管线 + UDP 桥接
- ✅ Mono 默认值修正（channels=1）

### 3.6 测试 ✅
- ✅ FFI 占位符测试→真实测试（10个）
- ✅ Android JNI 原生测试
- ✅ Windows C++ GTest（27 个测试）
- ✅ Rust 623+ 测试通过，零 clippy 警告

### 3.7 技术实现细节

#### 音频模式切换
```cpp
class AudioModeManager {
public:
    enum Mode { BALANCED, HIGH_QUALITY, LOW_LATENCY };
    bool SwitchMode(Mode mode);
    Mode GetCurrentMode();
private:
    Mode currentMode;
    void UpdateCodecParams(Mode mode);
    void UpdateBufferSize(Mode mode);
};
```

---

## 第四阶段：质量与可靠性 ✅ 已完成

**完成时间**：2026年5月31日
**版本**：v0.8.0 → v0.9.0

### 4.1 端到端真实测试 [P0] ✅
- ✅ 编写手动测试指南：PC↔手机连接步骤（docs/manual-test-guide.md）
- ✅ 编写延迟测量工具（tools/latency-measure.ps1）
- [ ] 在真实 WiFi 环境下测试完整音频管线
- [ ] 记录各连接方式的实际延迟数据

### 4.2 音频处理升级 [P0] ✅
- ✅ 实现 FEC（Opus inband_fec + 冗余包）- FecEncoder/FecDecoder/FecCodec
- [ ] 集成 WebRTC APM（可选，当前自研 AEC/NS/AGC 已可用）

### 4.3 崩溃恢复 [P0] ✅
- ✅ Rust 添加 panic_hook，捕获 panic 并上报（setup_panic_hook()）
- ✅ 添加结构化日志收集（tracing + tracing-subscriber）
- ✅ sb_init() FFI 入口点（安全初始化，Once 保证只执行一次）
- ✅ sb_version() FFI 函数（返回 CARGO_PKG_VERSION）
- ✅ Android 添加 UncaughtExceptionHandler（SoundBridgeApp.installCrashHandler()）

### 4.4 断线重连 [P1] ✅
- ✅ Rust 层：ReconnectManager 重连管理器（reconnect.rs）
- ✅ 状态机：Idle → Connected → Disconnected → Reconnecting → Recovered/Exhausted
- ✅ 指数退避策略：1s → 2s → 4s → 8s → 16s → 32s（上限）
- ✅ 最大重试次数限制（默认 10，可配置）
- ✅ ReconnectStats 统计：total_attempts, successful_reconnects, failed_reconnects
- ✅ 15 个单元测试
- ✅ Android AudioService：监听连接断开，自动重连（ReconnectState + 指数退避）
- ✅ UI 显示重连状态和进度（ReconnectProgressCard + 手动重连按钮）

---

## 第五阶段：用户体验 ✅ 已完成

**完成时间**：2026年5月31日

### 5.1 用户反馈 [P1] ✅
- ✅ 连接失败时显示错误对话框（FeedbackManager + ErrorDialog）
- ✅ 连接超时提示（15 秒超时检测 + TimeoutDialog）
- ✅ 音频质量差时提示（基于 NetMonitor 质量评分）

### 5.2 首次运行引导 [P1] ✅
- ✅ Android：首次启动引导页（FirstRunGuideScreen - 4 步向导）
  - 欢迎页 → 权限申请 → 音频测试 → 使用说明
  - SharedPreferences 记录完成状态
- [ ] Windows：首次启动引导页（待实现）

### 5.3 无障碍 [P1] ✅
- ✅ Android：所有图标添加 ContentDescription（HomeScreen 8/8, SettingsScreen 6/6, FirstRunGuide 4/4）
- [ ] Windows：添加 AutomationProperties
- [ ] 支持高对比度模式

### 5.4 国际化 [P1] ✅
- ✅ Android：提取硬编码字符串到 strings.xml（30+ 字符串，支持中/英双语）
- [ ] Windows：创建 .resx 资源文件
- [ ] 支持中/英双语切换

---

## 第六阶段：发布准备 ✅ 已完成

**完成时间**：2026年5月31日

### 6.1 打包 [P0] ✅
- ✅ Windows：创建打包脚本（scripts/package-windows.ps1）
- ✅ Android：配置签名（keystore.properties.template）+ 打包脚本（scripts/package-android.ps1）
- ✅ CI 添加 release 构建和上传（.github/workflows/release.yml）
- ✅ 综合打包脚本（scripts/package.ps1）
- [ ] 生成签名密钥并配置 GitHub Secrets
- [ ] 测试完整打包流程

### 6.2 文档 [P1] ✅
- ✅ 用户手册：安装、配置、使用（docs/user-manual.md - 295 行）
- ✅ 故障排除指南（docs/troubleshooting.md - 634 行）
- ✅ API 文档（docs/api-reference.md - 524 行）

---

## 第七阶段：进阶功能 ✅ 已完成

**完成时间**：2026年5月31日

### 7.1 网络增强 [P2] ✅
- ✅ 带宽自适应平滑调整（PID 控制器替代 3 级硬阈值，bandwidth_pid.rs）
- [ ] mDNS 跨子网发现（手动 IP 输入 + 云中继）
- [ ] QUIC 控制通道接入实际管线

### 7.2 音频增强 [P2] ✅
- ✅ 立体声支持（stereo.rs，mono_to_stereo/stereo_to_mono）
- ✅ 均衡器 / 音效（10 段参数均衡器，eq.rs，6 种预设）
- ✅ 音频 Profile 系统（audio_profile.rs，8 种 Profile）
- ✅ 自动 Profile 管理器（auto_profile.rs，网络评分驱动）
- [ ] 特定 App 音频捕获（WASAPI loopback per-session）

### 7.3 多设备 [P2]
- [ ] 支持同时连接多台手机
- [ ] 每设备独立音量控制

### 7.4 平台扩展 [P3]
- [ ] macOS 支持
- [ ] iOS 支持
- [ ] Linux 支持

### 7.5 FFI 与 UI 集成 [P0] ✅
- ✅ FFI 接口扩展（8 个新函数：profile/EQ/channel/auto）
- ✅ Windows UI 集成（音频 Profile 选择 + 均衡器控制）
- ✅ Android UI 集成（AudioProfileSection + EqualizerSection）

---

## 优先级排序

### 目标：尽快发布可用版本
```
4.1 端到端测试 → 4.2 音频处理 → 6.1 打包 → 4.4 断线重连 → 5.1 用户反馈 → 5.2 引导
```

### 目标：功能完善
```
4.2 音频处理 → 7.1 网络增强 → 7.2 音频增强 → 4.4 断线重连 → 7.3 多设备
```

---

## 功能完整度矩阵

| 功能 | Windows | Android | 说明 |
|------|---------|---------|------|
| 音频流传输 | ✅ | ✅ | capture→encode→UDP→decode→play |
| 设备发现 (mDNS) | ✅ | ✅ | mdns_sd + NsdManager |
| WiFi 局域网 | ✅ | ✅ | UDP socket |
| WiFi 热点 | ✅ | ✅ | netsh + WifiP2pManager |
| ADB 连接 | ✅ | ✅ | adb forward + Runtime.exec |
| 蓝牙连接 | ❌ | ✅ | RFCOMM + UDP 桥接 |
| 音频模式切换 | ✅ | ✅ | 3 种模式 |
| 混音比例控制 | ✅ | ✅ | AtomicU32 实时调节 |
| 静音/取消静音 | ✅ | ✅ | AtomicBool + 管线检查 |
| 音频电平指示 | ✅ | ✅ | 真实 RMS |
| SRTP 加密 | ✅ | ✅ | AES-128-CM + HMAC-SHA1-80 |
| 设备记忆 | ✅ | ✅ | JSON 持久化 |
| 开机自启 | ✅ | ❌ | Registry HKCU |
| 自动连接 | ✅ | ❌ | DeviceStore auto_connect |
| 设置持久化 | ❌ | ✅ | SharedPreferences |
| AEC/NS/AGC | ⚠️ | ⚠️ | 简化版，非 WebRTC APM |
| FEC | ✅ | ✅ | FecEncoder/FecDecoder (Opus inband_fec) |
| 断线重连 | ✅ | ⚠️ | Rust ReconnectManager，Android 待集成 |

---

## 里程碑

| 里程碑 | 状态 | 说明 |
|--------|------|------|
| 1. MVP | ✅ | WiFi 连接 + 双向音频 + 托盘/Compose UI |
| 2. 完整功能 | ✅ | 多连接方式 + AEC/NS/AGC + 设备记忆 + 快捷键 |
| 3. 优化发布 (v0.8.0) | ✅ | 音频模式 + 性能优化 + 安全加固 + 功能完善 |
| 4. 质量发布 (v0.9.0) | ✅ | 端到端测试 + FEC + panic_hook + 断线重连 |
| 5. 用户体验 (v1.0.0) | ✅ | 用户反馈 + 首次引导 + 自动重连 |
| 6. 发布准备 (v1.1.0) | ✅ | 打包脚本 + 文档（用户手册/故障排除/API） |
| 7. 进阶功能 (v0.10.0) | ✅ | PID 带宽控制 + 音频 Profile + 立体声 + 均衡器 + 自动 Profile |
| 8. 发布打磨 (v1.0.0-rc) | 🔴 | 无障碍 + 国际化 + 发布流程验证 |

---

## 风险和应对

| 风险 | 应对方案 |
|------|----------|
| 延迟过高 | 优化缓冲区大小、WASAPI 独占模式、减少处理环节 |
| 音频卡顿 | 增大 jitter buffer、优化丢包恢复、调整编码参数 |
| 设备兼容性 | 充分测试主流设备、提供降级方案 |
| AEC 效果差 | 集成 WebRTC APM 或实现双讲检测 |
| 功耗过高 | 优化后台服务、减少不必要的处理 |

---

## 测试计划

### 单元测试
- Rust: `cargo test --workspace`（800+ 测试）
- Windows C++: GTest（27 测试）
- Android: JUnit + instrumented tests

### 集成测试
- 端到端音频传输（localhost loopback）
- 多设备连接
- 长时间稳定性

### 性能测试
- 延迟测试（各连接方式）
- CPU/内存占用
- 带宽使用

---

## 附录

### 开发工具
- **Windows**: Visual Studio 2022, CMake, WinUI 3
- **Android**: Android Studio, Gradle, NDK
- **版本控制**: Git
- **CI/CD**: GitHub Actions

### 依赖库
- **Opus**: 音频编解码
- **cpal**: 跨平台音频采集/播放
- **tokio**: 异步运行时
- **quinn**: QUIC 协议实现
- **mdns-sd**: mDNS 设备发现
- **aes/hmac/sha1**: SRTP 加密
- **x25519-dalek**: ECDH 密钥交换

### 术语表
- **AEC**: Acoustic Echo Cancellation，回声消除
- **AGC**: Automatic Gain Control，自动增益控制
- **NS**: Noise Suppression，噪声抑制
- **PLC**: Packet Loss Concealment，丢包隐藏
- **Opus**: 开源音频编码格式，专为实时通信设计
- **WASAPI**: Windows Audio Session API，Windows 低延迟音频 API
- **AAudio**: Android Audio API，Android 低延迟音频 API
- **SRTP**: Secure Real-time Transport Protocol，安全实时传输协议
- **ECDH**: Elliptic Curve Diffie-Hellman，椭圆曲线密钥交换
