//! SoundBridge 协议模块
//!
//! 提供数据包序列化和反序列化功能。

/// 协议错误类型
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("无效的魔术数")]
    InvalidMagic,

    #[error("无效的版本: {0}")]
    InvalidVersion(u8),

    #[error("无效的包类型: {0}")]
    InvalidPacketType(u8),

    #[error("数据过短: 需要 {needed} 字节，实际 {actual} 字节")]
    DataTooShort { needed: usize, actual: usize },

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("反序列化错误: {0}")]
    DeserializationError(String),
}

/// 协议结果类型
pub type Result<T> = std::result::Result<T, ProtocolError>;

/// 魔术数
pub const MAGIC: u32 = 0x53424447; // "SBDG"

/// 协议版本
pub const PROTOCOL_VERSION: u8 = 1;

/// 包类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// 设备发现请求
    DiscoverRequest = 0x01,

    /// 设备发现响应
    DiscoverResponse = 0x02,

    /// 握手请求
    HandshakeRequest = 0x10,

    /// 握手响应
    HandshakeResponse = 0x11,

    /// 开始流传输
    StartStream = 0x20,

    /// 开始流传输确认
    StartStreamAck = 0x21,

    /// 音频数据
    AudioData = 0x30,

    /// 音频 RTCP
    AudioRtcp = 0x31,

    /// 控制模式切换
    ControlModeSwitch = 0x40,

    /// 控制参数更新
    ControlParamUpdate = 0x41,

    /// 心跳
    Heartbeat = 0xF0,

    /// 心跳确认
    HeartbeatAck = 0xF1,
}

impl PacketType {
    /// 从 u8 转换
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::DiscoverRequest),
            0x02 => Ok(Self::DiscoverResponse),
            0x10 => Ok(Self::HandshakeRequest),
            0x11 => Ok(Self::HandshakeResponse),
            0x20 => Ok(Self::StartStream),
            0x21 => Ok(Self::StartStreamAck),
            0x30 => Ok(Self::AudioData),
            0x31 => Ok(Self::AudioRtcp),
            0x40 => Ok(Self::ControlModeSwitch),
            0x41 => Ok(Self::ControlParamUpdate),
            0xF0 => Ok(Self::Heartbeat),
            0xF1 => Ok(Self::HeartbeatAck),
            _ => Err(ProtocolError::InvalidPacketType(value)),
        }
    }
}

/// 包头（对齐技术规格）
#[derive(Debug, Clone)]
pub struct PacketHeader {
    /// 序列号
    pub sequence: u32,

    /// 时间戳（毫秒）
    pub timestamp_ms: u32,

    /// 标志位
    pub flags: u8,

    /// 通道数
    pub channels: u8,

    /// Opus 数据长度
    pub opus_length: u16,
}

impl PacketHeader {
    /// 包头大小（字节）
    /// sequence(4) + timestamp(4) + flags(1) + channels(1) + opus_length(2) = 12
    pub const SIZE: usize = 12;

    /// 编码包头（网络字节序 = 大端）
    pub fn encode(&self, buf: &mut Vec<u8>) -> Result<()> {
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.timestamp_ms.to_be_bytes());
        buf.push(self.flags);
        buf.push(self.channels);
        buf.extend_from_slice(&self.opus_length.to_be_bytes());
        Ok(())
    }

    /// 解码包头（网络字节序 = 大端）
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < Self::SIZE {
            return Err(ProtocolError::DataTooShort {
                needed: Self::SIZE,
                actual: buf.len(),
            });
        }

        let sequence = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let timestamp_ms = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let flags = buf[8];
        let channels = buf[9];
        let opus_length = u16::from_be_bytes([buf[10], buf[11]]);

        Ok((
            Self {
                sequence,
                timestamp_ms,
                flags,
                channels,
                opus_length,
            },
            Self::SIZE,
        ))
    }
}

/// 协议处理器
pub struct Protocol {
    _private: (),
}

impl Default for Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Protocol {
    /// 创建新的协议处理器
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// 序列化包
    pub fn serialize(&self, packet: &Packet) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        match packet {
            Packet::Audio { header, data } => {
                header.encode(&mut buf)?;
                buf.extend_from_slice(data);
            }
            Packet::Control { header, data } => {
                header.encode(&mut buf)?;
                buf.extend_from_slice(data);
            }
        }

        Ok(buf)
    }

    /// 反序列化包
    pub fn deserialize(&self, data: &[u8]) -> Result<Packet> {
        let (header, _) = PacketHeader::decode(data)?;

        let payload_start = PacketHeader::SIZE;
        let payload_end = payload_start + header.opus_length as usize;

        if data.len() < payload_end {
            return Err(ProtocolError::DataTooShort {
                needed: payload_end,
                actual: data.len(),
            });
        }

        let payload = data[payload_start..payload_end].to_vec();

        // 根据标志位判断是否为音频数据
        if header.flags & 0x01 != 0 {
            Ok(Packet::Audio {
                header,
                data: payload,
            })
        } else {
            Ok(Packet::Control {
                header,
                data: payload,
            })
        }
    }
}

/// 控制消息类型（对齐技术规格 §2.2.2）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlMessageType {
    /// 握手请求
    Hello = 0x01,

    /// 握手响应
    HelloAck = 0x21,

    /// 认证请求
    Auth = 0x02,

    /// 认证响应
    AuthAck = 0x22,

    /// 开始音频传输
    StartAudio = 0x03,

    /// 停止音频传输
    StopAudio = 0x04,

    /// 切换音频模式
    ChangeMode = 0x05,

    /// 音量控制
    Volume = 0x06,

    /// 状态查询
    Status = 0x07,

    /// 状态响应
    StatusAck = 0x27,

    /// 心跳
    Heartbeat = 0x08,

    /// 心跳响应
    HeartbeatAck = 0x28,

    /// 错误
    Error = 0xFF,
}

impl ControlMessageType {
    /// 从 u8 转换
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::Hello),
            0x21 => Ok(Self::HelloAck),
            0x02 => Ok(Self::Auth),
            0x22 => Ok(Self::AuthAck),
            0x03 => Ok(Self::StartAudio),
            0x04 => Ok(Self::StopAudio),
            0x05 => Ok(Self::ChangeMode),
            0x06 => Ok(Self::Volume),
            0x07 => Ok(Self::Status),
            0x27 => Ok(Self::StatusAck),
            0x08 => Ok(Self::Heartbeat),
            0x28 => Ok(Self::HeartbeatAck),
            0xFF => Ok(Self::Error),
            _ => Err(ProtocolError::InvalidPacketType(value)),
        }
    }
}

/// 控制消息（对齐技术规格 §2.2.1）
#[derive(Debug, Clone)]
pub struct ControlMessage {
    /// 消息类型
    pub message_type: ControlMessageType,

    /// 消息体
    pub payload: Vec<u8>,
}

impl ControlMessage {
    /// 消息头大小：长度(4) + 类型(1) = 5 字节
    const HEADER_SIZE: usize = 5;

    /// 编码消息
    pub fn encode(&self, buf: &mut Vec<u8>) -> Result<()> {
        let length = Self::HEADER_SIZE as u32 + self.payload.len() as u32;
        buf.extend_from_slice(&length.to_be_bytes());
        buf.push(self.message_type as u8);
        buf.extend_from_slice(&self.payload);
        Ok(())
    }

    /// 解码消息
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < Self::HEADER_SIZE {
            return Err(ProtocolError::DataTooShort {
                needed: Self::HEADER_SIZE,
                actual: buf.len(),
            });
        }

        let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if buf.len() < length {
            return Err(ProtocolError::DataTooShort {
                needed: length,
                actual: buf.len(),
            });
        }

        let message_type = ControlMessageType::from_u8(buf[4])?;
        let payload = buf[Self::HEADER_SIZE..length].to_vec();

        Ok((Self { message_type, payload }, length))
    }
}

/// 数据包
#[derive(Debug, Clone)]
pub enum Packet {
    /// 音频数据
    Audio {
        header: PacketHeader,
        data: Vec<u8>,
    },

    /// 控制数据
    Control {
        header: PacketHeader,
        data: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type_from_u8() {
        assert_eq!(PacketType::from_u8(0x01).unwrap(), PacketType::DiscoverRequest);
        assert_eq!(PacketType::from_u8(0x30).unwrap(), PacketType::AudioData);
        assert!(PacketType::from_u8(0xFF).is_err());
    }

    #[test]
    fn test_packet_header_encode_decode() {
        let header = PacketHeader {
            sequence: 42,
            timestamp_ms: 1234567890,
            flags: 0x01, // 音频数据标志
            channels: 2,
            opus_length: 960,
        };

        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();

        assert_eq!(buf.len(), PacketHeader::SIZE);

        let (decoded, size) = PacketHeader::decode(&buf).unwrap();
        assert_eq!(size, PacketHeader::SIZE);
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.timestamp_ms, 1234567890);
        assert_eq!(decoded.flags, 0x01);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.opus_length, 960);
    }

    #[test]
    fn test_invalid_data_too_short() {
        let buf = vec![0u8; 5];
        assert!(PacketHeader::decode(&buf).is_err());
    }

    #[test]
    fn test_serialize_deserialize_audio() {
        let protocol = Protocol::new();

        let header = PacketHeader {
            sequence: 1,
            timestamp_ms: 1000,
            flags: 0x01, // 音频数据
            channels: 2,
            opus_length: 4,
        };

        let packet = Packet::Audio {
            header,
            data: vec![0x01, 0x02, 0x03, 0x04],
        };

        let serialized = protocol.serialize(&packet).unwrap();
        assert_eq!(serialized.len(), PacketHeader::SIZE + 4);

        let deserialized = protocol.deserialize(&serialized).unwrap();
        match deserialized {
            Packet::Audio { header, data } => {
                assert_eq!(header.sequence, 1);
                assert_eq!(header.timestamp_ms, 1000);
                assert_eq!(header.flags, 0x01);
                assert_eq!(data, vec![0x01, 0x02, 0x03, 0x04]);
            }
            _ => panic!("Expected Audio packet"),
        }
    }

    #[test]
    fn test_serialize_deserialize_control() {
        let protocol = Protocol::new();

        let header = PacketHeader {
            sequence: 2,
            timestamp_ms: 2000,
            flags: 0x00, // 控制数据
            channels: 0,
            opus_length: 2,
        };

        let packet = Packet::Control {
            header,
            data: vec![0xAA, 0xBB],
        };

        let serialized = protocol.serialize(&packet).unwrap();
        let deserialized = protocol.deserialize(&serialized).unwrap();

        match deserialized {
            Packet::Control { header, data } => {
                assert_eq!(header.sequence, 2);
                assert_eq!(header.timestamp_ms, 2000);
                assert_eq!(data, vec![0xAA, 0xBB]);
            }
            _ => panic!("Expected Control packet"),
        }
    }

    #[test]
    fn test_control_message_type() {
        assert_eq!(ControlMessageType::from_u8(0x01).unwrap(), ControlMessageType::Hello);
        assert_eq!(ControlMessageType::from_u8(0x21).unwrap(), ControlMessageType::HelloAck);
        assert_eq!(ControlMessageType::from_u8(0x08).unwrap(), ControlMessageType::Heartbeat);
        assert!(ControlMessageType::from_u8(0xFE).is_err());
    }

    #[test]
    fn test_control_message_encode_decode() {
        let msg = ControlMessage {
            message_type: ControlMessageType::Hello,
            payload: vec![0x01, 0x02, 0x03],
        };

        let mut buf = Vec::new();
        msg.encode(&mut buf).unwrap();

        assert_eq!(buf.len(), 8); // length(4) + type(1) + payload(3)

        let (decoded, size) = ControlMessage::decode(&buf).unwrap();
        assert_eq!(size, 8);
        assert_eq!(decoded.message_type, ControlMessageType::Hello);
        assert_eq!(decoded.payload, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_control_message_heartbeat() {
        let msg = ControlMessage {
            message_type: ControlMessageType::Heartbeat,
            payload: vec![],
        };

        let mut buf = Vec::new();
        msg.encode(&mut buf).unwrap();

        let (decoded, _) = ControlMessage::decode(&buf).unwrap();
        assert_eq!(decoded.message_type, ControlMessageType::Heartbeat);
        assert!(decoded.payload.is_empty());
    }
}
