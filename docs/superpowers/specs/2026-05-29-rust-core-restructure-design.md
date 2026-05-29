# SoundBridge Rust 核心重构计划

> 按依赖层次推进，每个 Phase 独立可验证

## 已完成部分

- **audio-core**: `Sample` trait, `AudioBuffer<T>`, `AudioFormat`, `AudioError`
- **audio-codec**: `OpusEncoder`/`OpusDecoder`/`Codec`, 22 个单元测试, Criterion 基准测试

## Phase 1：音频采集/播放

**目标**: 实现跨平台音频输入输出，打通"声卡→内存"和"内存→声卡"通路。

**audio-capture**:
- 基于 cpal 0.15.2，`CaptureDevice` 封装 `cpal::Stream`
- 回调模式采集，通过 `crossbeam-channel` 传出 `AudioBuffer<f32>`
- 设备枚举：列出系统所有输入设备，支持按名称/ID 选择
- 格式协商：优先 48kHz/单声道/f32，不支持时自动转换
- 静默检测：连续 N 帧静音时触发回调通知

**audio-playback**:
- 基于 cpal，`PlaybackDevice` 封装输出流
- 从 channel 拉取 `AudioBuffer`，填充 cpal 回调缓冲区
- 简单线性重采样（48kHz→设备原生采样率）
- 欠载保护：缓冲区为空时输出静音，不爆音

**统一参数**:
- 内部格式：f32
- 采样率：48000 Hz
- 通道数：单声道（Mono）
- 帧大小：960 samples（20ms @ 48kHz）

**依赖**: cpal 0.15.2, crossbeam-channel 0.5, audio-core

**验收标准**:
- [ ] 能枚举系统音频设备
- [ ] 能采集麦克风音频并获得 `AudioBuffer<f32>`
- [ ] 能播放 `AudioBuffer<f32>` 到扬声器
- [ ] 设备格式不匹配时自动转换
- [ ] 单元测试 + 集成测试通过

## Phase 2：音频处理/混音

**目标**: 实现基础音频处理管线，为后续 WebRTC APM 集成预留接口。

**audio-processor**:
- 纯 Rust 实现，不依赖 WebRTC APM
- 增益控制（`GainProcessor`）：可调节音量，线性/对数曲线
- 静音检测（`SilenceDetector`）：基于能量阈值，返回静音状态
- 噪声门（`NoiseGate`）：低于阈值时衰减到静音，带 attack/release 时间
- 接口设计兼容后续替换为 WebRTC APM：`trait AudioProcessor { fn process(&mut self, input: &AudioBuffer<f32>) -> AudioBuffer<f32>; }`

**audio-mixer**:
- 纯 Rust 实现
- `mix(inputs: &[AudioBuffer<f32>], weights: &[f32]) -> AudioBuffer<f32>`
- 可配置混音比例（权重归一化）
- 防削波：soft clipping 或 hard limiting，输出不超过 0dB
- 支持 N 路输入混合（典型场景：游戏音频 + 手机音频）

**依赖**: audio-core, audio-codec（用于测试编解码后的混音效果）

**验收标准**:
- [ ] 增益调节后能量变化符合预期
- [ ] 静音检测准确率 > 95%
- [ ] 噪声门 attack/release 时间准确
- [ ] 混音后无削波失真
- [ ] 单元测试 + 集成测试通过

## Phase 3：协议/网络/设备发现

**目标**: 实现跨设备音频传输通路，打通"手机→电脑"音频链路。

**protocol**:
- 包格式定义：
  ```
  +----------+--------+---------+--------+---------+
  | 魔术数    | 版本   | 类型    | 序列号  | 时间戳  |
  | 4 bytes  | 1 byte | 1 byte  | 4 bytes| 8 bytes |
  +----------+--------+---------+--------+---------+
  | 载荷长度   | 载荷                           |
  | 4 bytes   | variable                       |
  +----------+----------------------------------+
  ```
- 魔术数：`0x53424447`（"SBDG" = SoundBridge DataGram）
- 包类型：AudioData(0x01), Control(0x02), Heartbeat(0x03), Ack(0x04)
- 序列化：serde + bincode，`bytes::Bytes` 零拷贝传输
- 版本兼容：低字节向前兼容，高字节不兼容时拒绝

**network**:
- 基于 tokio 1.35 异步运行时
- 发送链：`AudioBuffer<f32>` → Opus 编码 → 打包 → UDP 发送
- 接收链：UDP 接收 → 解包 → Opus 解码 → `AudioBuffer<f32>`
- 抖动缓冲（Jitter Buffer）：
  - 目标延迟：20-60ms 可配置
  - 自适应调整：根据网络延迟动态调整缓冲深度
  - 丢包补偿：重复上一帧或静音填充
- 心跳机制：每 5 秒发送 Heartbeat 包，超时 15 秒判定离线

**discovery**:
- 基于 mdns-sd 0.7 库
- 服务类型：`_soundbridge._udp.local.`
- 服务注册：启动时广播设备名称、IP、端口、支持的编解码格式
- 服务发现：监听局域网内其他 SoundBridge 设备
- 事件通知：设备上线/下线通过 channel 通知上层

**依赖**: tokio 1.35, mdns-sd 0.7, serde 1.0, bincode 1.3, bytes 1.5, audio-core, audio-codec

**验收标准**:
- [ ] 包序列化/反序列化正确
- [ ] 魔术数校验生效
- [ ] UDP 收发音频数据正常
- [ ] 抖动缓冲在 5% 丢包率下音频连续
- [ ] mDNS 能发现局域网内设备
- [ ] 单元测试 + 集成测试通过

## Phase 4：FFI 绑定

**目标**: 生成 C ABI，供 Windows C++ 和 Android JNI 调用。

**绑定策略**:
- cbindgen 自动生成 C 头文件（`soundbridge.h`）
- 句柄模式：所有对象通过 `*mut c_void` 句柄传递，外部无法直接操作内部结构
- 错误处理：返回 `i32` 状态码（0=成功，负数=错误），`sb_last_error()` 获取错误信息
- 线程安全：所有公开 API 均为线程安全（内部加锁或使用原子操作）

**API 清单**:
```c
// 引擎生命周期
int sb_audio_engine_create(void** handle);
int sb_audio_engine_destroy(void* handle);

// 采集
int sb_capture_enumerate(void* handle, DeviceInfo* devices, int* count);
int sb_capture_open(void* handle, const char* device_id, void** capture);
int sb_capture_start(void* capture);
int sb_capture_stop(void* capture);
int sb_capture_read(void* capture, float* buffer, int samples);

// 播放
int sb_playback_open(void* handle, const char* device_id, void** playback);
int sb_playback_start(void* playback);
int sb_playback_stop(void* playback);
int sb_playback_write(void* playback, const float* buffer, int samples);

// 混音
int sb_mixer_create(void** mixer);
int sb_mixer_add_input(void* mixer, void* input, float weight);
int sb_mixer_mix(void* mixer, float* output, int samples);

// 处理器
int sb_processor_gain_create(float db, void** processor);
int sb_processor_process(void* processor, const float* input, float* output, int samples);

// 网络
int sb_network_create(void** network);
int sb_network_send(void* network, const float* buffer, int samples);
int sb_network_recv(void* network, float* buffer, int* samples);

// 设备发现
int sb_discovery_create(void** discovery);
int sb_discovery_start(void* discovery);
int sb_discovery_get_devices(void* discovery, DeviceInfo* devices, int* count);

// 错误
const char* sb_last_error();
```

**依赖**: cbindgen 0.26, 所有 Phase 1-3 的 crate

**验收标准**:
- [ ] cbindgen 生成的头文件编译通过
- [ ] Windows C++ 能调用所有 API
- [ ] Android JNI 能调用所有 API
- [ ] 句柄泄漏检测通过（每个 create 都有对应 destroy）
- [ ] 多线程并发调用不死锁

## Phase 6：集成验证

**目标**: 端到端验证整个音频链路，确保满足性能和稳定性要求。

**端到端链路测试**:
- 采集 → 编码 → 网络发送 → 网络接收 → 解码 → 播放
- 两台设备间音频往返测试
- 模拟弱网环境（丢包、延迟、抖动）

**性能基准**:
| 指标 | 目标 | 测量方法 |
|------|------|----------|
| 端到端延迟 | < 30ms | 发送脉冲，测量接收时间差 |
| CPU 占用 | < 5% | 单核占用，持续音频流 |
| 内存占用 | < 50MB | 运行时 RSS |
| 编码延迟 | < 5ms | 单帧编码耗时 |
| 解码延迟 | < 5ms | 单帧解码耗时 |

**稳定性测试**:
- 1 小时连续运行，无内存泄漏、无崩溃
- 音频连续性：无卡顿、无爆音
- 设备热插拔：拔掉耳机后自动切换到扬声器

**验收标准**:
- [ ] 端到端延迟 < 30ms
- [ ] CPU 占用 < 5%
- [ ] 内存占用 < 50MB
- [ ] 1 小时稳定性测试通过
- [ ] 所有基准测试记录到 `benches/` 目录

## 并行执行策略

```
Phase 1 (采集/播放) ──┐
Phase 2 (处理/混音) ──┼──→ Phase 4 (FFI) ──→ Phase 6 (集成验证)
Phase 3 (协议/网络) ──┘
```

Phase 1/2/3 无相互依赖，可并行开发。Phase 4 依赖所有前置 Phase。Phase 6 为最终验证。

## 质量标准

每个 crate 交付时必须满足：

1. **AI_GUIDE.md 更新**：记录当前状态、API 概览、下一步计划
2. **单元测试**：核心逻辑覆盖率 > 80%
3. **集成测试**：跨 crate 交互测试
4. **clippy 通过**：`cargo clippy -p <crate> -- -D warnings`
5. **fmt 通过**：`cargo fmt -p <crate> -- --check`
6. **doc comment**：所有公开 API 必须有文档注释
7. **CHANGELOG**：记录每个版本的变更

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| cpal 不支持某些音频设备 | 采集/播放失败 | 提供 fallback 设备列表，优雅降级 |
| mDNS 在企业网络被禁用 | 设备发现失败 | 提供手动 IP 输入作为备选 |
| Opus 编码延迟超标 | 端到端延迟超标 | 调整帧大小，使用低延迟模式 |
| FFI 边界内存安全 | 崩溃/数据损坏 | 严格句柄校验，Miri 检测 UB |

## 依赖版本锁定

```toml
[workspace.dependencies]
# 核心
cpal = "0.15.2"
crossbeam-channel = "0.5"

# 音频处理
# (纯 Rust 实现，无外部依赖)

# 网络
tokio = { version = "1.35", features = ["full"] }
mdns-sd = "0.7"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
bytes = "1.5"

# FFI
cbindgen = "0.26"

# 测试
criterion = { version = "0.5", features = ["html_reports"] }
```

## 附录 A：线程模型与实时性约束

音频管线分为实时线程和非实时线程：

实时线程（采集/播放回调）：
- 锁定高优先级（Windows AVRT / Android SCHED_FIFO）
- 禁止加锁、禁止内存分配、禁止 I/O
- 通过 lock-free ring buffer 与处理线程通信

非实时线程（处理/编解码/网络）：
- 普通优先级
- 可以加锁、分配内存、做 I/O
- 所有音频 buffer 启动时预分配

AEC 特殊要求：
- AEC 处理需要同时访问采集信号和播放信号
- 播放信号作为回声参考，从播放回调中获取
- AEC 处理点在采集线程和播放线程的交汇处

## 附录 B：Jitter Buffer 设计

核心功能：
- 按序列号排序到达的音频包
- 可配置缓冲延迟（默认 40ms，最小 20ms，最大 200ms）
- 动态调整：根据网络抖动自动伸缩缓冲区大小
- 丢包检测：通过序列号间隙检测丢包
- PLC 补偿：短期丢包（1-2 帧）用前一帧衰减填补，长期丢包插入静音

## 附录 C：连接协议设计

握手流程：
1. DISCOVER_REQUEST (UDP 广播) → 设备名、版本
2. DISCOVER_RESPONSE (UDP 单播) → IP、端口、能力
3. HANDSHAKE_REQUEST (TCP) → 协议版本、音频参数、加密能力
4. HANDSHAKE_RESPONSE (TCP) → 确认/拒绝、协商后参数
5. START_STREAM (TCP) → Opus 配置、序列号起始值
6. START_STREAM_ACK (TCP)
7. AUDIO_DATA (UDP) → 双向音频流
8. CONTROL (TCP) → 模式切换、参数调整

包类型：
- 0x01 DISCOVER_REQUEST, 0x02 DISCOVER_RESPONSE
- 0x10 HANDSHAKE_REQUEST, 0x11 HANDSHAKE_RESPONSE
- 0x20 START_STREAM, 0x21 START_STREAM_ACK
- 0x30 AUDIO_DATA, 0x31 AUDIO_RTCP
- 0x40 CONTROL_MODE_SWITCH, 0x41 CONTROL_PARAM_UPDATE
- 0xF0 HEARTBEAT, 0xF1 HEARTBEAT_ACK

音频数据包格式（二进制）：
- Magic: 0x53424447 (4 bytes)
- Version: 0x01 (1 byte)
- Type: 0x30 (1 byte)
- Sequence Number (4 bytes)
- Timestamp in μs (4 bytes)
- Payload Length (2 bytes)
- Opus Frame Count (2 bytes)
- Opus Data (variable)

## 附录 D：平台特定约束

Windows：
- WASAPI loopback 模式采集系统音频（AUDCLNT_STREAMFLAGS_LOOPBACK）
- 音频线程优先级：AVRT API 设置 AVRT_PRIORITY_CRITICAL
- 全局快捷键：RegisterHotKey API
- 系统托盘：Shell_NotifyIcon API

Android：
- 内部音频采集（最大风险）：
  - Android 10+ AudioPlaybackCapture API 需要 MEDIA_CONTENT_CONTROL 权限（系统级）
  - Fallback: MediaProjection（需要用户授权弹窗）
  - MVP 先用 MediaProjection，后续探索系统级方案
- 音频焦点：AudioManager.requestAudioFocus() 协调与其他 App
- 后台限制：前台服务 + FOREGROUND_SERVICE_TYPE_MICROPHONE
- 设备碎片化：不同厂商 AAudio 实现质量不同，需要 fallback 到 AudioTrack

## 附录 E：时钟同步

问题：两台设备的系统时钟不一致，长时间运行后音频会"错位"。

方案：
- 握手时交换时间戳，计算 RTT 和时钟偏移
- 运行时用心跳包定期校准（每 5 秒一次）
- 检测到漂移时通过重采样微调播放速度（±0.1%）

## 附录 F：降级策略

| 场景 | 检测方式 | 降级动作 |
|------|---------|---------|
| 网络抖动增大 | jitter buffer 统计 | 自动增大缓冲延迟 |
| 丢包率 >5% | 序列号间隙检测 | 启用 FEC 冗余包 |
| CPU 过载 | 处理线程耗时统计 | 降低编码复杂度、关闭 AEC |
| 带宽不足 | 发送队列积压 | 降低 Opus 比特率 |
| 设备断开 | 心跳超时 | 自动重连，播放静音 |

## 附录 G：热插拔处理

- 耳机插拔：检测音频设备变化，自动切换到新设备
- 网络变化：WiFi → 4G 切换时保持连接或优雅断开
- USB 设备：检测 USB 音频设备连接/断开

## 附录 H：安全模型

| 威胁 | MVP 方案 | 完整方案 |
|------|---------|---------|
| 未授权设备连接 | PIN 码配对 | TLS 证书验证 |
| 音频窃听 | 不加密（局域网） | DTLS 加密音频流 |
| 中间人攻击 | 不防御 | 证书固定 |
| DoS 攻击 | 速率限制 | 连接频率限制 |

## 附录 I：测试策略

| 类型 | 工具 | 覆盖范围 |
|------|------|---------|
| 单元测试 | cargo test | 每个 crate 核心逻辑 |
| 集成测试 | cargo test --workspace | crate 间交互 |
| 性能基准 | Criterion | 编解码、混音、网络吞吐 |
| 延迟测试 | 自定义工具 | 端到端延迟测量 |
| 稳定性测试 | 长时间运行脚本 | 1 小时无崩溃 |
| Fuzz 测试 | cargo-fuzz | protocol 序列化/反序列化 |
| 设备测试 | 真实设备 | 不同 Android 机型、Windows 版本 |
| 网络模拟 | clumsy (Windows) | 模拟丢包、延迟、抖动 |

## 附录 J：依赖许可证审计

| 依赖 | 用途 | 许可证 | 风险 |
|------|------|--------|------|
| cpal | 跨平台音频 | Apache-2.0 | 低 |
| opus | 编解码 | BSD-3 | 低 |
| tokio | 异步运行时 | MIT | 低 |
| quinn | QUIC | MIT/Apache-2.0 | 中 |
| mdns-sd | 设备发现 | MIT/Apache-2.0 | 低 |
| WebRTC APM | 音频处理 | BSD-3 | 中（C++ 依赖） |
| serde + bincode | 序列化 | MIT/Apache-2.0 | 低 |
| bytes | 零拷贝 | MIT | 低 |
| crossbeam | lock-free | MIT/Apache-2.0 | 低 |
| tracing | 日志 | MIT | 低 |
