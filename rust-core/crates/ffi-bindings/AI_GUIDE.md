# FFI Bindings Crate

## Purpose

跨语言绑定模块，提供 C API 供 Windows C++/C# 和 Android JNI 调用。

## Current Status

- ✅ C 语言 API 绑定完成（76 个 sb_* 函数）
- ✅ 完整管线：capture→encode→send + recv→decode→play
- ✅ RawJitterBuffer 集成（乱序容忍）
- ✅ 音频模式切换（均衡/高音质/超低延迟）
- ✅ 混音比例控制（sb_set_mix_ratio / sb_get_mix_ratio）
- ✅ 连接类型管理（WiFiLan, WiFiDirect, UsbAdb, Bluetooth）
- ✅ WiFi Direct 热点管理（sb_hotspot_*）
- ✅ USB/ADB 端口转发（sb_adb_*）
- ✅ 蓝牙连接管理（sb_bt_*）
- ✅ 真实音频电平（sb_get_audio_level，RMS 计算）
- ✅ 独占模式延迟自适应（sb_set_exclusive_mode）
- ✅ 带宽自适应（丢包率动态调整码率 64/96/128kbps）
- ✅ 76 个测试通过，零 clippy 警告

## Architecture

```
SbEngine            - 主引擎结构体（opaque handle）
SharedPipelineStats - 共享管线统计（frames_encoded/decoded, loss_rate, level, exclusive_mode）
AudioModeManager    - 音频模式管理器
RawJitterBuffer     - 原始 Opus 字节 Jitter Buffer
```

## Key FFI Functions

```c
// Lifecycle
SbEngine* sb_create(void);
void sb_destroy(SbEngine* engine);
int sb_start(SbEngine* engine, const char* peer_addr, int local_port);
int sb_stop(SbEngine* engine);

// Audio
int sb_set_audio_mode(SbEngine* engine, int mode);
int sb_set_mix_ratio(SbEngine* engine, int pc_volume, int phone_volume);
float sb_get_audio_level(SbEngine* engine);
int sb_set_exclusive_mode(SbEngine* engine, int exclusive);

// Connection
int sb_set_connection_type(SbEngine* engine, int type);
int sb_get_connection_type(SbEngine* engine);

// Hotspot/ADB/Bluetooth
int sb_hotspot_create(SbEngine* engine, const char* ssid, const char* password);
int sb_hotspot_destroy(SbEngine* engine);
int sb_hotspot_state(SbEngine* engine);
int sb_adb_setup_port_forward(SbEngine* engine, int port);
int sb_adb_state(SbEngine* engine);
int sb_bt_init(SbEngine* engine, const char* device_name);
int sb_bt_state(SbEngine* engine);
```

## Dependencies

- audio-core, audio-codec, audio-capture, audio-playback, audio-mixer, audio-processor
- network, discovery, protocol
- opus, cpal, tokio (workspace)
