//! UDP 传输实现
//!
//! 提供 UDP 传输、带宽自适应和丢包恢复功能。
//! 支持可选的 SRTP 加密/解密（透明集成）。

use crate::crypto::{CryptoKeys, SrtpContext, SRTP_MASTER_KEY_LEN, SRTP_MASTER_SALT_LEN};
use crate::{NetworkError, Result};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use tokio::net::UdpSocket;

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
///
/// 支持可选的 SRTP 加密/解密。默认不加密，调用 `enable_encryption` 启用。
pub struct UdpTransport {
    socket: UdpSocket,
    config: TransportConfig,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    packets_lost: AtomicU64,
    current_bitrate: AtomicU32,
    /// SRTP 加密上下文（可选，默认 None = 不加密）
    srtp: Option<Mutex<SrtpContext>>,
}

impl UdpTransport {
    /// 创建新的 UDP 传输
    pub async fn new(config: TransportConfig) -> Result<Self> {
        let socket = UdpSocket::bind(config.bind_addr)
            .await
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
            srtp: None,
        })
    }

    /// 使用默认配置创建
    pub async fn with_default_config() -> Result<Self> {
        Self::new(TransportConfig::default()).await
    }

    /// 发送数据到指定地址
    ///
    /// 如果启用了 SRTP 加密，会自动先加密再发送。
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize> {
        // 如果启用加密，先保护（加密）数据
        let send_data = if let Some(ref srtp_mutex) = self.srtp {
            let mut ctx = srtp_mutex
                .lock()
                .map_err(|e| NetworkError::CryptoError(format!("SRTP 锁获取失败: {}", e)))?;
            ctx.protect(data)?
        } else {
            data.to_vec()
        };

        let sent = self
            .socket
            .send_to(&send_data, addr)
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
        self.bytes_sent.fetch_add(sent as u64, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        Ok(sent)
    }

    /// 接收数据
    ///
    /// 如果启用了 SRTP 加密，会自动先解密再返回。
    pub async fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (received, addr) = self
            .socket
            .recv_from(buf)
            .await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;
        self.bytes_received
            .fetch_add(received as u64, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);

        // 如果启用加密，解密接收到的数据
        if let Some(ref srtp_mutex) = self.srtp {
            let mut ctx = srtp_mutex
                .lock()
                .map_err(|e| NetworkError::CryptoError(format!("SRTP 锁获取失败: {}", e)))?;
            let decrypted = ctx.unprotect(&buf[..received])?;
            let decrypted_len = decrypted.len();
            buf[..decrypted_len].copy_from_slice(&decrypted);
            return Ok((decrypted_len, addr));
        }

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
            current
                .saturating_sub(reduction * 100)
                .max(self.config.min_bitrate)
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
        self.socket
            .local_addr()
            .map_err(|e| NetworkError::BindFailed(e.to_string()))
    }

    /// 获取配置
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// 启用 SRTP 加密
    ///
    /// 使用提供的主密钥和主盐值初始化 SRTP 上下文。
    /// 启用后，`send_to` 自动加密，`receive_from` 自动解密。
    ///
    /// # 参数
    /// - `master_key`: 主加密密钥（必须为 16 字节）
    /// - `master_salt`: 主盐值（必须为 14 字节）
    ///
    /// # 错误
    /// 如果密钥或盐值长度不正确，返回 `NetworkError::CryptoError`。
    pub fn enable_encryption(&mut self, master_key: Vec<u8>, master_salt: Vec<u8>) -> Result<()> {
        if master_key.len() != SRTP_MASTER_KEY_LEN {
            return Err(NetworkError::CryptoError(format!(
                "主密钥长度错误: 期望 {} 字节, 实际 {} 字节",
                SRTP_MASTER_KEY_LEN,
                master_key.len()
            )));
        }
        if master_salt.len() != SRTP_MASTER_SALT_LEN {
            return Err(NetworkError::CryptoError(format!(
                "主盐值长度错误: 期望 {} 字节, 实际 {} 字节",
                SRTP_MASTER_SALT_LEN,
                master_salt.len()
            )));
        }

        let mut key_arr = [0u8; SRTP_MASTER_KEY_LEN];
        key_arr.copy_from_slice(&master_key);
        let mut salt_arr = [0u8; SRTP_MASTER_SALT_LEN];
        salt_arr.copy_from_slice(&master_salt);

        let keys = CryptoKeys::from_bytes(&key_arr, &salt_arr);
        let ctx = SrtpContext::new(keys, 0)
            .map_err(|e| NetworkError::CryptoError(format!("SRTP 上下文创建失败: {}", e)))?;

        self.srtp = Some(Mutex::new(ctx));
        Ok(())
    }

    /// 是否启用了加密
    pub fn is_encrypted(&self) -> bool {
        self.srtp.is_some()
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

    /// 创建本地回环传输（测试用）
    async fn new_loopback() -> Result<UdpTransport> {
        let config = TransportConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };
        UdpTransport::new(config).await
    }

    /// 构造测试用 RTP 数据包
    fn make_rtp_packet(ssrc: u32, seq: u16, payload: &[u8]) -> Vec<u8> {
        let mut pkt = Vec::with_capacity(12 + payload.len());
        pkt.push(0x80); // V=2, P=0, X=0, CC=0
        pkt.push(0x60); // M=0, PT=96
        pkt.extend_from_slice(&seq.to_be_bytes());
        pkt.extend_from_slice(&0u32.to_be_bytes()); // timestamp
        pkt.extend_from_slice(&ssrc.to_be_bytes());
        pkt.extend_from_slice(payload);
        pkt
    }

    #[tokio::test]
    async fn test_enable_encryption_and_is_encrypted() {
        let transport = new_loopback().await.unwrap();
        assert!(!transport.is_encrypted());

        // 使用可变引用需要重新绑定
        let mut transport = transport;
        let master_key = vec![0xABu8; SRTP_MASTER_KEY_LEN];
        let master_salt = vec![0xCDu8; SRTP_MASTER_SALT_LEN];
        transport
            .enable_encryption(master_key, master_salt)
            .unwrap();
        assert!(transport.is_encrypted());
    }

    #[tokio::test]
    async fn test_enable_encryption_wrong_key_length() {
        let mut transport = new_loopback().await.unwrap();

        // 密钥太短
        let result = transport.enable_encryption(vec![0x01; 8], vec![0x02; SRTP_MASTER_SALT_LEN]);
        assert!(result.is_err());

        // 密钥太长
        let result = transport.enable_encryption(vec![0x01; 32], vec![0x02; SRTP_MASTER_SALT_LEN]);
        assert!(result.is_err());

        assert!(!transport.is_encrypted());
    }

    #[tokio::test]
    async fn test_enable_encryption_wrong_salt_length() {
        let mut transport = new_loopback().await.unwrap();

        // 盐值太短
        let result = transport.enable_encryption(vec![0x01; SRTP_MASTER_KEY_LEN], vec![0x02; 8]);
        assert!(result.is_err());

        // 盐值太长
        let result = transport.enable_encryption(vec![0x01; SRTP_MASTER_KEY_LEN], vec![0x02; 32]);
        assert!(result.is_err());

        assert!(!transport.is_encrypted());
    }

    #[tokio::test]
    async fn test_encrypted_send_receive_roundtrip() {
        // 创建两个传输端，共享相同密钥
        let mut sender = new_loopback().await.unwrap();
        let mut receiver = new_loopback().await.unwrap();

        let master_key = vec![0x42u8; SRTP_MASTER_KEY_LEN];
        let master_salt = vec![0x69u8; SRTP_MASTER_SALT_LEN];

        sender
            .enable_encryption(master_key.clone(), master_salt.clone())
            .unwrap();
        receiver.enable_encryption(master_key, master_salt).unwrap();

        let receiver_addr = receiver.local_addr().unwrap();

        // 构造 RTP 数据包
        let payload = b"Hello SRTP encrypted audio!";
        let rtp_packet = make_rtp_packet(0x12345678, 1, payload);

        // 发送加密数据
        let sent = sender.send_to(&rtp_packet, receiver_addr).await.unwrap();
        // 发送的字节数应大于原始包（含认证标签）
        assert!(sent > rtp_packet.len());

        // 接收并解密
        let mut recv_buf = vec![0u8; 4096];
        let (received_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();

        // 解密后的数据应与原始 RTP 包一致
        assert_eq!(received_len, rtp_packet.len());
        assert_eq!(&recv_buf[..received_len], &rtp_packet[..]);
    }

    #[tokio::test]
    async fn test_unencrypted_send_receive() {
        // 不启用加密，验证现有行为不变
        let sender = new_loopback().await.unwrap();
        let receiver = new_loopback().await.unwrap();

        assert!(!sender.is_encrypted());
        assert!(!receiver.is_encrypted());

        let receiver_addr = receiver.local_addr().unwrap();

        let data = b"plain audio data";
        let sent = sender.send_to(data, receiver_addr).await.unwrap();
        assert_eq!(sent, data.len());

        let mut recv_buf = vec![0u8; 4096];
        let (received_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();

        assert_eq!(received_len, data.len());
        assert_eq!(&recv_buf[..received_len], data);
    }

    #[tokio::test]
    async fn test_encrypted_transport_stats() {
        let mut transport = new_loopback().await.unwrap();
        let master_key = vec![0x01u8; SRTP_MASTER_KEY_LEN];
        let master_salt = vec![0x02u8; SRTP_MASTER_SALT_LEN];
        transport
            .enable_encryption(master_key, master_salt)
            .unwrap();

        // 创建对端（不加密，仅用于接收）
        let receiver = new_loopback().await.unwrap();
        let receiver_addr = receiver.local_addr().unwrap();

        let payload = b"stats test";
        let rtp = make_rtp_packet(0xAABBCCDD, 1, payload);
        transport.send_to(&rtp, receiver_addr).await.unwrap();

        let stats = transport.stats();
        assert_eq!(stats.packets_sent, 1);
        assert!(stats.bytes_sent > 0);
    }

    #[tokio::test]
    async fn test_encrypted_multiple_packets() {
        let mut sender = new_loopback().await.unwrap();
        let mut receiver = new_loopback().await.unwrap();

        let master_key = vec![0x55u8; SRTP_MASTER_KEY_LEN];
        let master_salt = vec![0xAAu8; SRTP_MASTER_SALT_LEN];

        sender
            .enable_encryption(master_key.clone(), master_salt.clone())
            .unwrap();
        receiver.enable_encryption(master_key, master_salt).unwrap();

        let receiver_addr = receiver.local_addr().unwrap();

        // 发送多个加密包
        for seq in 0..5u16 {
            let payload = format!("frame_{}", seq);
            let rtp = make_rtp_packet(0x11111111, seq, payload.as_bytes());
            sender.send_to(&rtp, receiver_addr).await.unwrap();

            let mut recv_buf = vec![0u8; 4096];
            let (received_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
            assert_eq!(&recv_buf[..received_len], &rtp[..]);
        }

        let stats = sender.stats();
        assert_eq!(stats.packets_sent, 5);
    }

    #[tokio::test]
    async fn test_encrypted_wrong_key_rejected() {
        let mut sender = new_loopback().await.unwrap();
        let mut receiver = new_loopback().await.unwrap();

        // 使用不同密钥
        sender
            .enable_encryption(
                vec![0x01u8; SRTP_MASTER_KEY_LEN],
                vec![0x02u8; SRTP_MASTER_SALT_LEN],
            )
            .unwrap();
        receiver
            .enable_encryption(
                vec![0xFFu8; SRTP_MASTER_KEY_LEN],
                vec![0xFEu8; SRTP_MASTER_SALT_LEN],
            )
            .unwrap();

        let receiver_addr = receiver.local_addr().unwrap();

        let rtp = make_rtp_packet(0x12345678, 1, b"secret");
        sender.send_to(&rtp, receiver_addr).await.unwrap();

        let mut recv_buf = vec![0u8; 4096];
        let result = receiver.receive_from(&mut recv_buf).await;
        // 解密应失败（认证标签不匹配）
        assert!(result.is_err());
    }
}
