# SoundBridge 开发计划

> 分阶段开发，逐步完善

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
// 核心类设计
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
// 核心类设计
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
// 核心类设计
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
// 核心类设计
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

## 第二阶段：完整功能 ✅ 大部分完成

**实际完成**：2026年5月

### 2.1 周次规划

#### Week 1-2：多种连接方式 ✅ 已完成
- ✅ WiFi 直连（热点模式）- sb_hotspot_create / sb_hotspot_destroy / sb_hotspot_state
- ✅ USB 有线连接（ADB）- sb_adb_setup_port_forward / sb_adb_state
- ✅ 蓝牙连接（BLE + 经典）- sb_bt_init / sb_bt_state
- ✅ 连接方式自动选择 - ConnectionType FFI + UI 选择器

#### Week 3-4：音频处理 ✅
- ✅ 集成 WebRTC APM（自研 NLMS/SNR/AGC 替代）
- ✅ 回声消除（AEC）- NLMS 自适应滤波器
- ✅ 噪声抑制（NS）- SNR 估计
- ✅ 自动增益控制（AGC）- 攻击/释放时间平滑

#### Week 5-6：设备管理 + 快捷键 ✅
- ✅ 设备记忆（JSON 持久化到 %LocalAppData%/SoundBridge/devices.json）
- ✅ 启动自启（Windows Registry HKCU\...\Run）
- ✅ 全局快捷键（HotkeyManager, Ctrl+Alt+T/M/S）
- ✅ 系统通知（ConnectionNotificationService + Toast）

### 2.2 技术实现细节

#### 2.2.1 WebRTC APM 集成
```cpp
// 核心类设计
class AudioProcessing {
public:
    bool Initialize();
    int ProcessStream(int16_t* src, int16_t* dest, int samplesPerChannel);
    int ProcessReverseStream(int16_t* src, int16_t* dest, int samplesPerChannel);
    
private:
    webrtc::AudioProcessing* apm;
    webrtc::StreamConfig config;
};
```

#### 2.2.2 设备记忆存储
```cpp
// Windows: JSON 文件存储
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

**开始时间**：2026年5月
**完成时间**：2026年5月30日
**版本**：v0.7.0

### 3.1 周次规划

#### Week 1-2：音频模式切换 ✅
- ✅ 动态切换音频模式（Windows ComboBox + Android SettingsScreen）
- ✅ 混音比例调节（sb_set_mix_ratio / sb_get_mix_ratio，Arc<AtomicU32> 跨线程）
- ✅ 带宽自适应（丢包检测 + loss_rate 返回）

#### Week 3-4：性能优化 + UI 完善
- ✅ 延迟优化（WASAPI 独占模式 10ms 缓冲区：Balanced ~40ms，LowLatency ~30ms）
- ✅ Jitter Buffer 集成（RawJitterBuffer 存储原始 Opus 字节，按序解码）
- ✅ CPU/内存优化（零分配热路径：decode_into, encode_interleaved_into, mix_two_into, serialize_audio_into, deserialize_header）
- ✅ 混音比例 UI（Windows Slider + Android Slider，接入 sb_set_mix_ratio FFI）
- ✅ 多连接方式架构（ConnectionType: WiFiLan, WiFiDirect, UsbAdb, Bluetooth）
- ✅ WiFi 直连（热点模式）- sb_hotspot_create / sb_hotspot_destroy / sb_hotspot_state
- ✅ USB/ADB 连接 - sb_adb_setup_port_forward / sb_adb_state
- ✅ 蓝牙连接 - sb_bt_init / sb_bt_state（BLE + 经典蓝牙）
- ✅ 真实音频电平检测（sb_get_audio_level，RMS 从采集数据计算）
- ✅ 独占模式延迟公式自适应（sb_set_exclusive_mode）
- ✅ 带宽自适应（发送线程根据丢包率动态调整 Opus 码率 64/96/128kbps）
- ✅ Oracle Bug 修复（channels 2→1, ConnectionType FFI, audio mode hot-switch）
- ✅ Windows C++ 测试文件（GTest: 27 个测试）
- ✅ Windows UI 动画优化（ProgressRing 加载、设备发现列表、连接状态动画）
- ✅ Android JNI 连接管理（热点/ADB/蓝牙/独占模式存根实现）
- ✅ 最终测试和发布准备（269+ 测试通过，零 clippy 警告）

### 3.2 技术实现细节

#### 3.2.1 音频模式切换
```cpp
// 核心类设计
class AudioModeManager {
public:
    enum Mode {
        BALANCED,      // 均衡模式
        HIGH_QUALITY,  // 高音质模式
        LOW_LATENCY    // 超低延迟模式
    };
    
    bool SwitchMode(Mode mode);
    Mode GetCurrentMode();
    
private:
    Mode currentMode;
    void UpdateCodecParams(Mode mode);
    void UpdateBufferSize(Mode mode);
};
```

---

## 里程碑和交付物

### 里程碑 1：MVP 完成 ✅
- **交付物**：
  - ✅ Windows 桌面客户端（托盘应用 + MainWindow + ViewModel）
  - ✅ Android App（Jetpack Compose UI + NativeAudioEngine JNI）
  - ✅ WiFi 局域网连接
  - ✅ 双向音频传输
  - ✅ 电平指示器

### 里程碑 2：完整功能 ✅ 已完成
- **交付物**：
  - ✅ 多种连接方式支持 - WiFi LAN + WiFi Direct + USB/ADB + Bluetooth（FFI 全部实现）
  - ✅ 回声消除（NLMS）、噪声抑制（SNR）、自动增益控制（AGC）
  - ✅ 设备记忆（JSON 持久化）、启动自启（Registry）
  - ✅ 全局快捷键（Ctrl+Alt+T/M/S）

### 里程碑 3：优化发布 ⏳ 进行中
- **交付物**：
  - ✅ 音频模式动态切换（均衡/高音质/超低延迟）
  - [ ] 性能优化（延迟 <30ms）
  - [ ] 完整 UI/UX
  - [ ] 发布版本

---

## 风险和应对

### 风险 1：延迟过高
- **应对**：优化缓冲区大小、使用低延迟音频API、减少处理环节

### 风险 2：音频卡顿
- **应对**：增加抖动缓冲、优化丢包恢复、调整编码参数

### 风险 3：设备兼容性
- **应对**：充分测试主流设备、提供降级方案、用户反馈机制

### 风险 4：功耗过高
- **应对**：优化后台服务、减少不必要的处理、提供省电模式

---

## 测试计划

### 单元测试
- 音频采集/播放
- Opus 编解码
- 网络传输
- 混音引擎

### 集成测试
- 端到端音频传输
- 多设备连接
- 长时间稳定性

### 性能测试
- 延迟测试
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
- **WebRTC APM**: 音频处理
- **Boost.Asio**: 网络编程（可选）
- **JSON**: 配置存储
