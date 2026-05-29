# SoundBridge MVP 任务列表

## 第一阶段：项目搭建

- [x] Task 1: 创建项目根目录结构
  - [x] SubTask 1.1: 创建 Cargo 工作空间
  - [x] SubTask 1.2: 创建 AI_CONTEXT.md
  - [x] SubTask 1.3: 创建开发脚本

- [x] Task 2: 创建 rust-core 多 crate 结构
  - [x] SubTask 2.1: 创建 audio-core
  - [x] SubTask 2.2: 创建 audio-capture
  - [x] SubTask 2.3: 创建 audio-playback
  - [x] SubTask 2.4: 创建 audio-codec
  - [x] SubTask 2.5: 创建 audio-processor
  - [x] SubTask 2.6: 创建 audio-mixer
  - [x] SubTask 2.7: 创建 network
  - [x] SubTask 2.8: 创建 discovery
  - [x] SubTask 2.9: 创建 protocol
  - [x] SubTask 2.10: 创建 ffi-bindings (C API)

- [x] Task 3: 为每个 crate 创建 AI_GUIDE.md

## 第二阶段：核心音频引擎

- [ ] Task 4: 实现 audio-core
  - [ ] SubTask 4.1: 音频缓冲区抽象 (零拷贝)
  - [ ] SubTask 4.2: 音频格式定义
  - [ ] SubTask 4.3: 基础测试

- [ ] Task 5: 实现 audio-capture
  - [ ] SubTask 5.1: WASAPI 实现 (Windows)
  - [ ] SubTask 5.2: AAudio 实现 (Android)
  - [ ] SubTask 5.3: 统一 API 抽象
  - [ ] SubTask 5.4: 测试

- [ ] Task 6: 实现 audio-playback
  - [ ] SubTask 6.1: WASAPI 实现 (Windows)
  - [ ] SubTask 6.2: AAudio 实现 (Android)
  - [ ] SubTask 6.3: 统一 API 抽象
  - [ ] SubTask 6.4: 测试

- [x] Task 7: 实现 audio-codec (Opus)
  - [x] SubTask 7.1: 集成 opus-rs
  - [x] SubTask 7.2: 编码器实现
  - [x] SubTask 7.3: 解码器实现
  - [x] SubTask 7.4: 测试 + 基准测试

- [ ] Task 8: 实现 audio-mixer
  - [ ] SubTask 8.1: 基础混音算法
  - [ ] SubTask 8.2: 多通道支持
  - [ ] SubTask 8.3: 音量控制
  - [ ] SubTask 8.4: 测试 + 基准测试

- [ ] Task 9: 实现 audio-processor
  - [ ] SubTask 9.1: 噪声抑制
  - [ ] SubTask 9.2: 自动增益控制
  - [ ] SubTask 9.3: 测试 + 基准测试

## 第三阶段：网络传输

- [ ] Task 10: 实现 protocol
  - [ ] SubTask 10.1: 协议定义
  - [ ] SubTask 10.2: 序列化/反序列化 (零拷贝)
  - [ ] SubTask 10.3: 测试

- [ ] Task 11: 实现 network
  - [ ] SubTask 11.1: UDP 音频流传输
  - [ ] SubTask 11.2: QUIC 控制信令
  - [ ] SubTask 11.3: 丢包恢复
  - [ ] SubTask 11.4: 测试 + 基准测试

- [ ] Task 12: 实现 discovery
  - [ ] SubTask 12.1: mDNS 服务注册
  - [ ] SubTask 12.2: mDNS 服务发现
  - [ ] SubTask 12.3: 设备列表管理
  - [ ] SubTask 12.4: 测试

## 第四阶段：FFI 和应用

- [ ] Task 13: 实现 ffi-bindings
  - [ ] SubTask 13.1: C API 定义
  - [ ] SubTask 13.2: 实现 FFI 包装
  - [ ] SubTask 13.3: 测试

- [ ] Task 14: 创建 Windows 应用
  - [ ] SubTask 14.1: WinUI 3 项目结构
  - [ ] SubTask 14.2: 集成 Rust FFI
  - [ ] SubTask 14.3: 主界面实现
  - [ ] SubTask 14.4: 系统托盘
  - [ ] SubTask 14.5: 测试

- [ ] Task 15: 创建 Android 应用
  - [ ] SubTask 15.1: Kotlin + Compose 项目结构
  - [ ] SubTask 15.2: 集成 Rust JNI
  - [ ] SubTask 15.3: 主界面实现
  - [ ] SubTask 15.4: 前台服务
  - [ ] SubTask 15.5: 测试

## 第五阶段：集成测试与优化

- [ ] Task 16: 端到端功能测试
  - [ ] SubTask 16.1: Windows ↔ Android 音频传输测试
  - [ ] SubTask 16.2: 双向传输测试
  - [ ] SubTask 16.3: 混音测试
  - [ ] SubTask 16.4: 设备发现测试

- [ ] Task 17: 性能优化
  - [ ] SubTask 17.1: 延迟优化
  - [ ] SubTask 17.2: CPU 占用优化
  - [ ] SubTask 17.3: 内存占用优化
  - [ ] SubTask 17.4: SIMD 优化
  - [ ] SubTask 17.5: 基准对比测试

- [ ] Task 18: 稳定性测试
  - [ ] SubTask 18.1: 长时间运行测试
  - [ ] SubTask 18.2: 网络异常测试
  - [ ] SubTask 18.3: 设备兼容性测试

# 任务依赖关系

- Task 1 → Task 2 → Task 3
- Task 4 → Task 5, Task 6
- Task 5, Task 6 → Task 7, Task 8, Task 9
- Task 4 → Task 10 → Task 11 → Task 12
- Task 7, Task 8, Task 9, Task 11, Task 12 → Task 13
- Task 13 → Task 14, Task 15
- Task 14, Task 15 → Task 16 → Task 17 → Task 18
