# Contributing to SoundBridge

感谢你对 SoundBridge 项目的关注！

## 开发环境

### Rust 核心
- Rust 1.75+ (stable)
- `cargo test --workspace` 运行所有测试
- `cargo clippy --workspace` 代码质量检查
- `cargo fmt -- --check` 格式检查

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

## 代码规范

### Rust
- 遵循 `rustfmt.toml` 格式配置
- 零 clippy 警告
- 不使用 `unwrap()`（生产代码）
- 所有 crate 必须有 `AI_GUIDE.md`

### C++
- C++20 标准
- MSVC `/permissive-` 严格模式
- 接口/实现分离（`IAudioEngine` 纯虚 vs `AudioEngineImpl`）
- 工厂函数返回 `std::unique_ptr`

### Kotlin
- Jetpack Compose + Material3
- 深色主题优先
- `StateFlow` 响应式状态管理

## 提交流程

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `style:` 代码格式（不影响功能）
- `refactor:` 重构
- `test:` 测试相关
- `chore:` 构建/工具相关

## 测试

- Rust: `cargo test --workspace`（554 测试）
- Windows C++: CMake + GTest（27 测试）
- Android: Gradle + JUnit

## 问题反馈

使用 [GitHub Issues](https://github.com/ICU-bit/soundbridge/issues) 报告问题。
