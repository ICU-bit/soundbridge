# Network Crate

## Purpose

网络传输模块，实现 UDP 音频流传输、连接管理和 DTLS/SRTP 加密。

## Current Status

- ✅ UDP 传输实现完成（tokio::net::UdpSocket）
- ✅ 连接管理（ConnectionType: WiFiLan, WiFiDirect, UsbAdb, Bluetooth）
- ✅ HotspotConfig/HotspotState（WiFi Direct 热点管理）
- ✅ AdbConfig/AdbState（USB/ADB 端口转发）
- ✅ BluetoothConfig/BluetoothState（蓝牙连接管理）
- ✅ 零拷贝序列化
- ✅ 带宽自适应（丢包率检测 + 码率调整）
- ✅ DTLS/SRTP 加密层
  - AES-128-CM 加密（Ctr32BE<Aes128>）
  - HMAC-SHA1-80 认证标签（10 字节）
  - SRTP 密钥派生函数（KDF，RFC 3711）
  - 密钥轮换（每 2^31 包）
  - DTLS 握手状态机（简化实现）
  - HKDF-SHA1 会话密钥派生
  - 常量时间认证标签比较（防时序攻击）
- ✅ QUIC 控制信令通道
  - QuicServer / QuicClient（quinn 0.10）
  - ControlMessage 枚举（SessionControl, AudioConfig, NetworkStats, DeviceDiscovery）
  - 自签名证书生成（rcgen 0.11）
  - 长度前缀帧协议（4 字节 u32 + bincode 载荷）
  - 双向流请求/响应 + 单向流推送
- ✅ 38 个 crypto 单元测试通过
- ✅ 17 个 QUIC 单元测试通过
- ✅ 零 clippy 警告

## Architecture

```
UdpTransport        - UDP 传输实现
ConnectionType      - 连接类型枚举
HotspotConfig/State - WiFi Direct 热点配置/状态
AdbConfig/State     - USB/ADB 配置/状态
BluetoothConfig/State - 蓝牙配置/状态
SrtpContext         - SRTP 加密/解密上下文
DtlsSession         - DTLS 握手状态机
CryptoKeys          - 密钥材料（master_key + master_salt）
DtlsConfig          - DTLS 配置
QuicServer          - QUIC 控制信令服务器
QuicClient          - QUIC 控制信令客户端
QuicConnection      - QUIC 连接封装（双向流 + 单向流）
ControlMessage      - 控制消息枚举（会话/音频/统计/发现）
```

## Note

DTLS 实现为简化版本，适用于局域网场景。生产环境应考虑使用 rustls 等成熟库。

QUIC 控制信令使用 quinn 0.10 + rustls 0.21，提供可靠加密的控制通道。
音频流仍使用 UDP 传输（低延迟），控制信令走 QUIC（可靠有序）。

## Dependencies

- tokio (workspace)
- audio-core (workspace)
- quinn (workspace) - QUIC 协议实现
- quinn-proto (workspace) - QUIC 协议底层（rustls feature）
- rustls (workspace) - TLS 1.3
- rcgen (workspace) - 自签名证书生成
- serde (workspace) - 序列化框架
- bincode (workspace) - 二进制序列化
- aes (workspace) - AES-128 块加密
- ctr (workspace) - CTR 模式
- hmac (workspace) - HMAC
- sha1 (workspace) - SHA1 哈希
- hkdf (workspace) - HKDF 密钥派生
- rand (workspace) - 随机数生成
