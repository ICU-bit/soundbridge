# Discovery Crate

## Purpose

设备发现模块，基于 mDNS 实现局域网设备自动发现。

## Current Status

- ✅ 基于 mDNS 的设备发现实现完成（mdns_sd 库）
- ✅ 设备广播（mDNS 通告）
- ✅ 设备搜索（mDNS 查询）
- ✅ 设备信息管理
- ✅ 测试用例通过

## Architecture

```
MdnsDiscovery       - mDNS 设备发现实现
DeviceInfo          - 设备信息结构体
```

## mDNS Service

- Service Type: `_soundbridge._udp.local.`
- Protocol: UDP

## Dependencies

- mdns_sd (workspace)
- audio-core (workspace)
