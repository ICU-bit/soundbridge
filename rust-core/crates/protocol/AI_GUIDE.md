# Protocol Crate

## Purpose

协议定义模块，实现音频数据包和控制数据包的序列化/反序列化。

## Current Status

- ✅ 音频数据包格式定义（12 字节头）
- ✅ 控制数据包格式定义
- ✅ 序列化和反序列化实现（零拷贝）
- ✅ 数据包校验（魔术数 0x53424447）
- ✅ 8 个单元测试通过
- ✅ 29 个集成测试通过（PacketType、PacketHeader、ControlMessageType、ControlMessage、Protocol、Packet、ProtocolError、常量）

## Architecture

```
AudioPacket         - 音频数据包（12 字节头 + Opus 数据）
ControlPacket       - 控制数据包
PacketHeader        - 包头结构（magic, seq, timestamp, flags, len）
```

## Packet Format

```
┌────────────────────────────────────────────┐
│ Byte 0-3:  Magic (0x53424447 "SBDG")      │
│ Byte 4-7:  Sequence (uint32, big-endian)  │
│ Byte 8-11: Length (uint32, big-endian)    │
│ Byte 12-N: Payload (Opus encoded data)    │
└────────────────────────────────────────────┘
```

## Dependencies

- bytes (workspace)
- audio-core (workspace)
