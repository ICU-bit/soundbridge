# Android 端开发状态

> 最后更新：2026-06-01
> 本文档供后续 session（特别是 PC 端开发）快速了解 Android 端已完成的工作和接口约定。

---

## 一、已完成的功能

### 1.1 设置页面（全部真实生效 + 持久化）

| 设置项 | SharedPreferences Key | Rust FFI / Native 函数 | 生效方式 |
|--------|----------------------|----------------------|---------|
| 回声消除 | `echo_cancellation` (Bool, 默认 true) | `sb_set_echo_cancellation_enabled(engine, enabled)` | 即时生效 |
| 噪声抑制 | `noise_suppression` (Bool, 默认 true) | `sb_set_noise_suppression_enabled(engine, enabled)` | 即时生效 |
| 自动增益 | `gain_control` (Bool, 默认 true) | `sb_set_agc_enabled(engine, enabled)` | 即时生效 |
| 采样率 | `sample_rate` (Int, 默认 48000) | `sb_set_sample_rate(rate)` | 下次连接生效 |
| 码率 | `bitrate` (Int, 默认 128000) | `sb_set_bitrate(bitrate)` | 下次连接生效 |
| 自动档位 | `auto_mode` (Bool, 默认 false) | `sb_set_auto_profile_enabled(enabled)` | 下次连接生效 |
| 均衡器开关 | `eq_enabled` (Bool, 默认 true) | `sb_set_eq_enabled(enabled)` | 即时生效 |
| 均衡器预设 | `eq_preset` (Int, 默认 0) | `sb_set_eq_preset(preset)` | 即时生效 |
| 加密状态 | (由 AudioService 管理) | `sb_set_encryption_enabled(...)` | 即时生效 |
| 混音比例 | `mix_pc_volume` / `mix_phone_volume` (Float) | `sb_set_mix_ratio(...)` | 即时生效 |

**采样率选项：** 44100 / 48000 / 96000 / 192000 Hz
**码率选项：** 128000 / 192000 / 256000 / 320000 / 512000 / 1024000 bps
**均衡器预设：** Flat(0) / Gaming(1) / Music(2) / Voice(3) / Bass(4) / Treble(5)

**关键设计：**
- 自动档位 ON 时，采样率/码率下拉菜单变灰禁用，管线使用 `AudioModeManager` 的自动配置
- 自动档位 OFF 时，管线使用手动设置的采样率/码率
- 所有设置在 `AudioService.initializeEngine()` 时从 SharedPreferences 恢复到 native 层

### 1.2 主页功能

| 功能 | 状态 | 说明 |
|------|:---:|------|
| 连接/断开按钮 | ✅ | 调用 `AudioService.connectToServer(address, port, connectionType)` |
| 静音按钮 | ✅ | 调用 `AudioService.setMute(bool)` |
| 扫描设备 | ✅ | mDNS `_soundbridge._udp` 发现 |
| 设备列表点击 | ✅ | 自动填入服务器地址和端口 |
| 混音比例滑块 | ✅ | 0=全PC / 100=全手机，调用 `setMixRatio(pcVol, phoneVol)` |
| 连接方式选择 | ✅ | 4 种芯片，传入 `ConnectionType` 枚举 |

### 1.3 连接方式

```kotlin
// AudioService.ConnectionType 枚举
enum class ConnectionType {
    WIFI_LAN,       // WiFi 局域网（直连）— 完全可用
    WIFI_DIRECT,    // WiFi 直连（热点）— Android 端就绪，需 PC 配合
    USB_ADB,        // USB/ADB 端口转发 — Android 端就绪，需 PC 配合
    BLUETOOTH       // 蓝牙 RFCOMM — Android 端就绪，需 PC 配合
}
```

| 连接方式 | Android 端 | PC 端 | 端到端可用 |
|----------|:---:|:---:|:---:|
| WiFi 局域网 | ✅ | ✅ | ✅ |
| WiFi 直连 | ✅（热点创建） | ❌ 需实现 | ❌ |
| USB/ADB | ✅（端口转发） | ❌ 需实现 | ❌ |
| 蓝牙 | ✅（RFCOMM 监听） | ❌ 需实现 | ❌ |

---

## 二、Rust 核心改动（本次 session 新增）

### 2.1 FFI 新增函数

```rust
// 回声消除开关
pub extern "C" fn sb_set_echo_cancellation_enabled(engine: *mut c_void, enabled: c_int) -> c_int;

// 噪声抑制开关
pub extern "C" fn sb_set_noise_suppression_enabled(engine: *mut c_void, enabled: c_int) -> c_int;

// 自动增益控制开关
pub extern "C" fn sb_set_agc_enabled(engine: *mut c_void, enabled: c_int) -> c_int;
```

**实现位置：** `rust-core/crates/ffi-bindings/src/lib.rs`
**内部机制：** 修改 `SbEngine.processor` 的 `aec_enabled` / `ns_enabled` / `agc_enabled` 标志，`process()` 和 `process_with_aec()` 根据标志决定是否跳过对应处理器。

### 2.2 Bitrate 枚举扩展

```rust
// rust-core/crates/audio-codec/src/lib.rs
pub enum Bitrate {
    Kbps64 = 64000,
    Kbps96 = 96000,
    Kbps128 = 128000,
    Kbps192 = 192000,   // 新增
    Kbps256 = 256000,
    Kbps320 = 320000,   // 新增
    Kbps512 = 512000,   // 新增
    Kbps1024 = 1024000, // 新增
}
```

### 2.3 管线启动逻辑修改

管线创建 Opus 编解码器时，根据 `AUTO_PROFILE_ENABLED` 标志决定配置来源：
- **自动档 ON：** 读取 `AudioModeManager.current_config()`（根据网络状况动态调整）
- **自动档 OFF：** 读取 `SAMPLE_RATE` / `BITRATE` 原子变量（用户手动设置的值）

**修改位置：** `rust-core/crates/ffi-bindings/src/lib.rs` 约第 1099 行

---

## 三、JNI 桥接层

### 3.1 新增的 JNI 函数（jni_bridge.cpp）

```cpp
// 回声消除 — 调用 Rust FFI sb_set_echo_cancellation_enabled
JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEchoCancellationEnabled(
    JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled);

// 噪声抑制 — 调用 Rust FFI sb_set_noise_suppression_enabled
JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetNoiseSuppressionEnabled(
    JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled);

// 自动增益 — 调用 Rust FFI sb_set_agc_enabled
JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetGainControlEnabled(
    JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled);
```

**注意：** 这三个函数在 `SOUNDBRIDGE_USE_RUST_FFI` 宏开启时走 Rust FFI，否则走 C++ AudioEngine stub。

---

## 四、PC 端开发需要对接的接口

### 4.1 必须实现的连接方式

PC 端需要至少实现以下一种连接方式才能与 Android 端通信：

1. **WiFi 局域网（推荐优先实现）**
   - PC 端监听 UDP 端口
   - Android 端通过 `sb_bind` + `sb_connect` 连接
   - 双向音频流：PC→Android（PC 电脑声音）+ Android→Android（手机麦克风）

2. **WiFi 直连（后续）**
   - PC 端连接到 Android 创建的热点
   - 热点默认网关：`192.168.43.1`
   - PC 端用热点内 IP 连接

3. **USB/ADB（后续）**
   - PC 端运行 `adb forward tcp:<port> tcp:<port>`
   - 然后通过 `127.0.0.1:<port>` 连接

4. **蓝牙（后续）**
   - PC 端发起 RFCOMM 连接到 Android 的蓝牙监听

### 4.2 网络协议约定

- **音频流：** UDP 低延迟
- **包头 Magic：** `0x53424447` ("SBDG")
- **音频参数：** 48000 Hz / Mono / 960 samples/frame (20ms) / Float32
- **编码：** Opus
- **服务发现：** mDNS `_soundbridge._udp`

### 4.3 PC 端需要调用的 Rust FFI 函数

PC 端（Windows C++）通过 Rust FFI 与 Android 通信，核心函数：

```rust
// 引擎生命周期
sb_engine_create() -> *mut c_void
sb_engine_destroy(engine)
sb_bind(engine, port) -> c_int
sb_connect(engine, address: *const c_char) -> c_int

// 管线控制
sb_pipeline_start(engine) -> c_int
sb_pipeline_stop(engine) -> c_int

// 音频处理设置（与 Android 端对称）
sb_set_echo_cancellation_enabled(engine, enabled)
sb_set_noise_suppression_enabled(engine, enabled)
sb_set_agc_enabled(engine)
sb_set_sample_rate(rate)
sb_set_bitrate(bitrate)
sb_set_auto_profile_enabled(enabled)
sb_set_eq_enabled(enabled)
sb_set_eq_preset(preset)

// 静音/混音
sb_set_mute(engine, muted)
sb_set_mix_ratio(engine, pc_volume, phone_volume)

// 加密
sb_set_encryption_enabled(engine, enabled, key, salt)
```

---

## 五、已知限制

1. **蓝牙连接：** Android 端只启动了 RFCOMM 监听，等待 PC 端主动连接。配对流程未封装。
2. **WiFi 直连：** 热点创建依赖 Android 系统权限，部分设备可能失败。
3. **ADB 端口转发：** 需要 PC 端配合运行 `adb forward`。
4. **自动重连：** 仅对 WiFi 局域网模式有效，其他连接方式的重连逻辑未实现。
5. **设置页面 `engineHandle` 参数：** 当前未使用（设置通过全局静态函数调用），保留为未来扩展。

---

## 六、文件清单

### 本次 session 修改的文件

| 文件 | 改动 |
|------|------|
| `rust-core/crates/audio-processor/src/lib.rs` | 新增 `aec_enabled` / `ns_enabled` / `agc_enabled` 标志及 setter |
| `rust-core/crates/audio-codec/src/lib.rs` | Bitrate 枚举新增 Kbps192/320/512/1024 |
| `rust-core/crates/ffi-bindings/src/lib.rs` | 新增 3 个 FFI 函数 + 管线启动逻辑修改 |
| `android/app/src/main/cpp/jni_bridge.cpp` | Echo/NS/AGC 改为调用 Rust FFI |
| `android/app/src/main/java/.../audio/AudioService.kt` | 新增 ConnectionType 枚举 + connectToServer 分支 |
| `android/app/src/main/java/.../ui/HomeScreen.kt` | 连接方式芯片接入 + 主页可滚动 |
| `android/app/src/main/java/.../ui/SettingsScreen.kt` | 全部设置持久化 + native 同步 |
| `android/app/src/main/java/.../native/NativeAudioEngine.kt` | 已有 echo/noise/gain JNI 声明 |
| `android/app/src/main/res/values/strings.xml` | 新增 auto_mode 相关字符串 |
| `android/app/src/main/res/values-en/strings.xml` | 新增 auto_mode 英文字符串 |
