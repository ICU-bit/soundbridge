# audio-core 开发指南

## 概述

`audio-core` 是整个 SoundBridge 项目的基础音频库，提供核心的音频数据结构和抽象。

## 核心设计原则

1. **零拷贝优先**: 使用 `bytes::Bytes` 实现零拷贝的音频数据传输
2. **类型安全**: 通过泛型 `Sample` trait 确保类型安全
3. **最小依赖**: 保持依赖精简，只引入必要的库

## 当前功能

- **SampleFormat**: 音频样本格式枚举 (I16, I32, F32, F64)
- **AudioFormat**: 音频格式描述 (采样率、通道数、样本格式)
- **Sample Trait**: 样本类型抽象
- **AudioBuffer<T>**: 零拷贝音频缓冲区，支持多种样本类型
- **RingBuffer<T>**: Lock-free SPSC 环形缓冲区，用于线程间音频数据传递
- **AudioMode**: 音频模式枚举 (Balanced, HighQuality, LowLatency)
- **AudioError**: 统一错误类型
- **Pipeline**: 音频管线 trait
- **SharedPipelineStats**: 共享管线统计（frames_encoded/decoded, loss_rate, level, exclusive_mode）

## RingBuffer 使用

```rust
use audio_core::RingBuffer;

// 创建缓冲区（容量必须是 2 的幂）
let rb = RingBuffer::<f32>::new(1024);

// 写入数据（生产者线程）
let data = [1.0f32, 2.0, 3.0];
rb.write(&data);

// 读取数据（消费者线程）
let mut output = [0.0f32; 3];
rb.read(&mut output);
```

**特性**:
- Lock-free（无锁）
- SPSC（单生产者单消费者）
- 容量自动向上取整到 2 的幂
- 线程安全（实现了 Send + Sync）

## 测试

测试文件放在 `tests/` 目录下，使用 `cargo test` 运行测试。

## 性能考虑

- 使用 `Bytes` 替代 `Vec<u8>` 以实现高效的零拷贝
- 避免不必要的内存分配
- 考虑 SIMD 优化（未来版本）

## 注意事项

- 保持 API 稳定，这是其他 crates 的基础
- 任何破坏性变更都需要谨慎评估
- 确保与其他 audio-* crates 的兼容性
