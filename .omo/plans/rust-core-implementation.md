# SoundBridge Rust 核心实施计划

> Sisyphus 执行手册。按顺序执行，遇到问题参考 context.md。

---

## 已完成（跳过）

- `audio-core`：基础类型（Sample、AudioBuffer、AudioFormat、AudioError）
- `audio-codec`：Opus 编解码（22 个测试、Criterion 基准）

---

## Phase 0：技术 Spike（验证最大风险）

### Spike 1：cpal 延迟测试

**目标**：验证 cpal 在 Windows 上的延迟是否 <30ms

**步骤**：
1. 创建临时项目 `rust-core/spikes/cpal-latency/`
2. 用 cpal 打开默认输入设备（麦克风）
3. 用 cpal 打开默认输出设备（扬声器）
4. 采集 → 直接播放，测量环回延迟
5. 测试不同 buffer size（480/960/1920 samples）
6. 记录延迟数据

**成功标准**：buffer size 960 时延迟 <50ms

**失败处理**：参考 context.md "cpal 延迟过高" 决策树

**文件**：
- `rust-core/spikes/cpal-latency/Cargo.toml`
- `rust-core/spikes/cpal-latency/src/main.rs`

---

### Spike 2：WebRTC APM 跨编译验证

**目标**：验证能否在 Rust 中调用 WebRTC APM C++ 代码

**步骤**：
1. 创建临时项目 `rust-core/spikes/webrtc-apm/`
2. 编写 build.rs：用 cmake 编译 WebRTC APM C++ 代码
3. 用 bindgen 生成 Rust 绑定
4. 调用一个简单的 APM 函数（如 `AudioProcessing::Create`）
5. 验证 Windows 和 Android 交叉编译

**成功标准**：能在 Rust 中调用 WebRTC APM 并返回结果

**失败处理**：参考 context.md "WebRTC APM 编译失败" 决策树

**文件**：
- `rust-core/spikes/webrtc-apm/Cargo.toml`
- `rust-core/spikes/webrtc-apm/build.rs`
- `rust-core/spikes/webrtc-apm/src/lib.rs`
- `rust-core/spikes/webrtc-apm/cpp/`（C++ wrapper）

---

### Spike 3：Android 内部音频采集 PoC

**目标**：验证 MediaProjection 能否采集其他 App 音频

**步骤**：
1. 创建 Android 测试项目
2. 实现 MediaProjection 请求流程
3. 用 AudioPlaybackCapture API 采集内部音频
4. 验证采集到的 PCM 数据是否正确
5. 测试不同 Android 版本（10、11、12、13、14）

**成功标准**：能在 Android 10+ 上采集到其他 App 的音频

**失败处理**：参考 context.md "Android 内部音频采集失败" 决策树

**注意**：这个 Spike 需要在 Android 设备上测试，不是 Rust 代码

---

## Phase 1：音频采集 / 播放

### Task 1.1：实现 audio-capture crate

**依赖**：Phase 0 Spike 1 通过

**创建文件**：
- `rust-core/crates/audio-capture/src/lib.rs`（重写）
- `rust-core/crates/audio-capture/src/device.rs`（设备枚举）
- `rust-core/crates/audio-capture/src/config.rs`（配置）
- `rust-core/crates/audio-capture/tests/capture_test.rs`

**核心类型**：
```rust
pub struct CaptureDevice { ... }
pub struct CaptureConfig {
    pub sample_rate: u32,      // 默认 48000
    pub channels: u16,         // 默认 1（单声道）
    pub buffer_size: u32,      // 默认 960（20ms）
}
```

**核心函数**：
```rust
impl CaptureDevice {
    pub fn list_devices() -> Result<Vec<DeviceInfo>>
    pub fn new(device: &DeviceInfo, config: CaptureConfig) -> Result<Self>
    pub fn start(&mut self) -> Result<()>
    pub fn stop(&mut self) -> Result<()>
    pub fn read(&mut self) -> Result<AudioBuffer<f32>>
}
```

**实现细节**：
- 用 cpal 打开音频流
- 回调模式：采集到数据后写入 lock-free ring buffer
- `read()` 从 ring buffer 取数据
- 统一输出 f32 格式，48kHz，单声道

**测试用例**：
- 列出设备不为空
- 创建设备成功
- 采集到的 buffer 格式正确（48kHz、单声道、f32）
- 采集到的数据不全为零（需要真实音频输入）

**验收标准**：
- `cargo test -p audio-capture` 通过
- `cargo clippy -p audio-capture` 无 warning
- AI_GUIDE.md 更新为"已完成"

---

### Task 1.2：实现 audio-playback crate

**依赖**：Phase 0 Spike 1 通过

**创建文件**：
- `rust-core/crates/audio-playback/src/lib.rs`（重写）
- `rust-core/crates/audio-playback/src/device.rs`（设备枚举）
- `rust-core/crates/audio-playback/src/resample.rs`（简单重采样）
- `rust-core/crates/audio-playback/tests/playback_test.rs`

**核心类型**：
```rust
pub struct PlaybackDevice { ... }
pub struct PlaybackConfig {
    pub sample_rate: u32,      // 默认 48000
    pub channels: u16,         // 默认 1
    pub buffer_size: u32,      // 默认 960
}
```

**核心函数**：
```rust
impl PlaybackDevice {
    pub fn list_devices() -> Result<Vec<DeviceInfo>>
    pub fn new(device: &DeviceInfo, config: PlaybackConfig) -> Result<Self>
    pub fn start(&mut self) -> Result<()>
    pub fn stop(&mut self) -> Result<()>
    pub fn write(&mut self, buffer: &AudioBuffer<f32>) -> Result<()>
}
```

**实现细节**：
- 用 cpal 打开音频流
- 回调模式：从 ring buffer 取数据送给设备
- `write()` 往 ring buffer 写数据
- 如果输入采样率和设备采样率不同，做线性插值重采样

**测试用例**：
- 列出设备不为空
- 创建设备成功
- 播放无杂音（需要人工验证或自动化测试）
- 重采样后数据长度正确

**验收标准**：
- `cargo test -p audio-playback` 通过
- `cargo clippy -p audio-playback` 无 warning
- AI_GUIDE.md 更新

---

### Task 1.3：lock-free ring buffer 实现

**依赖**：无（可与 Task 1.1、1.2 并行）

**创建文件**：
- `rust-core/crates/audio-core/src/ring_buffer.rs`

**核心类型**：
```rust
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
    capacity: usize,
}
```

**核心函数**：
```rust
impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self
    pub fn write(&self, data: &[T]) -> usize  // 返回实际写入数量
    pub fn read(&self, output: &mut [T]) -> usize  // 返回实际读取数量
    pub fn available_read(&self) -> usize
    pub fn available_write(&self) -> usize
}
```

**实现细节**：
- 单生产者单消费者（SPSC）lock-free
- 用 AtomicUsize 做读写位置
- 缓冲区大小必须是 2 的幂（方便取模）

**测试用例**：
- 写入后读取得到相同数据
- 缓冲区满时写入返回 0
- 缓冲区空时读取返回 0
- 并发读写不崩溃

**验收标准**：
- `cargo test -p audio-core` 通过（新增 ring_buffer 测试）
- 无锁、无 panic

---

## Phase 2：音频处理 / 混音

### Task 2.1：实现 audio-mixer crate

**依赖**：Task 1.3（ring buffer）

**创建文件**：
- `rust-core/crates/audio-mixer/src/lib.rs`（重写）
- `rust-core/crates/audio-mixer/src/mixer.rs`（混音逻辑）
- `rust-core/crates/audio-mixer/tests/mixer_test.rs`

**核心类型**：
```rust
pub struct MixerConfig {
    pub output_sample_rate: u32,
    pub output_channels: u16,
    pub clipping_protection: bool,  // 默认 true
}

pub struct AudioMixer {
    config: MixerConfig,
}
```

**核心函数**：
```rust
impl AudioMixer {
    pub fn new(config: MixerConfig) -> Result<Self>
    pub fn mix(&self, inputs: &[&AudioBuffer<f32>], volumes: &[f32]) -> Result<AudioBuffer<f32>>
}
```

**实现细节**：
- 加权求和：`output[i] = sum(input[j][i] * volume[j])`
- 防削波：如果输出 >1.0 或 <-1.0，做 soft clipping（tanh 压缩）
- 如果输入采样率不同，先重采样到统一格式
- 用 SIMD 加速求和（可选优化）

**测试用例**：
- 单路输入，音量 1.0，输出等于输入
- 两路输入，音量各 0.5，输出为平均值
- 音量 0.0，输出为静音
- 输入采样率不匹配时自动重采样
- 防削波：大信号不会溢出

**验收标准**：
- `cargo test -p audio-mixer` 通过
- `cargo clippy -p audio-mixer` 无 warning
- AI_GUIDE.md 更新

---

### Task 2.2：实现 audio-processor crate（基础版）

**依赖**：无

**创建文件**：
- `rust-core/crates/audio-processor/src/lib.rs`（重写）
- `rust-core/crates/audio-processor/src/gain.rs`（增益控制）
- `rust-core/crates/audio-processor/src/silence_detector.rs`（静音检测）
- `rust-core/crates/audio-processor/src/noise_gate.rs`（噪声门）
- `rust-core/crates/audio-processor/tests/processor_test.rs`

**核心类型**：
```rust
pub struct ProcessorConfig {
    pub gain_db: f32,           // 默认 0.0
    pub silence_threshold_db: f32,  // 默认 -60.0
    pub noise_gate_threshold_db: f32,  // 默认 -50.0
}

pub struct AudioProcessor {
    config: ProcessorConfig,
}
```

**核心函数**：
```rust
impl AudioProcessor {
    pub fn new(config: ProcessorConfig) -> Result<Self>
    pub fn process(&self, buffer: &mut AudioBuffer<f32>) -> Result<()>
    pub fn is_silence(&self, buffer: &AudioBuffer<f32>) -> bool
}
```

**实现细节**：
- 增益控制：`sample *= 10^(gain_db/20)`
- 静音检测：计算 RMS，与阈值比较
- 噪声门：低于阈值的信号置零

**测试用例**：
- 增益 0dB，输出等于输入
- 增益 6dB，输出幅度翻倍
- 静音信号检测为静音
- 噪声门过滤低幅度信号

**验收标准**：
- `cargo test -p audio-processor` 通过
- `cargo clippy -p audio-processor` 无 warning
- AI_GUIDE.md 更新

---

## Phase 3：协议 / 网络 / 设备发现

### Task 3.1：实现 protocol crate

**依赖**：无

**创建文件**：
- `rust-core/crates/protocol/src/lib.rs`（重写）
- `rust-core/crates/protocol/src/packet.rs`（包格式）
- `rust-core/crates/protocol/src/handshake.rs`（握手协议）
- `rust-core/crates/protocol/src/error.rs`（错误类型）
- `rust-core/crates/protocol/tests/protocol_test.rs`

**核心类型**：
```rust
pub const MAGIC: u32 = 0x53424447;  // "SBDG"

pub struct PacketHeader {
    pub magic: u32,
    pub version: u8,
    pub packet_type: PacketType,
    pub sequence: u32,
    pub timestamp_us: u64,
    pub payload_length: u16,
}

pub enum PacketType {
    DiscoverRequest = 0x01,
    DiscoverResponse = 0x02,
    HandshakeRequest = 0x10,
    HandshakeResponse = 0x11,
    StartStream = 0x20,
    StartStreamAck = 0x21,
    AudioData = 0x30,
    AudioRtcp = 0x31,
    ControlModeSwitch = 0x40,
    ControlParamUpdate = 0x41,
    Heartbeat = 0xF0,
    HeartbeatAck = 0xF1,
}

pub struct AudioPacket {
    pub header: PacketHeader,
    pub opus_frames: Vec<Vec<u8>>,
}

pub struct HandshakeRequest {
    pub protocol_version: u16,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_size: u32,
    pub opus_bitrate: u32,
}
```

**核心函数**：
```rust
impl PacketHeader {
    pub fn encode(&self, buf: &mut Vec<u8>) -> Result<()>
    pub fn decode(buf: &[u8]) -> Result<(PacketHeader, usize)>
}

impl AudioPacket {
    pub fn encode(&self) -> Result<Vec<u8>>
    pub fn decode(data: &[u8]) -> Result<AudioPacket>
}

impl HandshakeRequest {
    pub fn encode(&self) -> Result<Vec<u8>>
    pub fn decode(data: &[u8]) -> Result<HandshakeRequest>
}
```

**实现细节**：
- 用 bincode 序列化（serde）
- 包头固定 18 字节
- 校验魔术数和版本号

**测试用例**：
- 编码后解码得到原始数据
- 魔术数不匹配返回错误
- 载荷长度超限返回错误
- 握手请求编解码正确

**验收标准**：
- `cargo test -p protocol` 通过
- `cargo clippy -p protocol` 无 warning
- AI_GUIDE.md 更新

---

### Task 3.2：实现 network crate

**依赖**：Task 3.1（protocol）、Task 1.3（ring buffer）

**创建文件**：
- `rust-core/crates/network/src/lib.rs`（重写）
- `rust-core/crates/network/src/udp_transport.rs`（UDP 传输）
- `rust-core/crates/network/src/tcp_transport.rs`（TCP 控制）
- `rust-core/crates/network/src/jitter_buffer.rs`（抖动缓冲）
- `rust-core/crates/network/tests/network_test.rs`

**核心类型**：
```rust
pub struct UdpTransport {
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
}

pub struct JitterBuffer {
    buffer: Vec<Option<AudioPacket>>,
    base_sequence: u32,
    target_delay_ms: u32,  // 默认 40ms
    min_delay_ms: u32,     // 20ms
    max_delay_ms: u32,     // 200ms
}

pub struct NetworkConfig {
    pub bind_addr: SocketAddr,
    pub target_delay_ms: u32,
}
```

**核心函数**：
```rust
impl UdpTransport {
    pub async fn new(bind_addr: SocketAddr) -> Result<Self>
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<()>
    pub async fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>
}

impl JitterBuffer {
    pub fn new(config: JitterBufferConfig) -> Self
    pub fn push(&mut self, packet: AudioPacket)
    pub fn pop(&mut self) -> Option<AudioPacket>
    pub fn adjust_delay(&mut self, jitter_ms: u32)
}
```

**实现细节**：
- UDP 用 tokio 异步 I/O
- Jitter Buffer 按序列号排序，定时取出
- 动态调整缓冲延迟

**测试用例**：
- UDP 发送接收正确
- Jitter Buffer 乱序包重排正确
- Jitter Buffer 丢包检测正确
- 延迟动态调整正确

**验收标准**：
- `cargo test -p network` 通过
- `cargo clippy -p network` 无 warning
- AI_GUIDE.md 更新

---

### Task 3.3：实现 discovery crate

**依赖**：无

**创建文件**：
- `rust-core/crates/discovery/src/lib.rs`（重写）
- `rust-core/crates/discovery/src/mdns.rs`（mDNS 实现）
- `rust-core/crates/discovery/tests/discovery_test.rs`

**核心类型**：
```rust
pub struct DeviceInfo {
    pub name: String,
    pub ip: IpAddr,
    pub port: u16,
    pub device_type: DeviceType,
}

pub enum DeviceType {
    Windows,
    Android,
    Unknown,
}

pub struct DiscoveryService { ... }
```

**核心函数**：
```rust
impl DiscoveryService {
    pub fn new(service_name: &str, port: u16) -> Result<Self>
    pub fn register(&self) -> Result<()>
    pub fn scan(&self, timeout_ms: u32) -> Result<Vec<DeviceInfo>>
    pub fn unregister(&self) -> Result<()>
}
```

**实现细节**：
- 用 mdns-sd 库
- 服务类型：`_soundbridge._udp.local.`
- 注册时广播设备名、IP、端口
- 扫描时返回局域网内所有 SoundBridge 设备

**测试用例**：
- 注册后能被扫描到
- 扫描超时返回空列表
- 注销后不再被扫描到

**验收标准**：
- `cargo test -p discovery` 通过
- `cargo clippy -p discovery` 无 warning
- AI_GUIDE.md 更新

---

## Phase 4：FFI 绑定

### Task 4.1：实现 ffi-bindings crate

**依赖**：Phase 1-3 全部完成

**创建文件**：
- `rust-core/crates/ffi-bindings/src/lib.rs`（重写）
- `rust-core/crates/ffi-bindings/src/engine.rs`（引擎 API）
- `rust-core/crates/ffi-bindings/src/capture.rs`（采集 API）
- `rust-core/crates/ffi-bindings/src/playback.rs`（播放 API）
- `rust-core/crates/ffi-bindings/src/mixer.rs`（混音 API）
- `rust-core/crates/ffi-bindings/src/network.rs`（网络 API）
- `rust-core/crates/ffi-bindings/src/error.rs`（错误处理）
- `rust-core/crates/ffi-bindings/build.rs`（cbindgen）
- `rust-core/crates/ffi-bindings/tests/ffi_test.rs`

**核心 API**：
```rust
// 引擎
pub extern "C" fn sb_engine_create() -> *mut c_void
pub extern "C" fn sb_engine_destroy(engine: *mut c_void)

// 采集
pub extern "C" fn sb_capture_start(engine: *mut c_void, device_name: *const c_char) -> i32
pub extern "C" fn sb_capture_stop(engine: *mut c_void) -> i32
pub extern "C" fn sb_capture_read(engine: *mut c_void, buf: *mut f32, len: usize) -> i32

// 播放
pub extern "C" fn sb_playback_start(engine: *mut c_void, device_name: *const c_char) -> i32
pub extern "C" fn sb_playback_stop(engine: *mut c_void) -> i32
pub extern "C" fn sb_playback_write(engine: *mut c_void, buf: *const f32, len: usize) -> i32

// 混音
pub extern "C" fn sb_mixer_mix(engine: *mut c_void, inputs: *const *const f32, input_lens: *const usize, volumes: *const f32, input_count: usize, output: *mut f32, output_len: usize) -> i32

// 网络
pub extern "C" fn sb_network_send(engine: *mut c_void, data: *const u8, len: usize, addr: *const c_char) -> i32
pub extern "C" fn sb_network_receive(engine: *mut c_void, buf: *mut u8, len: usize) -> i32

// 错误
pub extern "C" fn sb_last_error() -> *const c_char
```

**实现细节**：
- 用 cbindgen 自动生成 C 头文件
- 句柄模式：所有 API 通过 `*mut c_void` 传递
- 错误处理：返回 -1 表示错误，`sb_last_error()` 获取错误信息
- 线程安全：所有 API 可从任意线程调用

**测试用例**：
- 创建/销毁引擎不崩溃
- 采集/播放 API 调用正确
- 错误信息正确返回

**验收标准**：
- `cargo test -p ffi-bindings` 通过
- `cargo clippy -p ffi-bindings` 无 warning
- cbindgen 生成的头文件正确
- AI_GUIDE.md 更新

---

## Phase 5：集成验证

### Task 5.1：端到端集成测试

**依赖**：Phase 4 完成

**创建文件**：
- `rust-core/tests/integration_test.rs`

**测试场景**：
1. 采集 → 编码 → 解码 → 播放（本地环回）
2. 采集 → 编码 → 网络发送 → 接收 → 解码 → 播放（双线程模拟）
3. 两路音频混音后播放
4. 设备发现 → 连接 → 音频传输

**验收标准**：
- 所有集成测试通过
- 无 panic、无内存泄漏

---

### Task 5.2：性能基准测试

**依赖**：Task 5.1

**创建文件**：
- `rust-core/benches/pipeline_benchmark.rs`

**基准测试**：
- 采集 → 编码 → 解码 → 播放 端到端延迟
- 混音吞吐量（每秒处理多少帧）
- 网络吞吐量（每秒发送多少包）
- 内存占用

**验收标准**：
- 端到端延迟 <30ms（超低延迟模式）
- CPU <5%（空闲）、<15%（传输中）
- 内存 <50MB

---

### Task 5.3：长时间稳定性测试

**依赖**：Task 5.2

**步骤**：
1. 运行端到端测试 1 小时
2. 监控内存占用、CPU 使用率
3. 检查是否有崩溃、内存泄漏

**验收标准**：
- 1 小时无崩溃
- 内存无持续增长
- CPU 使用率稳定

---

## 执行顺序

```
Phase 0（并行）
├── Spike 1: cpal 延迟测试
├── Spike 2: WebRTC APM 跨编译
└── Spike 3: Android 内部音频 PoC

Phase 1（并行，依赖 Spike 1 通过）
├── Task 1.1: audio-capture
├── Task 1.2: audio-playback
└── Task 1.3: ring buffer

Phase 2（并行，依赖 Phase 1）
├── Task 2.1: audio-mixer
└── Task 2.2: audio-processor（基础版）

Phase 3（并行，依赖 Phase 1）
├── Task 3.1: protocol
├── Task 3.2: network（依赖 3.1）
└── Task 3.3: discovery

Phase 4（依赖 Phase 1-3）
└── Task 4.1: ffi-bindings

Phase 5（依赖 Phase 4）
├── Task 5.1: 集成测试
├── Task 5.2: 性能基准
└── Task 5.3: 稳定性测试
```

---

## 质量检查点

每个 Task 完成后：
- [ ] `cargo test -p {crate}` 通过
- [ ] `cargo clippy -p {crate}` 无 warning
- [ ] `cargo fmt -- --check` 通过
- [ ] AI_GUIDE.md 更新
- [ ] 公开 API 有 doc comment

每个 Phase 完成后：
- [ ] `cargo test --workspace` 通过
- [ ] `cargo clippy --workspace` 无 warning
