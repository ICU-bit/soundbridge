# SoundBridge AI 迭代全局上下文

## 🎯 项目核心目标
- **功能**：跨平台音频传输 (Windows ↔ Android)，双向传输，混音播放
- **目标场景**：游戏时不用摘耳机，同时听电脑和手机的声音
- **极致性能**：端到端延迟 < 30ms，CPU < 5%，内存 < 50MB
- **极致稳定性**：Rust 内存安全，编译时检查

## 🏗️ 技术架构
```
SoundBridge
├── rust-core/          # 100% Rust 核心库
│   ├── audio-core/
│   ├── audio-capture/
│   ├── audio-playback/
│   ├── audio-codec/
│   ├── audio-processor/
│   ├── audio-mixer/
│   ├── network/
│   ├── discovery/
│   ├── protocol/
│   └── ffi-bindings/
├── windows-app/        # WinUI 3 (C#)
└── android-app/        # Kotlin + Compose
```

## 📊 当前性能基准
| 指标 | 目标值 | 当前值 |
|------|--------|--------|
| 端到端延迟 | < 30ms | TBD |
| CPU 占用 | < 5% | TBD |
| 内存占用 | < 50MB | TBD |

## 🔄 迭代历史
- **2024-01-15**: 项目初始化，创建规格文档和目录结构

## 🎯 下一步迭代方向
1. 实现 audio-core 基础音频抽象
2. 创建各 crate 的基础结构
3. 完善测试体系
4. 添加基准测试

## 📝 开发原则
- **每个 crate 独立可迭代**：修改一个 crate 不影响其他
- **先保证正确性，再优化性能**
- **完善的测试覆盖**：单元测试 > 80%
- **性能可追踪**：每次迭代都有基准对比
- **每个 crate 都有 AI_GUIDE.md**：明确当前状态和下一步

## 🛠️ 常用命令
```bash
# 运行所有测试
cargo test --workspace

# 运行基准测试
cargo bench --workspace

# 代码质量检查
cargo clippy --workspace
cargo fmt -- --check

# 运行特定 crate 的测试
cargo test -p audio-core
```

## ⚠️ 注意事项
- 先看每个 crate 的 AI_GUIDE.md，了解当前状态
- 每次修改后运行基准测试，对比性能变化
- 保持 API 稳定，避免破坏现有代码
