//! # SoundBridge 网络传输模块
//!
//! 本 crate 提供 SoundBridge 跨端音频融合软件的完整网络传输层，包括：
//!
//! - **UDP 音频传输**（[`transport`]）— 低延迟 UDP 数据报收发，支持可选 SRTP 加密
//! - **DTLS/SRTP 加密**（[`crypto`]）— AES-128-CM + HMAC-SHA1-80 端到端加密
//! - **会话握手协议**（[`session`]）— 状态机驱动的能力协商、心跳检测、优雅断开
//! - **QUIC 控制信令**（[`quic_control`]）— 基于 QUIC 的可靠加密控制通道
//! - **自适应 Jitter Buffer**（[`jitter_buffer`]）— 基于网络抖动动态调整延迟
//! - **网络状况监控**（[`net_monitor`]）— RTT、丢包率、带宽估计
//! - **连接管理**（[`connection`]）— WiFi/USB/蓝牙多连接方式
//!
//! ## 跨平台统一参数
//!
//! | 参数 | 值 |
//! |------|------|
//! | 采样率 | 48000 Hz |
//! | 通道 | 单声道（Mono） |
//! | 帧大小 | 960 samples（20ms@48kHz） |
//!
//! ## 安全功能
//!
//! 本模块提供三层安全机制：
//!
//! 1. **DTLS 握手** — 自签名证书交换，密钥派生（HKDF-SHA1）
//! 2. **SRTP 加密** — AES-128-CTR 加密 + HMAC-SHA1-80 认证，密钥轮换
//! 3. **QUIC 控制通道** — TLS 1.3 加密的可靠控制信令
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use network::{UdpTransport, TransportConfig, CryptoKeys};
//!
//! # async fn example() -> network::Result<()> {
//! // 创建 UDP 传输（不加密）
//! let transport = UdpTransport::new(TransportConfig::default()).await?;
//!
//! // 如需加密，启用 SRTP
//! let mut transport = UdpTransport::new(TransportConfig::default()).await?;
//! let master_key = vec![0u8; 16];
//! let master_salt = vec![0u8; 14];
//! transport.enable_encryption(master_key, master_salt)?;
//! # Ok(())
//! # }
//! ```

pub mod bandwidth_pid;
pub mod connection;
pub mod crypto;
pub mod jitter_buffer;
pub mod net_monitor;
pub mod quic_control;
pub mod reconnect;
pub mod session;
pub mod transport;

pub use bandwidth_pid::{NetworkMetrics, PidBandwidthController, PidConfig};
pub use connection::{
    AdbConfig, AdbState, BluetoothConfig, BluetoothState, ConnectionConfig, ConnectionManager,
    ConnectionState, ConnectionType, HotspotConfig, HotspotState,
};
pub use crypto::{CryptoKeys, DtlsConfig, DtlsSession, DtlsState, SrtpContext, SRTP_AUTH_TAG_LEN};
pub use jitter_buffer::{
    AdaptiveConfig, AudioPacket, JitterBuffer, JitterBufferConfig, JitterStats, NetworkQuality,
    RawAudioPacket, RawJitterBuffer,
};
pub use net_monitor::{
    BitrateRecommendation, BurstLossEvent, NetMonitor, NetMonitorConfig, NetworkStats,
};
pub use quic_control::{
    AudioConfig, ControlMessage, DeviceInfo, NetworkStatsData, QuicClient, QuicConnection,
    QuicServer,
};
pub use reconnect::{ReconnectConfig, ReconnectManager, ReconnectState, ReconnectStats};
pub use session::{
    generate_session_id, Capability, DisconnectReason, EcdhPublicKey, EncryptionMode,
    HandshakeMessage, NegotiatedParams, OpusConfig, Session, SessionConfig, SessionRole,
    SessionState, SessionStats, TransportProtocol,
};
pub use transport::{TransportConfig, UdpTransport};

/// 网络错误类型
///
/// 涵盖网络传输层所有可能的错误场景，包括连接、发送、接收、加密等。
///
/// # 示例
///
/// ```rust
/// use network::NetworkError;
///
/// let err = NetworkError::Timeout;
/// assert_eq!(err.to_string(), "连接超时");
///
/// let err = NetworkError::ConnectionFailed("设备离线".into());
/// assert!(err.to_string().contains("设备离线"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// 连接失败，包含具体原因描述
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    /// 数据发送失败（UDP/QUIC 发送错误）
    #[error("发送失败: {0}")]
    SendFailed(String),

    /// 数据接收失败（UDP/QUIC 接收错误）
    #[error("接收失败: {0}")]
    ReceiveFailed(String),

    /// Socket 绑定失败（端口占用或权限不足）
    #[error("绑定失败: {0}")]
    BindFailed(String),

    /// 连接超时（握手或心跳超时）
    #[error("连接超时")]
    Timeout,

    /// 连接已断开（对端关闭或网络中断）
    #[error("连接已断开")]
    Disconnected,

    /// 底层 IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// QUIC 协议错误（TLS 证书、连接失败等）
    #[error("QUIC 错误: {0}")]
    QuicError(String),

    /// 消息序列化/反序列化错误（bincode 编解码失败）
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 加密/解密错误（密钥无效、认证标签验证失败等）
    #[error("加密错误: {0}")]
    CryptoError(String),
}

/// 网络结果类型别名
///
/// 所有网络模块函数统一使用此类型作为返回值。
///
/// # 示例
///
/// ```rust
/// use network::{Result, NetworkError};
///
/// fn example() -> Result<u32> {
///     Ok(42)
/// }
///
/// fn failing() -> Result<()> {
///     Err(NetworkError::Timeout)
/// }
/// ```
pub type Result<T> = std::result::Result<T, NetworkError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_buffer() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(1, vec![1.0f32; 100]);
        jb.push(2, vec![2.0f32; 100]);
        jb.push(3, vec![3.0f32; 100]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_jitter_buffer_reorder() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // push 1 first so next_sequence initializes to 1
        jb.push(1, vec![1.0f32; 100]);
        jb.push(3, vec![3.0f32; 100]);
        jb.push(2, vec![2.0f32; 100]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 3);
    }

    #[test]
    fn test_jitter_buffer_empty() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);
        assert!(jb.pop().is_none());
    }

    #[test]
    fn test_jitter_buffer_stats() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(1, vec![1.0f32; 100]);
        jb.push(2, vec![2.0f32; 100]);

        assert_eq!(jb.len(), 2);
        assert!(!jb.is_empty());

        jb.pop();
        assert_eq!(jb.len(), 1);
    }

    #[test]
    fn test_connection_manager() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        assert_eq!(manager.state(), ConnectionState::Disconnected);
        assert!(!manager.is_connected());
    }

    #[test]
    fn test_connection_state() {
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
    }

    #[test]
    fn test_raw_jitter_buffer_basic() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.push(1, 100, vec![0x01, 0x02]);
        jb.push(2, 200, vec![0x03, 0x04]);
        jb.push(3, 300, vec![0x05, 0x06]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);
        assert_eq!(packet.timestamp, 100);
        assert_eq!(packet.data, vec![0x01, 0x02]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_raw_jitter_buffer_reorder() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        // push 1 first so next_sequence initializes to 1
        jb.push(1, 100, vec![0x01, 0x02]);
        jb.push(3, 300, vec![0x05, 0x06]);
        jb.push(2, 200, vec![0x03, 0x04]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 3);
    }

    #[test]
    fn test_raw_jitter_buffer_empty() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);
        assert!(jb.pop().is_none());
        assert!(jb.is_empty());
        assert_eq!(jb.len(), 0);
    }

    #[test]
    fn test_raw_jitter_buffer_skip_missing() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        // 跳过序列号 1，直接推入 2 和 3
        jb.push(2, 200, vec![0x03, 0x04]);
        jb.push(3, 300, vec![0x05, 0x06]);

        // 第一次 pop 应该跳到 2（跳过缺失的 1）
        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
        assert_eq!(jb.next_sequence(), 3);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 3);
    }

    #[test]
    fn test_raw_jitter_buffer_overflow() {
        let config = JitterBufferConfig {
            max_packets: 3,
            ..Default::default()
        };
        let mut jb = RawJitterBuffer::new(config);

        jb.push(1, 100, vec![0x01]);
        jb.push(2, 200, vec![0x02]);
        jb.push(3, 300, vec![0x03]);
        jb.push(4, 400, vec![0x04]); // 应该丢弃最旧的包

        assert_eq!(jb.len(), 3);
    }

    #[test]
    fn test_raw_jitter_buffer_clear() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.push(1, 100, vec![0x01]);
        jb.push(2, 200, vec![0x02]);

        jb.clear();
        assert!(jb.is_empty());
        assert_eq!(jb.next_sequence(), 0);
    }

    #[test]
    fn test_raw_jitter_buffer_adjust_delay() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.adjust_delay(50);
        assert_eq!(jb.config().target_delay_ms, 50);

        // 测试边界值
        jb.adjust_delay(5); // 低于 min_delay_ms
        assert_eq!(jb.config().target_delay_ms, 20);

        jb.adjust_delay(300); // 高于 max_delay_ms
        assert_eq!(jb.config().target_delay_ms, 200);
    }
}
