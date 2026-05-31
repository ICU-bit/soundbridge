# SoundBridge 故障排除指南

> 常见问题诊断与解决方案
> 最后更新：2026-05-31

---

## 目录

- [1. 连接问题](#1-连接问题)
- [2. 音频问题](#2-音频问题)
- [3. 性能问题](#3-性能问题)
- [4. 平台特定问题](#4-平台特定问题)
- [5. 日志收集](#5-日志收集)

---

## 1. 连接问题

### 1.1 无法发现设备

**症状：** Windows 或 Android 端搜索不到对端设备。

**排查步骤：**

1. **确认网络环境**
   - 两台设备必须在同一局域网（同一 WiFi 或有线网络）
   - 检查路由器是否启用了 AP 隔离（Client Isolation）——部分公共 WiFi 会阻止设备间通信
   - 如使用 WiFi 热点模式，确认热点已成功开启且对端已连接

2. **检查 mDNS 服务**
   - SoundBridge 使用 mDNS 服务类型 `_soundbridge._udp.local.` 进行设备发现
   - Windows：确保防火墙未阻止 UDP 5353 端口（mDNS 组播端口）
   - Android：确保 APP 有 `ACCESS_WIFI_STATE` 和 `CHANGE_WIFI_MULTICAST_STATE` 权限
   - 部分路由器默认禁用 mDNS 组播转发，尝试重启路由器

3. **检查防火墙设置**
   - Windows 防火墙：将 SoundBridge 添加到允许列表，或临时关闭防火墙测试
   - 第三方安全软件：检查是否拦截了 UDP 组播流量
   - 路由器防火墙：确认未阻止局域网内设备间通信

4. **尝试手动连接**
   - 如果自动发现失败，可使用手动输入 IP 地址方式连接
   - 在 Windows 端查看对端 IP：Android 设置 → 关于手机 → 状态 → IP 地址

**参考日志关键字：** `mDNS`, `discovery`, `service_not_found`

---

### 1.2 连接超时

**症状：** 设备已发现但连接建立失败，显示超时。

**排查步骤：**

1. **检查网络连通性**
   - 在 Windows 上打开 CMD，执行 `ping <Android-IP>`
   - 在 Android 上使用终端模拟器，执行 `ping <Windows-IP>`
   - 如果 ping 不通，检查网络配置或尝试 WiFi 热点直连模式

2. **检查端口占用**
   - SoundBridge 使用 UDP 端口进行音频传输和控制信令
   - 确认目标端口未被其他程序占用
   - Windows：`netstat -ano | findstr "UDP"` 查看端口占用
   - Android：`netstat -tulnp`（需 root 或使用 Termux）

3. **检查连接模式**
   - WiFi 局域网：确认两设备在同一子网
   - WiFi 热点：确认 Android 端热点已开启，Windows 已连接该热点
   - USB/ADB：确认 ADB 连接正常，`adb devices` 显示设备
   - 蓝牙：确认蓝牙已配对且在有效范围内

4. **QUIC 控制通道问题**
   - 控制信令使用 QUIC 协议（UDP），依赖 TLS 1.3
   - 如果自签名证书验证失败，尝试重新启动两端应用
   - 检查系统时间是否同步——TLS 证书验证对时间敏感

**参考日志关键字：** `connection_timeout`, `quic`, `handshake`, `udp_transport`

---

### 1.3 连接后立即断开

**症状：** 连接成功建立但几秒内断开。

**排查步骤：**

1. **检查 SRTP 加密握手**
   - 音频流使用 SRTP 加密（AES-128-CM + HMAC-SHA1-80）
   - 密钥交换基于 ECDH (x25519)
   - 如果加密握手失败，连接会被对端拒绝
   - 尝试重启两端应用，重新建立密钥交换

2. **检查网络稳定性**
   - WiFi 信号弱会导致频繁断连
   - 查看 WiFi 信号强度，尝试靠近路由器
   - 检查是否有频繁的 IP 地址变更（DHCP 租期过短）

3. **检查心跳机制**
   - SoundBridge 使用心跳包（PacketType::Heartbeat）维持连接
   - 如果心跳超时，连接会被判定为断开
   - 高网络延迟或丢包率过高会导致心跳超时

4. **检查电源管理**
   - Android：检查电池优化设置，将 SoundBridge 设为不受优化
   - Windows：检查电源计划是否为"高性能"，避免 USB WiFi 适配器休眠

**参考日志关键字：** `srtp`, `heartbeat`, `connection_lost`, `key_exchange`

---

## 2. 音频问题

### 2.1 没有声音

**症状：** 连接成功但没有音频输出。

**排查步骤：**

1. **检查音频设备**
   - Windows：确认默认播放设备正确设置
     - 右键任务栏音量图标 → 声音设置 → 输出设备
     - 如果使用 WASAPI 独占模式，确认目标设备支持
   - Android：确认媒体音量不为零
     - 按音量键 → 检查媒体音量滑块

2. **检查音频捕获**
   - Windows：确认系统音频环回捕获（Loopback）正常工作
     - 检查是否有其他程序独占了音频设备
   - Android：确认麦克风权限已授予
     - 设置 → 应用 → SoundBridge → 权限 → 麦克风

3. **检查 Opus 编解码**
   - 音频参数：48000 Hz, 单声道, Float32, 960 samples/frame (20ms)
   - 如果采样率不匹配，重采样可能引入延迟或静音
   - 检查日志中是否有编解码器初始化错误

4. **检查音频管线状态**
   - 音频管线状态机：Idle → Starting → Running
   - 如果卡在 Starting 状态，可能是音频设备初始化失败
   - Windows：`AudioStreamState` 枚举反映当前状态
   - Android：`ConnectionState` + `AudioService` 状态

5. **音量电平检查**
   - 如果电平指示器有显示但无声，问题在播放端
   - 如果电平指示器无显示，问题在捕获端

**参考日志关键字：** `audio_capture`, `playback`, `opus_encode`, `audio_pipeline`, `silent`

---

### 2.2 音频卡顿

**症状：** 声音断断续续，有明显卡顿。

**排查步骤：**

1. **检查网络质量**
   - 音频流使用 UDP 传输，对丢包敏感
   - 查看日志中的丢包率（loss_rate）和自适应码率状态
   - 带宽自适应档位：64 / 96 / 128 kbps
   - 如果丢包率 > 5%，码率会自动降低
   - 尝试靠近路由器或使用 WiFi 热点直连

2. **检查 JitterBuffer**
   - RawJitterBuffer 负责乱序容忍和抖动缓冲
   - 缓冲区太小 → 乱序包被丢弃 → 卡顿
   - 缓冲区太大 → 延迟增加
   - 默认配置适用于大多数场景

3. **检查 PLC（丢包隐藏）**
   - PLC 使用波形外推 + Hanning 窗填补丢包
   - 连续丢包超过 PLC 能力时会出现明显卡顿
   - 如果频繁触发 PLC，根本原因是网络问题

4. **CPU 负载**
   - 音频处理链（AEC + NS + AGC）需要 CPU 资源
   - 高 CPU 占用会导致音频线程得不到及时调度
   - 参见 [3.2 CPU 占用高](#32-cpu-占用高)

5. **检查缓冲区溢出/下溢**
   - 环形缓冲区（RingBuffer）溢出会导致数据丢失
   - 缓冲区下溢会导致播放端等待数据
   - 默认缓冲区大小为 2 的幂次

**参考日志关键字：** `jitter`, `packet_loss`, `plc`, `ring_buffer`, `underrun`

---

### 2.3 回声/啸叫

**症状：** 听到自己的声音回传，或出现刺耳啸叫。

**排查步骤：**

1. **检查 AEC（回声消除）**
   - AEC 使用 NLMS 自适应滤波器
   - AEC 需要参考信号（播放端音频）来消除回声
   - 如果参考信号路径不正确，AEC 无法工作
   - Windows：确认环回捕获（Loopback）正常提供参考信号

2. **使用耳机**
   - 最简单的回声解决方案：使用耳机
   - 扬声器播放 → 麦克风拾取 → 回声，这是物理回声路径
   - AEC 可以消除部分回声，但物理隔离效果最好

3. **检查啸叫条件**
   - 啸叫 = 增益 > 1 且存在反馈回路
   - AGC（自动增益控制）会尝试稳定音量
   - 如果 AGC 参数不当，可能加剧啸叫
   - 尝试降低一端的播放音量

4. **检查音频处理链顺序**
   - 正确顺序：捕获 → AEC → NS → AGC → 编码
   - 如果顺序错误（如 AGC 在 AEC 之前），会放大回声

**参考日志关键字：** `aec`, `echo`, `agc`, `feedback`

---

### 2.4 音量太小/太大

**症状：** 音频音量异常，过大导致削波或过小听不清。

**排查步骤：**

1. **检查 AGC（自动增益控制）**
   - AGC 使用攻击/释放时间平滑控制音量
   - 如果增益过大，可能导致削波失真
   - 如果增益过小，可能导致音量不足
   - AGC 参数在 `audio-processor` crate 中配置

2. **检查系统音量**
   - Windows：检查系统音量混合器中各应用音量
   - Android：检查媒体音量和通话音量
   - 确认 SoundBridge 的音量未被单独调低

3. **检查音频格式**
   - SoundBridge 内部使用 Float32 格式
   - 如果输入源是 16-bit PCM，转换可能导致音量差异
   - 确认输入设备的位深设置正确

4. **检查 Opus 编码增益**
   - Opus 编码器有内置的增益控制
   - 检查编码参数中的比特率设置
   - 较低比特率（64kbps）可能影响音质

**参考日志关键字：** `agc`, `gain`, `clipping`, `volume`, `opus_bitrate`

---

## 3. 性能问题

### 3.1 延迟过高

**症状：** 音频延迟明显（> 100ms），影响实时交互体验。

**排查步骤：**

1. **音频参数基准**
   - 帧大小：960 samples = 20ms（单帧处理延迟）
   - 编码延迟：Opus at 48kHz ≈ 5ms
   - 网络延迟：WiFi 局域网通常 1-5ms
   - 理论最低端到端延迟：约 30-50ms

2. **检查音频缓冲区**
   - WASAPI 共享模式通常有额外 10-30ms 缓冲
   - 尝试使用 WASAPI 独占模式降低延迟（参见 [4.1](#41-windowswasapi-独占模式)）
   - Android：AAudio 低延迟模式比 AudioRecord 延迟更低

3. **检查网络路径**
   - 直连（WiFi 热点）比经过路由器转发延迟更低
   - 检查是否有 VPN 或代理增加网络跳数
   - USB/ADB 模式通常延迟最低且最稳定

4. **检查 JitterBuffer 配置**
   - JitterBuffer 增加缓冲以容忍网络抖动
   - 缓冲越大 → 延迟越高但更平滑
   - 在稳定网络环境下可以减小缓冲

5. **检查音频处理链**
   - AEC + NS + AGC 处理增加约 2-5ms 延迟
   - 如果不需要回声消除，可以在设置中禁用

**参考日志关键字：** `latency`, `buffer_size`, `jitter_buffer`, `processing_time`

---

### 3.2 CPU 占用高

**症状：** SoundBridge 进程 CPU 占用率异常偏高。

**排查步骤：**

1. **检查音频处理负载**
   - NLMS 自适应滤波器（AEC）是 CPU 密集型操作
   - 滤波器阶数越高 → CPU 负载越大但回声消除效果越好
   - 如果不需要 AEC，禁用可显著降低 CPU 占用

2. **检查编解码负载**
   - Opus 编解码在低比特率模式下计算量较小
   - 高比特率（128kbps）比低比特率（64kbps）CPU 占用略高
   - 确认编解码器使用了平台优化（SSE/NEON）

3. **检查网络线程**
   - UDP 传输使用 tokio 异步运行时
   - 如果网络线程阻塞，会增加 CPU 等待开销
   - 确认 DNS 解析和 mDNS 查询未频繁重试

4. **Windows 特定**
   - C# UI 线程和 C++ 音频线程分离
   - 如果 UI 刷新频率过高（电平指示器），会增加 CPU 占用
   - 检查是否有 WPF/WinUI 渲染问题

5. **Android 特定**
   - JNI 调用有固定开销，避免高频 JNI 调用
   - Jetpack Compose 重组频率检查
   - 使用 Android Profiler 查看 CPU 热点

**参考日志关键字：** `cpu_usage`, `processing_time`, `thread_blocked`

---

### 3.3 内存泄漏

**症状：** 长时间运行后内存持续增长，最终导致 OOM 或系统变慢。

**排查步骤：**

1. **检查 JNI Handle 释放（Android）**
   - 所有 `native*` 方法创建的 handle 必须在 `onDestroy` 中释放
   - `nativeRelease` / `nativeReleaseEncoder` / `nativeReleaseDecoder`
   - handle 值为 `0L` 表示无效，释放前检查
   - 这是最常见的内存泄漏来源

2. **检查 Rust 内存管理**
   - Rust 的所有权系统通常防止内存泄漏
   - 但 `Rc`/`Arc` 循环引用仍可能导致泄漏
   - 检查 `RingBuffer` 和 `AudioBuffer` 的生命周期

3. **检查 C++ 对象生命周期**
   - `AudioEngineImpl` 和 `Session` 使用 `std::unique_ptr` 管理
   - 确认工厂函数创建的对象在不再使用时正确销毁
   - 回调中的 `std::function` 可能持有外部引用

4. **检查网络缓冲区**
   - UDP 接收缓冲区如果处理不及时会堆积
   - 确认接收线程及时消费数据
   - 断开连接时确认所有网络资源已释放

5. **监控工具**
   - Windows：任务管理器 → 详细信息 → 内存列，观察增长趋势
   - Android：Android Studio Profiler → Memory 页签
   - Rust：可使用 `dhat` 或 `valgrind` 检测泄漏

**参考日志关键字：** `memory`, `allocation`, `leak`, `handle_release`

---

## 4. 平台特定问题

### 4.1 Windows：WASAPI 独占模式

**问题：** WASAPI 独占模式下无法捕获或播放音频。

**原因：** WASAPI 独占模式允许应用程序独占音频设备，其他程序无法同时使用。

**排查步骤：**

1. **确认设备支持独占模式**
   - 不是所有音频设备都支持独占模式
   - 右键音量图标 → 声音设置 → 更多声音设置 → 播放 → 设备属性 → 高级
   - 查看"独占模式"相关选项

2. **检查设备占用**
   - 独占模式下只能有一个程序使用设备
   - 关闭其他可能占用音频设备的程序（浏览器、音乐播放器等）
   - 如果其他程序正在使用，WASAPI 独占请求会失败

3. **回退到共享模式**
   - 如果独占模式不可用，SoundBridge 自动回退到共享模式
   - 共享模式延迟略高但兼容性更好
   - 日志中查看 `exclusive_mode` 标记确认当前模式

4. **音频会话冲突**
   - Windows 音频会话管理器可能阻止新的独占请求
   - 重启 Windows Audio 服务：`net stop Audiosrv && net start Audiosrv`

**参考日志关键字：** `wasapi`, `exclusive`, `shared`, `device_busy`

---

### 4.2 Android：权限问题

**问题：** 应用无法正常工作，日志显示权限相关错误。

**所需权限清单：**

| 权限 | 用途 | 必需 |
|------|------|------|
| `RECORD_AUDIO` | 麦克风音频采集 | 是 |
| `INTERNET` | 网络通信 | 是 |
| `ACCESS_NETWORK_STATE` | 检查网络状态 | 是 |
| `ACCESS_WIFI_STATE` | 检查 WiFi 状态 | 是 |
| `CHANGE_WIFI_MULTICAST_STATE` | mDNS 设备发现 | 是 |
| `FOREGROUND_SERVICE` | 前台服务保活 | 是 |
| `FOREGROUND_SERVICE_TYPE_MICROPHONE` | 前台服务音频类型 (Android 10+) | 是 |
| `POST_NOTIFICATIONS` | 显示通知 (Android 13+) | 是 |
| `ACCESS_FINE_LOCATION` | WiFi 扫描（部分设备需要） | 否 |
| `BLUETOOTH_CONNECT` | 蓝牙连接 (Android 12+) | 否 |
| `NEARBY_WIFI_DEVICES` | WiFi 设备发现 (Android 13+) | 否 |

**排查步骤：**

1. **检查运行时权限**
   - Android 6.0+ 需要运行时请求危险权限
   - 设置 → 应用 → SoundBridge → 权限 → 确认所有必要权限已授予
   - `RECORD_AUDIO` 是运行时权限，必须在 APP 中主动请求

2. **Android 13+ 通知权限**
   - `POST_NOTIFICATIONS` 在 Android 13+ 为运行时权限
   - 如果未授予，前台服务通知可能不显示，但服务仍可运行
   - 建议首次启动时引导用户授予权限

3. **Android 14+ 前台服务类型**
   - `FOREGROUND_SERVICE_TYPE_MICROPHONE` 必须在 `AndroidManifest.xml` 中声明
   - `startForeground()` 时必须指定正确的服务类型
   - 缺少类型声明会导致 `ForegroundServiceStartNotAllowedException`

4. **权限被永久拒绝**
   - 如果用户选择"不再询问"，`requestPermissions` 不再弹出对话框
   - 引导用户到系统设置手动开启
   - 使用 `shouldShowRequestPermissionRationale()` 判断状态

**参考日志关键字：** `permission_denied`, `security_exception`, `foreground_service`

---

### 4.3 Android：后台服务被杀

**问题：** SoundBridge 在后台运行时服务被系统终止。

**排查步骤：**

1. **确认前台服务正在运行**
   - `AudioService` 使用 `START_STICKY` 标记，被杀后会自动重启
   - 确认通知栏有 SoundBridge 的常驻通知
   - 如果通知消失，说明服务已被终止

2. **电池优化设置**
   - 设置 → 电池 → 电池优化 → 找到 SoundBridge → 选择"不优化"
   - 不同厂商路径可能不同：
     - 小米/红米：设置 → 电池 → 后台无限制
     - 华为/荣耀：设置 → 电池 → 启动管理 → 手动管理全部开启
     - OPPO/一加：设置 → 电池 → 更多电池设置 → 优化电池使用
     - vivo：设置 → 电池 → 后台耗电管理
     - 三星：设置 → 电池 → 后台使用限制 → 从不休眠的应用

3. **厂商自定义限制**
   - 部分厂商有额外的后台限制（MIUI、ColorOS、EMUI 等）
   - 需要用户手动将 SoundBridge 加入"白名单"或"自启动"列表
   - 锁定最近任务卡片（下拉锁定）可防止被清理

4. **前台服务通知**
   - 前台服务必须显示持续通知
   - 通知渠道 `soundbridge_channel` 必须创建且未被用户关闭
   - Android 8.0+ 需要 `NotificationChannel`

5. **`START_STICKY` 行为**
   - 服务被杀后系统会尝试重启
   - 但重启时 `intent` 为 `null`，需要在 `onStartCommand` 中处理
   - 如果短时间内被杀多次，系统可能放弃重启

**参考日志关键字：** `service_killed`, `restart`, `foreground`, `battery_optimization`

---

## 5. 日志收集

### 5.1 Windows 日志位置

SoundBridge Windows 端使用 spdlog 进行日志记录。

**日志文件位置：**
```
%LOCALAPPDATA%\SoundBridge\logs\
├── soundbridge.log        # 主日志文件（滚动更新）
└── soundbridge_YYYY-MM-DD.log  # 按日期归档
```

**日志级别：**
- `trace` — 详细跟踪信息（仅调试版本）
- `debug` — 调试信息
- `info` — 一般运行信息
- `warn` — 警告信息
- `error` — 错误信息
- `critical` — 严重错误

**查看实时日志：**
```powershell
# PowerShell 实时查看日志
Get-Content "$env:LOCALAPPDATA\SoundBridge\logs\soundbridge.log" -Wait

# 过滤错误和警告
Get-Content "$env:LOCALAPPDATA\SoundBridge\logs\soundbridge.log" | Select-String -Pattern "error|warn|critical"
```

**Windows 事件查看器：**
- 应用程序事件日志中也可能有相关信息
- `eventvwr.msc` → Windows 日志 → 应用程序

**Debug 版本日志：**
- Debug 构建包含 `trace` 级别日志
- Release 构建默认 `info` 级别
- 如需在 Release 版本启用详细日志，修改 spdlog 配置

---

### 5.2 Android 日志获取

**使用 adb logcat：**
```bash
# 查看所有 SoundBridge 日志
adb logcat -s SoundBridge

# 过滤特定级别
adb logcat -s SoundBridge:V    # Verbose
adb logcat -s SoundBridge:D    # Debug
adb logcat -s SoundBridge:W    # Warning
adb logcat -s SoundBridge:E    # Error

# 保存日志到文件
adb logcat -s SoundBridge > soundbridge_log.txt

# 查看崩溃堆栈
adb logcat -s AndroidRuntime:E

# 清除旧日志后重新捕获
adb logcat -c && adb logcat -s SoundBridge
```

**使用 Android Studio Logcat：**
- 打开 Android Studio → Logcat 窗口
- 过滤标签：`SoundBridge`
- 可按级别、关键字、进程过滤

**关键日志标签：**
- `AudioService` — 音频服务生命周期
- `NativeAudioEngine` — JNI 调用
- `AudioCaptureManager` — 音频采集
- `AudioPlaybackManager` — 音频播放

---

### 5.3 崩溃报告

**Windows 崩溃报告：**

1. **Windows 事件日志**
   - 打开 `eventvwr.msc`
   - Windows 日志 → 应用程序
   - 查找来源为 `Application Error` 或 `.NET Runtime` 的事件

2. **Windows 错误报告（WER）**
   - 崩溃转储文件位于：
   ```
   %LOCALAPPDATA%\CrashDumps\
   %LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive\
   ```

3. **手动收集崩溃信息**
   - 如果应用崩溃无日志输出，启用 Windows 错误报告：
   - 注册表 `HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps`
   - 创建 `DumpType` (DWORD) = 2（完整转储）

**Android 崩溃报告：**

1. **logcat 崩溃堆栈**
   ```bash
   # 获取最近的崩溃信息
   adb logcat -b crash -d

   # 保存崩溃日志
   adb logcat -b crash > crash_log.txt
   ```

2. **ANR（Application Not Responding）**
   ```bash
   # 查看 ANR 信息
   adb pull /data/anr/traces.txt

   # 部分设备路径
   adb pull /data/anr/anr_* 
   ```

3. **Tombstone（Native 崩溃）**
   ```bash
   # C++ 层崩溃（JNI/native 代码）
   adb pull /data/tombstones/

   # 查看最新的 tombstone
   adb shell ls -lt /data/tombstones/
   ```

4. **Bug 报告（完整系统信息）**
   ```bash
   # 生成完整 bug 报告
   adb bugreport > bugreport.zip

   # 解压后查看 main_entry.txt 和 FS/data/ 部分
   ```

**提交 Bug 报告时请包含：**
- 操作系统版本
- 设备型号（Android 需要）
- SoundBridge 版本
- 复现步骤
- 日志文件（使用上述方法获取）
- 网络环境描述（WiFi/热点/USB）

---

## 附录：快速诊断清单

遇到问题时，按此清单快速排查：

- [ ] 两台设备在同一网络？
- [ ] 防火墙已放行 SoundBridge？
- [ ] Android 权限全部授予？
- [ ] Android 电池优化已关闭？
- [ ] 音频设备正常工作？（其他 APP 能播放/录音）
- [ ] 日志中是否有明显错误？
- [ ] 重启两端 APP 后是否恢复？
- [ ] 重启路由器后是否恢复？
