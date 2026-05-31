# SoundBridge 开发计划

> 分阶段开发，逐步完善
> 最后更新：2026-05-31

---

## 项目概览

跨端音频融合：Windows (C++/C#) ↔ Android (Kotlin/JNI)，Rust 核心引擎。
核心场景：游戏时不用摘耳机，同时听电脑和手机的声音。

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

#### 1.2.1 Windows 音频采集 (WASAPI)
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

#### 1.2.2 Android 音频采集 (AAudio)
```kotlin
class AudioCapture {
    private var aaudioStream: AAudioStream? = null
    fun initialize(): Boolean
    fun start(): Boolean
    fun stop(): Boolean
    fun setCallback(callback: AudioCallback)
}
```

#### 1.2.3 Opus 编解码
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

#### 1.2.4 UDP 音频流
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

## 第四阶段：质量与可靠性 🔴 未开始

### 4.1 端到端真实测试 [P0]
- [ ] 编写手动测试指南：PC↔手机连接步骤
- [ ] 编写延迟测量工具（跨网络，非 localhost）
- [ ] 在真实 WiFi 环境下测试完整音频管线
- [ ] 记录各连接方式的实际延迟数据

### 4.2 音频处理升级 [P0]
- [ ] 集成 WebRTC APM（spikes/webrtc-apm/ 已有 spike）
  - AEC：双讲检测、延迟估计、非线性处理
  - NS：频域处理 (FFT)、Wiener 滤波
  - AGC：前瞻、压缩拐点
- [ ] 或：实现 FEC（Opus inband_fec + 冗余包）

### 4.3 崩溃恢复 [P0]
- [ ] Rust 添加 panic_hook，捕获 panic 并上报
- [ ] 添加结构化日志收集（tracing→文件）
- [ ] Android 添加 UncaughtExceptionHandler

### 4.4 断线重连 [P1]
- [ ] Rust 层：receiver 线程检测丢包后重新握手
- [ ] Android AudioService：监听连接断开，自动重连
- [ ] UI 显示重连状态和进度

---

## 第五阶段：用户体验 🔴 未开始

### 5.1 用户反馈 [P1]
- [ ] 连接失败时显示错误对话框
- [ ] 连接超时提示
- [ ] 音频质量差时提示（基于 NetMonitor 质量评分）

### 5.2 首次运行引导 [P1]
- [ ] Windows：首次启动引导页（选择连接方式、测试音频）
- [ ] Android：首次启动权限申请引导

### 5.3 无障碍 [P1]
- [ ] Android：所有图标添加 ContentDescription
- [ ] Windows：添加 AutomationProperties
- [ ] 支持高对比度模式

### 5.4 国际化 [P1]
- [ ] Android：提取硬编码字符串到 strings.xml
- [ ] Windows：创建 .resx 资源文件
- [ ] 支持中/英双语

---

## 第六阶段：发布准备 🔴 未开始

### 6.1 打包 [P0]
- [ ] Windows：创建 MSIX/MSI 安装包
- [ ] Android：配置签名 AAB (Google Play)
- [ ] CI 添加 release 构建和上传

### 6.2 文档 [P1]
- [ ] 用户手册：安装、配置、使用
- [ ] 故障排除指南
- [ ] API 文档（FFI 接口）

---

## 第七阶段：进阶功能 🔴 未开始

### 7.1 网络增强 [P2]
- [ ] mDNS 跨子网发现（手动 IP 输入 + 云中继）
- [ ] QUIC 控制通道接入实际管线
- [ ] 带宽自适应平滑调整（替代 3 级硬阈值）

### 7.2 音频增强 [P2]
- [ ] 特定 App 音频捕获（WASAPI loopback per-session）
- [ ] 立体声支持
- [ ] 均衡器 / 音效

### 7.3 多设备 [P2]
- [ ] 支持同时连接多台手机
- [ ] 每设备独立音量控制

### 7.4 平台扩展 [P3]
- [ ] macOS 支持
- [ ] iOS 支持
- [ ] Linux 支持

---

## 优先级排序建议

### 目标：尽快发布可用版本
```
4.1 端到端测试 → 4.2 音频处理 → 6.1 打包 → 4.4 断线重连 → 5.1 用户反馈 → 5.2 引导
```

### 目标：功能完善
```
4.2 音频处理 → 7.1 网络增强 → 7.2 音频增强 → 4.4 断线重连 → 7.3 多设备
```

---

## 当前功能完整度矩阵

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
| FEC | ❌ | ❌ | 未实现 |
| 断线重连 | ⚠️ | ❌ | 部分实现 |

---

## 里程碑

### 里程碑 1：MVP 完成 ✅
- Windows 桌面客户端（托盘应用 + MainWindow + ViewModel）
- Android App（Jetpack Compose UI + NativeAudioEngine JNI）
- WiFi 局域网连接 + 双向音频传输 + 电平指示器

### 里程碑 2：完整功能 ✅
- 多种连接方式（WiFi LAN + WiFi Direct + USB/ADB + Bluetooth）
- 回声消除 + 噪声抑制 + 自动增益控制
- 设备记忆 + 启动自启 + 全局快捷键

### 里程碑 3：优化发布 ✅ (v0.8.0)
- 音频模式动态切换 + 性能优化
- 安全加固（ECDH + DTLS + SRTP）
- 功能完善（静音/自启/热点/ADB/蓝牙/持久化）

### 里程碑 4：质量发布 🔴
- 端到端真实测试
- 音频处理升级（WebRTC APM 或 FEC）
- 打包发布（MSIX + AAB）
- 断线重连 + 用户反馈

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
- 音频采集/播放、Opus 编解码、网络传输、混音引擎
- Rust: `cargo test --workspace`（623+ 测试）
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

### 用户测试
- 真实使用场景
- 用户体验反馈
- Bug 报告

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
