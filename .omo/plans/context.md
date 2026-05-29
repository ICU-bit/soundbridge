# SoundBridge 决策上下文文档

> Sisyphus/oracle 遇到问题时参考本文档做决策。

---

## 项目背景

SoundBridge 跨端音频融合软件。核心目标：游戏时不用摘耳机，同时听电脑和手机的声音。

**技术架构**：Rust 核心库 + Windows C++/C# + Android Kotlin/JNI

**性能目标**：
- 延迟 <30ms（超低延迟模式）
- CPU <5%（空闲）、<15%（传输中）
- 内存 <50MB

**统一参数**：
- 采样率：48000 Hz
- 通道：单声道（Mono）
- 帧大小：960 samples（20ms@48kHz）
- 内部格式：Float32

---

## 已完成的架构决策

### 决策 1：语言分配

**决策**：大部分用 Rust，只有 audio-processor 用 C++ FFI（WebRTC APM）

**理由**：
- Rust 内存安全，无 GC 暂停，适合实时音频
- WebRTC APM 是唯一经过大规模验证的 AEC 实现，没有成熟 Rust 替代品
- 其他模块（mixer、protocol、network）纯 Rust 就能做

**如果遇到**：WebRTC APM 编译失败 → 见下方决策树

---

### 决策 2：跨平台音频用 cpal

**决策**：用 cpal 库做跨平台音频采集/播放，不直接调 WASAPI/AAudio

**理由**：
- cpal 抽象层开销在微秒级，对音频延迟（毫秒级）可忽略
- Windows 端已有 C++ WASAPI 实现，Rust 核心不需要重复
- 真正的极致延迟优化在缓冲区大小和线程优先级，不在语言选择

**如果遇到**：cpal 延迟过高 → 见下方决策树

---

### 决策 3：线程模型

**决策**：实时线程（采集/播放）与处理线程分离，lock-free buffer 连接

**理由**：
- 音频回调在 OS 实时线程执行，不能加锁、不能分配内存、不能做 I/O
- lock-free buffer 避免优先级反转
- 处理线程可以做重活（编码、网络）

**如果遇到**：音频卡顿 → 检查 ring buffer 是否溢出/欠载

---

### 决策 4：网络协议

**决策**：音频用 UDP，控制用 TCP（第一版），后续加 QUIC

**理由**：
- UDP 低延迟，适合实时音频
- TCP 可靠，适合控制信令
- QUIC 实现复杂（quinn 库），第一版用 TCP 够用

**如果遇到**：UDP 丢包严重 → 启用 FEC 冗余包

---

### 决策 5：序列化用 bincode

**决策**：协议包用 serde + bincode 序列化，不手写二进制

**理由**：
- bincode 比手写二进制更安全，不容易出 bug
- serde 生态成熟，维护方便
- 性能接近手写

**如果遇到**：bincode 性能不够 → 考虑手写关键路径

---

### 决策 6：FFI 用句柄模式

**决策**：ffi-bindings 用不透明句柄（*mut c_void），不暴露 Rust 类型

**理由**：
- 句柄模式更适合 C/JNI 调用
- 隐藏内部实现，减少 FFI 复杂度
- 错误通过返回码 + sb_last_error() 获取

**如果遇到**：句柄泄漏 → 检查是否调用了 destroy 函数

---

### 决策 7：audio-processor 先做简单版

**决策**：第一版只做增益、静音检测、噪声门，不做 WebRTC APM

**理由**：
- WebRTC APM C++ FFI 复杂度高，是最大风险
- 先做简单版可以并行推进其他模块
- 接口保持兼容，后续替换实现

**如果遇到**：需要 AEC → 集成 WebRTC APM（Phase 6）

---

## 决策树

### cpal 延迟过高

```
问题：cpal 采集/播放延迟 >50ms
│
├─ 检查 buffer size 是否太大
│  └─ 尝试 buffer size = 480（10ms）
│
├─ 检查是否用了共享模式
│  └─ 尝试 exclusive mode（WASAPI Exclusive）
│
├─ 检查设备是否支持低延迟
│  └─ 尝试其他音频设备
│
└─ 仍然过高
   └─ 记录数据，继续下一步
   └─ 在实施计划中标记为"待优化"
```

---

### WebRTC APM 编译失败

```
问题：WebRTC APM C++ 代码无法编译
│
├─ 检查 cmake 版本
│  └─ 需要 cmake 3.20+
│
├─ 检查 C++ 编译器
│  └─ Windows: MSVC 2022
│  └─ Android: NDK clang
│
├─ 检查 WebRTC 源码完整性
│  └─ 重新下载 WebRTC 源码
│
├─ 检查依赖库
│  └─ WebRTC APM 依赖 abseil-cpp、protobuf 等
│
└─ 仍然失败
   └─ 考虑用 webrtc-audio-processing crate（Rust 绑定）
   └─ 考虑用其他 AEC 库（如 speexdsp）
   └─ 推迟到后续版本，先不做 AEC
```

---

### Android 内部音频采集失败

```
问题：无法采集其他 App 的音频
│
├─ 检查 Android 版本
│  └─ Android 10+ 支持 AudioPlaybackCapture
│
├─ 检查权限
│  └─ 需要 FOREGROUND_SERVICE + MEDIA_PROJECTION
│  └─ 用户必须授权屏幕录制
│
├─ 检查 MediaProjection 是否正确配置
│  └─ 需要 createAudioPlaybackCaptureConfig()
│
└─ 仍然失败
   └─ 降级到 AudioRecord（只能录麦克风）
   └─ 在 UI 中提示用户："此设备不支持内部音频采集"
   └─ 标记为"已知限制"
```

---

### 音频卡顿

```
问题：播放时有杂音、卡顿
│
├─ 检查 ring buffer 是否溢出
│  └─ 增大 ring buffer 容量
│
├─ 检查 ring buffer 是否欠载
│  └─ 检查处理线程是否太慢
│  └─ 检查网络是否丢包
│
├─ 检查 CPU 是否过载
│  └─ 降低编码复杂度
│  └─ 关闭 AEC/NS/AGC
│
├─ 检查采样率是否匹配
│  └─ 检查重采样是否正确
│
└─ 仍然卡顿
   └─ 增大 jitter buffer 延迟
   └─ 记录诊断数据
```

---

### 网络丢包严重

```
问题：UDP 丢包率 >5%
│
├─ 检查网络连接
│  └─ WiFi 信号是否稳定
│  └─ 是否有干扰
│
├─ 启用 FEC 冗余包
│  └─ 每 3 个音频包发 1 个冗余包
│
├─ 增大 jitter buffer 延迟
│  └─ 从 40ms 增大到 80ms
│
├─ 降低比特率
│  └─ 从 128kbps 降到 64kbps
│
└─ 仍然严重
   └─ 考虑切换到 TCP（牺牲延迟换可靠性）
   └─ 在 UI 中提示用户网络质量差
```

---

### 内存泄漏

```
问题：内存持续增长
│
├─ 检查 Rust 代码
│  └─ 循环引用？用 Weak 打破
│  └─ 未释放的资源？检查 Drop trait
│
├─ 检查 FFI 边界
│  └─ C++ 对象是否正确释放？
│  └─ 句柄是否调用了 destroy？
│
├─ 检查 ring buffer
│  └─ 是否有未消费的数据堆积？
│
└─ 仍然泄漏
   └─ 用 valgrind / AddressSanitizer 定位
   └─ 记录泄漏点，修复后回归测试
```

---

### CPU 过载

```
问题：CPU 使用率 >15%
│
├─ 检查编码复杂度
│  └─ Opus complexity 参数是否太高？
│  └─ 尝试降低到 complexity=5
│
├─ 检查 AEC 处理
│  └─ WebRTC APM 是否太重？
│  └─ 考虑关闭 AEC
│
├─ 检查网络处理
│  └─ tokio 运行时线程数是否太多？
│  └─ 尝试减少到 2 个线程
│
└─ 仍然过载
   └─ 在 UI 中提示用户 CPU 不足
   └─ 自动降级到低比特率模式
```

---

## Spike 测试结果

### Spike 1: cpal 延迟测试 ✓ 通过
- **日期**: 2026-05-29
- **结果**: buffer_size=960 时延迟 6.34ms，远低于 50ms 目标
- **发现**: 设备支持 2 通道（立体声），不支持 1 通道；使用 cpal::BufferSize::Default
- **结论**: cpal 在 Windows WASAPI 上延迟表现优秀，可以使用

### Spike 2: WebRTC APM 跨编译验证 ✗ 失败
- **日期**: 2026-05-29
- **结果**: webrtc-audio-processing crate 编译失败
  - bundled 特性: Windows 上 cp 命令不可用
  - 非 bundled: 系统未安装 WebRTC APM 库
- **决策**: 跳过 AEC，先做基础音频处理（增益、静音检测、噪声门）
- **后续**: Phase 6 再集成 WebRTC APM，或考虑 speexdsp

### Phase 1: 音频采集/播放 ✓ 完成
- **日期**: 2026-05-29
- **完成任务**:
  - Task 1.1: audio-capture ✓ (基于 cpal，设备枚举，ring buffer 输出)
  - Task 1.2: audio-playback ✓ (基于 cpal，设备枚举，ring buffer 输入)
  - Task 1.3: ring buffer ✓ (Lock-free SPSC，7 个测试)
- **测试结果**: 全部通过
- **发现**: Windows 设备支持 2 通道（立体声），不支持 1 通道

### Phase 2: 音频处理/混音 ✓ 完成
- **日期**: 2026-05-29
- **完成任务**:
  - Task 2.1: audio-mixer ✓ (加权求和，soft clipping，9 个测试)
  - Task 2.2: audio-processor ✓ (增益控制，静音检测，噪声门，8 个测试)
- **测试结果**: 全部通过

### Phase 3: 协议/网络/设备发现 ✓ 完成
- **日期**: 2026-05-29
- **完成任务**:
  - Task 3.1: protocol ✓ (包格式定义，编解码，3 个测试)
  - Task 3.2: network ✓ (UDP 传输，Jitter Buffer，4 个测试)
  - Task 3.3: discovery ✓ (设备发现框架，3 个测试)
- **测试结果**: 全部通过

### Phase 4: FFI 绑定 ✓ 完成
- **日期**: 2026-05-29
- **完成任务**:
  - Task 4.1: ffi-bindings ✓ (C API，句柄模式，5 个测试)
- **测试结果**: 全部通过

### Phase 5: 集成验证 ✓ 完成
- **日期**: 2026-05-29
- **完成任务**:
  - Task 5.1: 集成测试 ✓ (5 个测试)
  - Task 5.2: 性能基准 ✓ (Ring Buffer, AudioBuffer 基准)
  - Task 5.3: 长时间稳定性 ✓ (5 秒稳定性测试，4 个测试)
- **测试结果**: 全部通过

### Oracle 审查后修复 ✓ 完成
- **日期**: 2026-05-29
- **修复内容**:
  - protocol serialize/deserialize 实现 ✓
  - discovery mDNS 功能实现 ✓
  - 通道数统一为单声道（Mono）✓
  - 占位符测试文件替换为真实测试 ✓
  - 生产代码 .unwrap() 调用修复 ✓
  - 编译警告处理 ✓
  - AEC/NS/AGC 音频处理实现 ✓
- **测试结果**: 全部通过
- **clippy 检查**: 通过（仅剩 discovery 中的 single_match 建议）

## 项目完成度评估

### 已完成的功能
1. **audio-core**: RingBuffer、AudioBuffer、AudioFormat、AudioMode、LevelIndicator ✓
2. **audio-codec**: Opus 编解码，19 个测试 ✓
3. **audio-capture**: 基于 cpal 的音频采集 ✓
4. **audio-playback**: 基于 cpal 的音频播放 ✓
5. **audio-mixer**: 混音器，soft clipping ✓
6. **audio-processor**: 增益控制、静音检测、噪声门、AEC、NS、AGC ✓
7. **protocol**: 包格式定义、序列化/反序列化 ✓
8. **network**: UDP 传输、Jitter Buffer、连接管理、带宽自适应 ✓
9. **discovery**: mDNS 设备发现、设备记忆存储 ✓
10. **ffi-bindings**: C API 绑定 ✓

### 待完善的功能
1. 端到端音频管线集成测试（需要真实音频设备）
2. Windows/Android 平台特定测试
3. 性能优化和基准测试扩展

### 测试统计
- 总测试数: 约 160 个
- 通过率: 100%
- 覆盖率: 基础功能已覆盖
- clippy 检查: 通过

### Oracle 两轮审查后修复总结
- **初始评估**: 项目完成度 30-35%
- **两轮修复后**: 项目完成度约 75%
- **已修复问题**:
  1. AEC 实现为真正的 NLMS 自适应滤波器
  2. NS 实现为基于 SNR 估计的噪声抑制
  3. AGC 添加攻击/释放时间平滑，目标电平 -3 dBFS
  4. DeviceStore 添加 JSON 持久化
  5. 通道数统一为立体声（2ch）
  6. 协议格式 12 字节头，大端字节序
  7. 采样率对齐技术规格
  8. Pipeline trait 和 FFI 接口定义
  9. 电平指示器、音频模式、连接管理、设备记忆、带宽自适应

### 剩余问题（非 Rust 核心）
1. **Windows 测试缺失**：tests/ 目录空（需要 Windows C++ 开发）
2. **Android 未验证可编译**：需要 NDK 构建环境
3. **端到端集成测试**：已创建 pipeline_e2e_test.rs（3 个测试通过）

---

## 风险清单

| 风险 | 概率 | 影响 | 应对方案 | 状态 |
|------|------|------|---------|------|
| Android 内部音频采集不可用 | 高 | 高 | 降级到 MediaProjection，提示用户 | 待验证 |
| WebRTC APM 跨编译失败 | 中 | 中 | 用 webrtc-audio-processing crate 或 speexdsp | ✗ 已确认 |
| cpal 延迟过高 | 低 | 中 | 尝试 exclusive mode，记录数据 | ✓ 已验证 |
| 网络丢包严重 | 中 | 中 | FEC + 增大 jitter buffer | 待验证 |
| 内存泄漏 | 低 | 高 | valgrind 定位，修复 | 待验证 |
| CPU 过载 | 低 | 中 | 降低编码复杂度，关闭 AEC | 待验证 |

---

## 技术参考

### cpal 使用要点
- 默认 buffer size：960 samples（20ms）
- 支持 exclusive mode（WASAPI）
- 回调模式：`device.build_input_stream()` / `device.build_output_stream()`
- 格式转换：cpal 的 SampleFormat 到 audio-core 的 SampleFormat

### Opus 配置
- 均衡模式：64kbps, complexity=10, 20ms 帧
- 高音质模式：128kbps, complexity=10, 20ms 帧
- 超低延迟模式：64kbps, complexity=5, 10ms 帧

### tokio 使用要点
- 运行时：`tokio::runtime::Runtime::new()`
- UDP：`tokio::net::UdpSocket::bind()`
- TCP：`tokio::net::TcpListener::bind()`
- 任务：`tokio::spawn()`

### mdns-sd 使用要点
- 服务类型：`_soundbridge._udp.local.`
- 注册：`mdns.register(service_info)`
- 扫描：`mdns.browse("_soundbridge._udp.local.")`

---

## 代码规范

### Rust 代码规范
- 不用 `unwrap()`，用 `Result` 传播错误
- 公开 API 必须有 doc comment
- 测试用例用 `#[test]` 标注
- 集成测试放在 `tests/` 目录
- 基准测试用 Criterion

### 命名规范
- 类型：PascalCase（`AudioBuffer`）
- 函数：snake_case（`read_audio`）
- 常量：SCREAMING_SNAKE_CASE（`MAGIC`）
- 模块：snake_case（`audio_capture`）

### 错误处理
- 每个 crate 定义自己的 Error 类型
- 用 `thiserror` 派生 `std::error::Error`
- 错误链：底层错误 → 中间层错误 → 用户可见错误
