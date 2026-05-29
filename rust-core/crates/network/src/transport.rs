//! UDP 传输实现
//!
//! 提供 UDP 传输、带宽自适应和丢包恢复功能。

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use tokio::net::UdpSocket;
use crate::{NetworkError, Result};

/// 传输配置
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// 绑定地址
    pub bind_addr: SocketAddr,

    /// 发送缓冲区大小
    pub send_buffer_size: usize,

    /// 接收缓冲区大小
    pub recv_buffer_size: usize,

    /// 初始比特率（bps）
    pub initial_bitrate: u32,

    /// 最小比特率（bps）
    pub min_bitrate: u32,

    /// 最大比特率（bps）
    pub max_bitrate: u32,

    /// 丢包率阈值（触发降码率）
    pub loss_threshold: f32,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
            send_buffer_size: 65536,
            recv_buffer_size: 65536,
            initial_bitrate: 128000,
            min_bitrate: 32000,
            max_bitrate: 256000,
            loss_threshold: 0.05,
        }
    }
}

/// 传输统计
#[derive(Debug, Clone)]
pub struct TransportStats {
    /// 发送字节数
    pub bytes_sent: u64,

    /// 接收字节数
    pub bytes_received: u64,

    /// 发送包数
    pub packets_sent: u64,

    /// 接收包数
    pub packets_received: u64,

    /// 丢包数
    pub packets_lost: u64,

    /// 当前比特率（bps）
    pub current_bitrate: u32,

    /// 丢包率（0.0 - 1.0）
    pub loss_rate: f32,
}

/// UDP 传输
pub struct UdpTransport {
    socket: UdpSocket,
    config: TransportConfig,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    packets_lost: AtomicU64,
    current_bitrate: AtomicU32,
}

impl UdpTransport {
    /// 创建新的 UDP 传输
    pub async fn new(config: TransportConfig) -> Result<Self> {
        let socket = UdpSocket::bind(config.bind_addr).await
            .map_err(|e| NetworkError::BindFailed(e.to_string()))?;

        let initial_bitrate = config.initial_bitrate;

        Ok(Self {
            socket,
            config,
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
            packets_received: AtomicU64::new(0),
            packets_lost: AtomicU64::new(0),
            current_bitrate: AtomicU32::new(initial_bitrate),
        })
    }

    /// 使用默认配置创建
    pub async fn with_default_config() -> Result<Self> {
        Self::new(TransportConfig::default()).await
    }

    /// 发送数据到指定地址
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize> {
        let sent = self.socket.send_to(data, addr).await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
        self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        Ok(sent)
    }

    /// 接收数据
    pub async fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (received, addr) = self.socket.recv_from(buf).await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;
        self.bytes_received.fetch_add(received as u64, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        Ok((received, addr))
    }

    /// 报告丢包
    pub fn report_packet_loss(&self) {
        self.packets_lost.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取当前统计
    pub fn stats(&self) -> TransportStats {
        let packets_sent = self.packets_sent.load(Ordering::Relaxed);
        let packets_received = self.packets_received.load(Ordering::Relaxed);
        let packets_lost = self.packets_lost.load(Ordering::Relaxed);

        let loss_rate = if packets_sent > 0 {
            packets_lost as f32 / packets_sent as f32
        } else {
            0.0
        };

        TransportStats {
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent,
            packets_received,
            packets_lost,
            current_bitrate: self.current_bitrate.load(Ordering::Relaxed),
            loss_rate,
        }
    }

    /// 调整比特率（带宽自适应）
    pub fn adjust_bitrate(&self, loss_rate: f32) {
        let current = self.current_bitrate.load(Ordering::Relaxed);
        let new_bitrate = if loss_rate > self.config.loss_threshold {
            // 丢包率高，降低比特率
            let reduction = (loss_rate * 100.0) as u32;
            current.saturating_sub(reduction * 100).max(self.config.min_bitrate)
        } else if loss_rate < 0.01 {
            // 丢包率低，尝试提高比特率
            let increase = current / 10; // 增加 10%
            (current + increase).min(self.config.max_bitrate)
        } else {
            current
        };
        self.current_bitrate.store(new_bitrate, Ordering::Relaxed);
    }

    /// 获取当前比特率
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate.load(Ordering::Relaxed)
    }

    /// 获取本地地址
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.socket.local_addr()
            .map_err(|e| NetworkError::BindFailed(e.to_string()))
    }

    /// 获取配置
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.initial_bitrate, 128000);
        assert_eq!(config.min_bitrate, 32000);
        assert_eq!(config.max_bitrate, 256000);
        assert_eq!(config.loss_threshold, 0.05);
    }

    #[test]
    fn test_transport_stats() {
        // 创建一个模拟的传输统计
        let stats = TransportStats {
            bytes_sent: 1000,
            bytes_received: 800,
            packets_sent: 10,
            packets_received: 8,
            packets_lost: 2,
            current_bitrate: 128000,
            loss_rate: 0.2,
        };

        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.loss_rate, 0.2);
    }
}

