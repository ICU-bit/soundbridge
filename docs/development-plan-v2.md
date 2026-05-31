# SoundBridge 开发计划 v2

> 最后更新：2026-05-31

## 项目概览

跨端音频融合：Windows (C++/C#) ↔ Android (Kotlin/JNI)，Rust 核心引擎。
核心场景：游戏时不用摘耳机，同时听电脑和手机的声音。

---

## 第一阶段：基础架构 ✅ 已完成

| 任务 | 状态 | 完成时间 |
|------|------|----------|
| Rust 核心 10 crate 实现 | ✅ | 2026-05 |
| Windows C++/C# 完整应用 | ✅ | 2026-05 |
| Android Kotlin/JNI 完整应用 | ✅ | 2026-05 |
| CI/CD (GitHub Actions) | ✅ | 2026-05 |
| 开发工具链 (scripts/, tools/) | ✅ | 2026-05 |

---

## 第二阶段：安全加固 ✅ 已完成

| 任务 | 状态 | Commit |
|------|------|--------|
| ECDH 密钥交换：模拟→真实 x25519-dalek | ✅ | `bfd8d2f` |
| DTLS 密钥推导：XOR→HKDF-SHA1 (Windows) | ✅ | `4b4d264` |
| 线程安全：encryption_enabled_→AtomicBool | ✅ | `4b4d264` |
| JNI 全局变量→std::atomic | ✅ | `4b4d264` |
| SRTP 密钥材料加 mutex 保护 | ✅ | `4b4d264` |
| WSA/COM 资源清理 | ✅ | `4b4d264` |
| 移除 6 处 unwrap()/expect() | ✅ | `4b4d264` |

---

## 第三阶段：功能完善 ✅ 已完成

| 任务 | 状态 | Commit |
|------|------|--------|
| 静音：管线线程检查 muted 标志 | ✅ | `c05cf2b` |
| Windows 开机自启 (Registry) | ✅ | `46afa21` |
| Windows WiFi 热点 (netsh) | ✅ | `b3fa67d` |
| Windows ADB 端口转发 | ✅ | `b3fa67d` |
| Windows 自动连接上次设备 | ✅ | `b3fa67d` |
| Android 静音按钮→nativeSetMute | ✅ | `b3fa67d` |
| Android 设置持久化 (SharedPreferences) | ✅ | `366f4c1` |
| Android 蓝牙 RFCOMM 接入管线 | ✅ | `32bf4bb` |
| Android 蓝牙 RFCOMM↔UDP 桥接 | ✅ | `895a2d1` |
| FFI 占位符测试→真实测试 (10个) | ✅ | `78c524b` |
| Android JNI 原生测试 | ✅ | `3c03abd` |
| 性能：VecDeque jitter buffer | ✅ | `4b4d264` |
| 性能：零分配 SRTP | ✅ | `4b4d264` |
| Mono 默认值修正 (channels=1) | ✅ | `4b4d264` |

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
