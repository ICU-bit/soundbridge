//! SoundBridge 网络传输模块
//!
//! 提供 UDP 音频传输、Jitter Buffer 和连接管理功能。

pub mod transport;
pub mod jitter_buffer;
pub mod connection;

pub use transport::{UdpTransport, TransportConfig};
pub use jitter_buffer::{JitterBuffer, JitterBufferConfig, RawJitterBuffer, RawAudioPacket};
pub use connection::{ConnectionManager, ConnectionState, ConnectionConfig, ConnectionType};

/// 网络错误类型
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("发送失败: {0}")]
    SendFailed(String),

    #[error("接收失败: {0}")]
    ReceiveFailed(String),

    #[error("绑定失败: {0}")]
    BindFailed(String),

    #[error("连接超时")]
    Timeout,

    #[error("连接已断开")]
    Disconnected,

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// 网络结果类型
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

        jb.push(3, vec![3.0f32; 100]);
        jb.push(1, vec![1.0f32; 100]);
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

        jb.push(3, 300, vec![0x05, 0x06]);
        jb.push(1, 100, vec![0x01, 0x02]);
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
