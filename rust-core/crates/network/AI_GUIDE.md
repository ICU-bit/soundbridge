# Network Crate

## Purpose

网络传输模块，实现 UDP 音频流传输和连接管理。

## Current Status

- ✅ UDP 传输实现完成（tokio::net::UdpSocket）
- ✅ 连接管理（ConnectionType: WiFiLan, WiFiDirect, UsbAdb, Bluetooth）
- ✅ HotspotConfig/HotspotState（WiFi Direct 热点管理）
- ✅ AdbConfig/AdbState（USB/ADB 端口转发）
- ✅ BluetoothConfig/BluetoothState（蓝牙连接管理）
- ✅ 零拷贝序列化
- ✅ 带宽自适应（丢包率检测 + 码率调整）
- ✅ 26 个单元测试通过
- ✅ 34 个集成测试通过（RawJitterBuffer、JitterBuffer、ConnectionManager、ConnectionType、HotspotConfig/State、AdbConfig/State、BluetoothConfig/State、TransportConfig、NetworkError）

## Architecture

```
UdpTransport        - UDP 传输实现
ConnectionType      - 连接类型枚举
HotspotConfig/State - WiFi Direct 热点配置/状态
AdbConfig/State     - USB/ADB 配置/状态
BluetoothConfig/State - 蓝牙配置/状态
```

## Note

QUIC 控制信令尚未实现。当前仅使用 UDP 传输音频流。

## Dependencies

- tokio (workspace)
- audio-core (workspace)
