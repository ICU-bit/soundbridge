# SoundBridge API Reference

> C API 绑定，供 Windows C++/C# 和 Android JNI 调用。
> 源码：`rust-core/crates/ffi-bindings/src/lib.rs`
> 头文件：`rust-core/crates/ffi-bindings/include/soundbridge.h`

---

## 1. FFI 接口

所有函数返回 `int`（SbError 枚举），`0` 表示成功。失败时可通过 `sb_last_error()` 获取错误信息。

### 1.1 引擎生命周期

#### `sb_init()`

全局初始化。**必须在调用任何其他 `sb_*` 函数之前调用一次。**

- 安装 panic hook（panic 信息输出到 stderr）
- 初始化 tracing 日志系统（通过 `RUST_LOG` 环境变量控制级别）

重复调用是安全的（仅首次生效）。

```c
int sb_init(void);
```

| 返回值 | 含义 |
|--------|------|
| `SB_OK (0)` | 成功 |

#### `sb_version()`

获取 SoundBridge 版本字符串。返回静态指针，进程生命周期内有效。

```c
const char* sb_version(void);
```

#### `sb_engine_create()`

创建引擎实例，返回不透明句柄。失败时返回 `NULL`。

```c
void* sb_engine_create(void);
```

#### `sb_engine_destroy(engine)`

销毁引擎，释放所有资源（含停止运行中的管线）。

```c
void sb_engine_destroy(void* engine);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | `sb_engine_create` 返回的句柄 |

---

### 1.2 连接管理

#### `sb_bind(engine, port)`

绑定本地 UDP 端口。`port=0` 自动分配。

```c
int sb_bind(void* engine, uint16_t port);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | 引擎句柄 |
| `port` | `uint16_t` | 端口号，0=自动分配 |

#### `sb_connect(engine, addr)`

设置目标地址（远端），格式 `"ip:port"`。调用后状态变为 `Connecting`。

```c
int sb_connect(void* engine, const char* addr);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | 引擎句柄 |
| `addr` | `const char*` | 目标地址，如 `"192.168.1.100:5000"` |

#### `sb_local_port(engine, port)`

获取已绑定的本地端口号。

```c
int sb_local_port(void* engine, uint16_t* port);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | 引擎句柄 |
| `port` | `uint16_t*` | [out] 输出端口号 |

#### `sb_get_connection_state(engine, state)`

获取当前连接状态。

```c
int sb_get_connection_state(void* engine, SbConnectionState* state);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | 引擎句柄 |
| `state` | `SbConnectionState*` | [out] 输出状态 |

#### `sb_set_state_callback(engine, callback, user_data)`

注册连接状态变化回调。传 `NULL` 取消注册。

```c
int sb_set_state_callback(void* engine, SbStateCallback callback, void* user_data);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `engine` | `void*` | 引擎句柄 |
| `callback` | `SbStateCallback` | 回调函数，或 `NULL` |
| `user_data` | `void*` | 用户数据指针，原样传递给回调 |

#### `sb_set_connection_type(engine, conn_type)` / `sb_get_connection_type(engine, conn_type)`

设置/获取连接方式。

```c
int sb_set_connection_type(void* engine, int conn_type);
int sb_get_connection_type(void* engine, int* conn_type);
```

| 值 | 含义 |
|----|------|
| 0 | WiFi LAN（默认） |
| 1 | WiFi Direct |
| 2 | USB/ADB |
| 3 | Bluetooth |

---

### 1.3 音频控制

#### `sb_set_audio_mode(engine, mode)` / `sb_get_audio_mode(engine, mode)`

设置/获取音频模式。

```c
int sb_set_audio_mode(void* engine, SbAudioMode mode);
int sb_get_audio_mode(void* engine, SbAudioMode* mode);
```

| 模式 | 值 | 说明 |
|------|----|------|
| `SB_BALANCED` | 0 | 均衡模式（默认），50-100ms 延迟 |
| `SB_HIGH_QUALITY` | 1 | 高音质模式，48kHz/24bit，128kbps |
| `SB_LOW_LATENCY` | 2 | 超低延迟模式，<30ms |

#### `sb_set_mix_ratio(engine, pc_volume, phone_volume)` / `sb_get_mix_ratio(engine, pc_volume, phone_volume)`

设置/获取混音比例。范围 `0.0 ~ 1.0`。默认 `0.5 / 0.5`。

```c
int sb_set_mix_ratio(void* engine, float pc_volume, float phone_volume);
int sb_get_mix_ratio(void* engine, float* pc_volume, float* phone_volume);
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `pc_volume` | `float` | PC 音频音量 |
| `phone_volume` | `float` | 手机音频音量 |

#### `sb_set_mute(engine, muted)` / `sb_get_mute(engine)`

设置/获取静音状态。`1`=静音，`0`=取消静音。

```c
int sb_set_mute(void* engine, int muted);
int sb_get_mute(void* engine);  // 返回 0 或 1，负值=错误
```

#### `sb_get_audio_level(engine, level)`

获取当前采集音频电平（RMS，`0.0 ~ 1.0`）。管线未运行时返回 `0.0`。

```c
int sb_get_audio_level(void* engine, float* level);
```

#### `sb_set_exclusive_mode(engine, exclusive)`

设置 WASAPI 独占模式标志（影响延迟计算）。

```c
int sb_set_exclusive_mode(void* engine, bool exclusive);
```

---

### 1.4 音频管线

管线启动后自动运行两个线程：
- **发送线程**：采集 → 编码(Opus) → UDP 发送
- **接收线程**：UDP 接收 → Jitter Buffer → 解码 → 混音 → 播放

带宽自适应：根据丢包率动态调整码率（64/96/128 kbps）。

#### `sb_pipeline_start(engine)`

启动音频管线。前置条件：采集、播放、UDP 绑定、目标地址均已设置。

```c
int sb_pipeline_start(void* engine);
```

#### `sb_pipeline_stop(engine)`

停止音频管线（含采集和播放）。

```c
int sb_pipeline_stop(void* engine);
```

#### `sb_pipeline_state(engine, state)`

获取管线状态。

```c
int sb_pipeline_state(void* engine, int* state);
```

| 值 | 含义 |
|----|------|
| 0 | Stopped |
| 1 | Running |
| 2 | Error |

#### `sb_pipeline_stats(engine, frames_captured, frames_played, latency_ms, loss_rate)`

获取管线统计信息。

```c
int sb_pipeline_stats(void* engine,
                      uint64_t* frames_captured, uint64_t* frames_played,
                      float* latency_ms, float* loss_rate);
```

---

### 1.5 音频采集 / 播放

#### `sb_capture_start(engine, device_name)` / `sb_capture_stop(engine)`

启动/停止音频采集。`device_name=NULL` 使用默认设备。

```c
int sb_capture_start(void* engine, const char* device_name);
int sb_capture_stop(void* engine);
```

#### `sb_capture_read(engine, buf, len)`

读取采集的音频样本。返回实际读取的样本数（负值=错误）。

```c
int sb_capture_read(void* engine, float* buf, size_t len);
```

#### `sb_capture_device_count(count)`

获取采集设备数量。

```c
int sb_capture_device_count(size_t* count);
```

#### `sb_playback_start(engine, device_name)` / `sb_playback_stop(engine)`

启动/停止音频播放。`device_name=NULL` 使用默认设备。

```c
int sb_playback_start(void* engine, const char* device_name);
int sb_playback_stop(void* engine);
```

#### `sb_playback_write(engine, buf, len)`

写入音频数据到播放缓冲区。

```c
int sb_playback_write(void* engine, const float* buf, size_t len);
```

#### `sb_playback_device_count(count)`

获取播放设备数量。

```c
int sb_playback_device_count(size_t* count);
```

#### `sb_mixer_mix(engine, inputs, input_lens, volumes, input_count, output, output_len)`

混音多路音频，各路独立音量控制。

```c
int sb_mixer_mix(void* engine,
                 const float** inputs, const size_t* input_lens,
                 const float* volumes, size_t input_count,
                 float* output, size_t output_len);
```

#### `sb_processor_process(engine, buf, len)`

就地处理音频（AEC/NS/AGC）。

```c
int sb_processor_process(void* engine, float* buf, size_t len);
```

---

### 1.6 设备管理

#### 设备发现（mDNS）

```c
void* sb_discovery_create(void);                           // 创建发现实例
void  sb_discovery_close(void* discovery);                 // 关闭
int   sb_discovery_init(void* discovery);                  // 初始化 mDNS
int   sb_discovery_register(void* discovery,               // 注册本机服务
                            const char* name, uint16_t port);
int   sb_discovery_find_devices(void* discovery,           // 搜索设备
                                void** devices_buf, size_t buf_size);
void  sb_discovery_free_device_info(void* device_info);    // 释放设备信息
```

#### 设备存储（JSON 持久化）

```c
void* sb_device_store_open(const char* path);              // 打开/创建存储
void  sb_device_store_close(void* store);                  // 关闭
int   sb_device_store_add(void* store,                     // 添加设备
                          const char* name, const char* address, uint16_t port);
int   sb_device_store_remove(void* store, const char* name);  // 移除设备
int   sb_device_store_set_auto_connect(void* store,        // 设置自动连接
                                       const char* name, bool auto_connect);
int   sb_device_store_count(void* store, size_t* count);   // 设备数量
int   sb_device_store_has(void* store, const char* name);  // 是否存在
void  sb_device_store_clear(void* store);                  // 清空
int   sb_device_store_get_address(void* store,             // 获取地址
                                  const char* name, char* buf, size_t buf_len);
int   sb_device_store_get_port(void* store,                // 获取端口
                               const char* name, uint16_t* port);
int   sb_device_store_get_name_at(void* store,             // 按索引获取名称
                                  size_t index, char* buf, size_t buf_len);
```

---

### 1.7 网络连接

#### WiFi Direct 热点

```c
int sb_hotspot_create(void* engine, const char* ssid,      // 创建热点
                      const char* password, uint32_t channel);
int sb_hotspot_destroy(void* engine);                      // 销毁热点
int sb_hotspot_state(void* engine, int32_t* state);        // 获取热点状态
int sb_hotspot_set_state(void* engine, int32_t state);     // 设置热点状态
```

#### USB/ADB 连接

```c
int sb_adb_setup_port_forward(void* engine,                // 设置端口转发
                              uint32_t local_port, uint32_t remote_port,
                              const char* device_serial);
int sb_adb_state(void* engine, int32_t* state);            // 获取 ADB 状态
int sb_adb_set_state(void* engine, int32_t state);         // 设置 ADB 状态
```

#### 蓝牙连接

```c
int sb_bt_init(void* engine, const char* device_name,      // 初始化蓝牙
               bool use_ble);
int sb_bt_state(void* engine, int32_t* state);             // 获取蓝牙状态
int sb_bt_set_state(void* engine, int32_t state);          // 设置蓝牙状态
```

---

### 1.8 加密（DTLS/SRTP）

```c
int sb_enable_encryption(void* engine,                     // 启用加密
                         const uint8_t* master_key,        // 16 字节
                         const uint8_t* master_salt);      // 14 字节
int sb_disable_encryption(void* engine);                   // 禁用加密
int sb_is_encrypted(void* engine);                         // 查询状态（1=启用）
```

### 1.9 双向控制

```c
int sb_send_volume(void* engine, float volume);            // 发送音量到远端
int sb_send_pause(void* engine);                           // 发送暂停命令
int sb_send_resume(void* engine);                          // 发送恢复命令
```

---

## 2. 错误码

```c
typedef enum SbError {
    SB_OK                 =  0,   // 成功
    SB_ERROR              = -1,   // 通用错误
    SB_INVALID_ARGUMENT   = -2,   // 无效参数（null 指针等）
    SB_DEVICE_NOT_FOUND   = -3,   // 设备未找到
    SB_CONFIG_ERROR       = -4,   // 配置错误
    SB_STREAM_ERROR       = -5,   // 流错误（采集/播放失败）
    SB_CODEC_ERROR        = -6,   // 编解码错误（Opus）
    SB_NETWORK_ERROR      = -7,   // 网络错误（绑定/发送失败）
    SB_PIPELINE_NOT_READY = -8,   // 管线未就绪（缺少前置条件）
} SbError;
```

#### 错误信息

```c
const char* sb_last_error(void);  // 获取最后的错误信息，无错误时返回 NULL
```

返回的指针在下次调用 FFI 函数前有效。

---

## 3. 枚举类型

### SbConnectionState

```c
typedef enum SbConnectionState {
    SB_DISCONNECTED = 0,   // 未连接
    SB_CONNECTING   = 1,   // 正在连接
    SB_CONNECTED    = 2,   // 已连接
    SB_ERROR_STATE  = 3,   // 连接错误
} SbConnectionState;
```

### SbAudioMode

```c
typedef enum SbAudioMode {
    SB_BALANCED     = 0,   // 均衡模式（默认），50-100ms
    SB_HIGH_QUALITY = 1,   // 高音质，48kHz，128kbps
    SB_LOW_LATENCY  = 2,   // 超低延迟，<30ms
} SbAudioMode;
```

---

## 4. 回调函数

### SbStateCallback

连接状态变化时触发。

```c
typedef void (*SbStateCallback)(SbConnectionState state, void* user_data);
```

| 参数 | 说明 |
|------|------|
| `state` | 新的连接状态 |
| `user_data` | 注册时传入的用户数据指针 |

**使用注意**：
- 回调在管线线程中触发，不要执行阻塞操作
- 引擎销毁后回调不再触发
- 传 `NULL` 可取消注册

---

## 5. 典型使用流程

```c
// 1. 初始化
sb_init();

// 2. 创建引擎
void* engine = sb_engine_create();

// 3. 设置回调
sb_set_state_callback(engine, on_state_change, user_data);

// 4. 绑定 & 连接
sb_bind(engine, 0);                          // 自动端口
sb_connect(engine, "192.168.1.100:5000");

// 5. 启动采集 & 播放
sb_capture_start(engine, NULL);              // 默认设备
sb_playback_start(engine, NULL);

// 6. 启动管线
sb_pipeline_start(engine);

// 7. 运行中可调用
sb_set_mix_ratio(engine, 0.6, 0.4);         // 调整混音
sb_set_audio_mode(engine, SB_LOW_LATENCY);  // 切换模式
sb_set_mute(engine, 1);                     // 静音

// 8. 清理
sb_pipeline_stop(engine);
sb_engine_destroy(engine);
```
