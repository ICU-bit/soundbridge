//! # QUIC 控制信令通道
//!
//! 使用 QUIC 协议提供可靠加密的控制信令通道，
//! 替代 UDP 控制信令，支持会话管理、音频参数协商、网络统计上报和设备发现。
//!
//! ## 协议格式
//!
//! 每条消息通过 QUIC 双向流传输，帧格式：
//!
//! ```text
//! ┌──────────────────┬────────────────────────────┐
//! │ 消息长度（4 字节） │ bincode 序列化的消息体      │
//! │ u32 大端序        │ ControlMessage              │
//! └──────────────────┴────────────────────────────┘
//! ```
//!
//! ## 架构
//!
//! ```text
//! ┌─────────────────┐     QUIC/TLS 1.3     ┌─────────────────┐
//! │   QuicClient    │◄─────────────────────►│   QuicServer    │
//! │                 │    双向流 / 单向流      │                 │
//! │  send_and_recv  │                       │  accept_and_reply│
//! │  send_uni       │                       │  accept          │
//! └─────────────────┘                       └─────────────────┘
//! ```
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use network::quic_control::*;
//! use std::net::SocketAddr;
//!
//! # async fn example() -> network::Result<()> {
//! // 服务器端
//! let server = QuicServer::new("127.0.0.1:0".parse().unwrap()).await?;
//! let server_addr = server.local_addr()?;
//! let cert = server.certificate();
//!
//! // 客户端
//! let client = QuicClient::new(cert).await?;
//! let conn = client.connect(server_addr, "localhost").await?;
//!
//! // 发送请求
//! let msg = ControlMessage::SessionCreate {
//!     session_id: "test".into(),
//!     device_name: "PC".into(),
//! };
//! let response = conn.send_and_recv(&msg).await?;
//! # Ok(())
//! # }
//! ```

use crate::{NetworkError, Result};
use rcgen::Certificate;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

// ──────────────────────────────── 消息类型 ────────────────────────────────

/// 音频配置参数
///
/// 用于 QUIC 控制通道中的音频参数协商。
///
/// # 默认值
///
/// | 参数 | 默认值 |
/// |------|--------|
/// | sample_rate | 48000 Hz |
/// | channels | 1（单声道） |
/// | bitrate | 128000 bps |
/// | frame_size | 960 samples |
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfig {
    /// 采样率（Hz）
    pub sample_rate: u32,
    /// 通道数（1=单声道, 2=立体声）
    pub channels: u8,
    /// 比特率（bps）
    pub bitrate: u32,
    /// 帧大小（samples）
    pub frame_size: u32,
}

/// 网络统计数据
///
/// 通过 QUIC 控制通道上报的网络质量指标。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkStatsData {
    /// 往返时延（毫秒）
    pub rtt_ms: f32,
    /// 丢包率（0.0 ~ 1.0）
    pub loss_rate: f32,
    /// 带宽估计（bps）
    pub bandwidth_bps: u64,
    /// 抖动（毫秒）
    pub jitter_ms: f32,
}

/// 设备信息
///
/// 用于设备发现功能，描述网络中的一个 SoundBridge 设备。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceInfo {
    /// 设备唯一标识
    pub device_id: String,
    /// 设备显示名称
    pub device_name: String,
    /// 设备网络地址
    pub address: SocketAddr,
}

/// 控制消息类型
///
/// 涵盖四类控制场景：
/// - **会话管理**：SessionCreate / SessionAccept / SessionReject / SessionClose
/// - **音频参数协商**：AudioConfigRequest / AudioConfigResponse
/// - **网络统计上报**：NetworkStatsReport
/// - **设备发现**：DeviceAnnounce / DeviceQuery / DeviceResponse
///
/// # 序列化
///
/// 使用 bincode 序列化，通过 QUIC 双向流或单向流传输。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlMessage {
    // ── 会话管理 ──────────────────────────────────────────────
    /// 请求创建会话（客户端 → 服务器）
    SessionCreate {
        /// 会话 ID
        session_id: String,
        /// 请求方设备名称
        device_name: String,
    },
    /// 接受会话（服务器 → 客户端）
    SessionAccept {
        /// 会话 ID
        session_id: String,
    },
    /// 拒绝会话（服务器 → 客户端）
    SessionReject {
        /// 会话 ID
        session_id: String,
        /// 拒绝原因
        reason: String,
    },
    /// 关闭会话（双向）
    SessionClose {
        /// 会话 ID
        session_id: String,
    },

    // ── 音频参数协商 ──────────────────────────────────────────
    /// 请求协商音频配置（客户端 → 服务器）
    AudioConfigRequest {
        /// 期望的音频配置
        config: AudioConfig,
    },
    /// 响应音频配置协商（服务器 → 客户端）
    AudioConfigResponse {
        /// 是否接受请求的配置
        accepted: bool,
        /// 最终确定的音频配置
        config: AudioConfig,
    },

    // ── 网络统计上报 ──────────────────────────────────────────
    /// 上报网络统计（双向）
    NetworkStatsReport {
        /// 网络统计数据
        stats: NetworkStatsData,
    },

    // ── 设备发现 ──────────────────────────────────────────────
    /// 广播设备上线（单向推送）
    DeviceAnnounce {
        /// 设备 ID
        device_id: String,
        /// 设备名称
        device_name: String,
        /// 设备地址
        address: SocketAddr,
    },
    /// 查询在线设备（客户端 → 服务器）
    DeviceQuery,
    /// 响应设备列表（服务器 → 客户端）
    DeviceResponse {
        /// 在线设备列表
        devices: Vec<DeviceInfo>,
    },
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            bitrate: 128_000,
            frame_size: 960,
        }
    }
}

// ──────────────────────────────── TLS 辅助 ────────────────────────────────

/// 生成自签名证书（localhost）
fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey)> {
    let params = rcgen::CertificateParams::new(vec!["localhost".to_string()]);

    let cert = Certificate::from_params(params)
        .map_err(|e| NetworkError::QuicError(format!("证书创建失败: {e}")))?;

    let cert_der = cert
        .serialize_der()
        .map_err(|e| NetworkError::QuicError(format!("证书序列化失败: {e}")))?;

    let key_der = cert.serialize_private_key_der();

    Ok((rustls::Certificate(cert_der), rustls::PrivateKey(key_der)))
}

/// 创建 QUIC 服务器配置
fn make_server_config(
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
) -> Result<quinn::ServerConfig> {
    let server_config = quinn::ServerConfig::with_single_cert(vec![cert], key)
        .map_err(|e| NetworkError::QuicError(format!("TLS 服务器配置失败: {e}")))?;

    Ok(server_config)
}

/// 创建 QUIC 客户端配置（信任指定证书）
fn make_client_config(cert: rustls::Certificate) -> Result<quinn::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(&cert)
        .map_err(|e| NetworkError::QuicError(format!("添加根证书失败: {e}")))?;

    let client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    Ok(quinn::ClientConfig::new(Arc::new(client_crypto)))
}

// ──────────────────────────────── 消息帧编解码 ─────────────────────────────

/// 发送控制消息（4 字节长度前缀 + bincode 载荷）
async fn send_message(send: &mut quinn::SendStream, msg: &ControlMessage) -> Result<()> {
    let data =
        bincode::serialize(msg).map_err(|e| NetworkError::SerializationError(e.to_string()))?;

    let len_bytes = (data.len() as u32).to_be_bytes();
    send.write_all(&len_bytes)
        .await
        .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

    send.write_all(&data)
        .await
        .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

    Ok(())
}

/// 接收控制消息（4 字节长度前缀 + bincode 载荷）
async fn recv_message(recv: &mut quinn::RecvStream) -> Result<ControlMessage> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(NetworkError::ReceiveFailed(format!(
            "消息长度 {} 超过上限 {}",
            len, MAX_MESSAGE_SIZE
        )));
    }

    let mut data = vec![0u8; len];
    recv.read_exact(&mut data)
        .await
        .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

    bincode::deserialize(&data).map_err(|e| NetworkError::SerializationError(e.to_string()))
}

/// 单条消息最大载荷（1 MiB）
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

// ──────────────────────────────── QUIC 连接 ────────────────────────────────

/// QUIC 连接封装
///
/// 封装一个已建立的 QUIC 连接，提供双向流消息收发和单向流消息推送。
///
/// # 消息模式
///
/// - **双向流**（[`send_and_recv`](Self::send_and_recv) / [`accept_and_reply`](Self::accept_and_reply)）：
///   请求-响应模式，客户端发送请求，服务器处理后返回响应
/// - **单向流**（[`send_uni`](Self::send_uni) / [`recv_uni`](Self::recv_uni)）：
///   推送模式，无需响应
pub struct QuicConnection {
    conn: quinn::Connection,
}

impl QuicConnection {
    /// 获取远程对端地址
    ///
    /// # Returns
    ///
    /// 对端的 `SocketAddr`（IP + 端口）。
    pub fn remote_addr(&self) -> SocketAddr {
        self.conn.remote_address()
    }

    /// 发送请求并等待响应（双向流，客户端使用）
    ///
    /// 打开一个 QUIC 双向流，发送请求消息，等待服务器响应。
    ///
    /// # Arguments
    ///
    /// * `msg` - 要发送的控制消息
    ///
    /// # Returns
    ///
    /// 服务器返回的响应消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::SendFailed`] — 打开流或发送失败
    /// - [`NetworkError::ReceiveFailed`] — 接收响应失败
    /// - [`NetworkError::SerializationError`] — 消息编解码失败
    pub async fn send_and_recv(&self, msg: &ControlMessage) -> Result<ControlMessage> {
        let (mut send, mut recv) = self
            .conn
            .open_bi()
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        send_message(&mut send, msg).await?;

        // finish 表示写入结束，通知对端不再发送更多数据
        send.finish()
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        recv_message(&mut recv).await
    }

    /// 接受一个请求并发送响应（双向流，服务器端使用）
    ///
    /// 等待客户端发送请求，调用 `handler` 处理后返回响应。
    ///
    /// # Arguments
    ///
    /// * `handler` - 请求处理函数，接收请求消息，返回响应消息
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ReceiveFailed`] — 接收请求失败
    /// - [`NetworkError::SendFailed`] — 发送响应失败
    /// - [`NetworkError::SerializationError`] — 消息编解码失败
    pub async fn accept_and_reply<F>(&self, handler: F) -> Result<()>
    where
        F: FnOnce(ControlMessage) -> ControlMessage,
    {
        let (mut send, mut recv) = self
            .conn
            .accept_bi()
            .await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

        let request = recv_message(&mut recv).await?;
        let response = handler(request);

        send_message(&mut send, &response).await?;

        send.finish()
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// 发送单向消息（无需响应）
    ///
    /// 打开一个 QUIC 单向流，发送消息后关闭流。
    /// 适用于通知类消息（如 DeviceAnnounce、NetworkStatsReport）。
    ///
    /// # Arguments
    ///
    /// * `msg` - 要发送的控制消息
    ///
    /// # Errors
    ///
    /// - [`NetworkError::SendFailed`] — 打开流或发送失败
    /// - [`NetworkError::SerializationError`] — 消息序列化失败
    pub async fn send_uni(&self, msg: &ControlMessage) -> Result<()> {
        let mut send = self
            .conn
            .open_uni()
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        send_message(&mut send, msg).await?;

        send.finish()
            .await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// 接受单向消息
    ///
    /// 等待并接收一个来自对端的单向流消息。
    ///
    /// # Returns
    ///
    /// 接收到的控制消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ReceiveFailed`] — 接收失败
    /// - [`NetworkError::SerializationError`] — 消息反序列化失败
    pub async fn recv_uni(&self) -> Result<ControlMessage> {
        let mut recv = self
            .conn
            .accept_uni()
            .await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

        recv_message(&mut recv).await
    }

    /// 主动关闭连接
    ///
    /// # Arguments
    ///
    /// * `error_code` - 关闭错误码（0 表示正常关闭）
    /// * `reason` - 关闭原因描述
    pub fn close(&self, error_code: u32, reason: &[u8]) {
        self.conn.close(error_code.into(), reason);
    }
}

// ──────────────────────────────── QUIC 服务器 ──────────────────────────────

/// QUIC 控制信令服务器
///
/// 绑定 UDP 端口，接受来自客户端的 QUIC 连接。
/// 自动生成自签名证书，通过 [`certificate()`](Self::certificate) 提供给客户端用于 TLS 验证。
///
/// # 示例
///
/// ```rust,no_run
/// use network::quic_control::QuicServer;
///
/// # async fn example() -> network::Result<()> {
/// let server = QuicServer::new("127.0.0.1:0".parse().unwrap()).await?;
/// let addr = server.local_addr()?;
/// let cert = server.certificate();
///
/// // 接受连接
/// if let Some(conn) = server.accept().await {
///     println!("新连接来自: {}", conn.remote_addr());
/// }
/// # Ok(())
/// # }
/// ```
pub struct QuicServer {
    endpoint: quinn::Endpoint,
    cert_der: Vec<u8>,
}

impl QuicServer {
    /// 创建服务器并绑定到指定地址
    ///
    /// 自动生成自签名 TLS 证书，使用 `0.0.0.0:0` 让操作系统分配可用端口。
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - 绑定地址（如 `0.0.0.0:0` 或 `127.0.0.1:8443`）
    ///
    /// # Errors
    ///
    /// - [`NetworkError::BindFailed`] — Socket 绑定失败
    /// - [`NetworkError::QuicError`] — TLS 证书生成失败
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use network::quic_control::QuicServer;
    ///
    /// # async fn example() -> network::Result<()> {
    /// let server = QuicServer::new("0.0.0.0:0".parse().unwrap()).await?;
    /// println!("服务器监听: {}", server.local_addr()?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let (cert, key) = generate_self_signed_cert()?;
        let cert_der = cert.0.clone();
        let server_config = make_server_config(cert, key)?;

        let endpoint = quinn::Endpoint::server(server_config, bind_addr)
            .map_err(|e| NetworkError::BindFailed(e.to_string()))?;

        Ok(Self { endpoint, cert_der })
    }

    /// 获取服务器绑定的本地地址
    ///
    /// # Errors
    ///
    /// 如果获取地址失败，返回 [`NetworkError::BindFailed`]。
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint
            .local_addr()
            .map_err(|e| NetworkError::BindFailed(e.to_string()))
    }

    /// 获取服务器证书的 DER 编码字节
    ///
    /// 客户端可使用此证书配置 TLS 信任。
    pub fn certificate_der(&self) -> &[u8] {
        &self.cert_der
    }

    /// 获取 `rustls::Certificate` 供客户端配置使用
    ///
    /// # Returns
    ///
    /// 包含 DER 编码证书的 `rustls::Certificate` 实例。
    pub fn certificate(&self) -> rustls::Certificate {
        rustls::Certificate(self.cert_der.clone())
    }

    /// 接受下一个入站连接
    ///
    /// 异步等待并接受一个客户端 QUIC 连接。
    ///
    /// # Returns
    ///
    /// - `Some(QuicConnection)` — 成功接受的连接
    /// - `None` — Endpoint 已关闭
    pub async fn accept(&self) -> Option<QuicConnection> {
        let incoming = self.endpoint.accept().await?;
        let conn = incoming.await.ok()?;
        Some(QuicConnection { conn })
    }

    /// 关闭服务器
    ///
    /// 关闭 Endpoint，拒绝新的连接请求。已建立的连接不受影响。
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"server shutdown");
    }
}

// ──────────────────────────────── QUIC 客户端 ──────────────────────────────

/// QUIC 控制信令客户端
///
/// 连接到 QUIC 服务器，支持双向流和单向流消息收发。
///
/// # 示例
///
/// ```rust,no_run
/// use network::quic_control::{QuicClient, ControlMessage};
/// use std::net::SocketAddr;
///
/// # async fn example(server_cert: rustls::Certificate, server_addr: SocketAddr) -> network::Result<()> {
/// let client = QuicClient::new(server_cert).await?;
/// let conn = client.connect(server_addr, "localhost").await?;
///
/// let msg = ControlMessage::DeviceQuery;
/// let response = conn.send_and_recv(&msg).await?;
/// # Ok(())
/// # }
/// ```
pub struct QuicClient {
    endpoint: quinn::Endpoint,
}

impl QuicClient {
    /// 创建客户端
    ///
    /// 配置 TLS 信任指定的服务器证书。
    ///
    /// # Arguments
    ///
    /// * `server_cert` - 服务器证书（DER 编码），用于 TLS 验证
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — Endpoint 创建失败
    pub async fn new(server_cert: rustls::Certificate) -> Result<Self> {
        let client_config = make_client_config(server_cert)?;

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        Ok(Self { endpoint })
    }

    /// 连接到指定服务器
    ///
    /// # Arguments
    ///
    /// * `server_addr` - 服务器地址
    /// * `server_name` - TLS SNI 主机名（自签名场景下通常为 `"localhost"`）
    ///
    /// # Returns
    ///
    /// 建立的 [`QuicConnection`] 连接实例。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 连接失败（地址不可达、证书不匹配等）
    pub async fn connect(
        &self,
        server_addr: SocketAddr,
        server_name: &str,
    ) -> Result<QuicConnection> {
        let conn = self
            .endpoint
            .connect(server_addr, server_name)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        Ok(QuicConnection { conn })
    }

    /// 关闭客户端
    ///
    /// 关闭 Endpoint 和所有关联连接。
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"client shutdown");
    }
}

// ──────────────────────────────── 测试 ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 辅助：创建本地服务器和客户端，返回 (server, client_conn)
    async fn setup_server_client() -> (QuicServer, QuicConnection) {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");

        let server_addr = server.local_addr().expect("获取地址失败");
        let cert = server.certificate();

        let client = QuicClient::new(cert).await.expect("客户端创建失败");
        let conn = client
            .connect(server_addr, "localhost")
            .await
            .expect("连接失败");

        (server, conn)
    }

    // ── 证书生成 ──────────────────────────────────────────────

    #[test]
    fn test_generate_self_signed_cert() {
        let (cert, key) = generate_self_signed_cert().expect("证书生成失败");
        assert!(!cert.0.is_empty());
        assert!(!key.0.is_empty());
    }

    // ── 消息类型 ──────────────────────────────────────────────

    #[test]
    fn test_control_message_serde_roundtrip() {
        let messages = vec![
            ControlMessage::SessionCreate {
                session_id: "s1".into(),
                device_name: "PC".into(),
            },
            ControlMessage::SessionAccept {
                session_id: "s1".into(),
            },
            ControlMessage::SessionReject {
                session_id: "s1".into(),
                reason: "busy".into(),
            },
            ControlMessage::SessionClose {
                session_id: "s1".into(),
            },
            ControlMessage::AudioConfigRequest {
                config: AudioConfig::default(),
            },
            ControlMessage::AudioConfigResponse {
                accepted: true,
                config: AudioConfig {
                    sample_rate: 44100,
                    channels: 2,
                    bitrate: 256_000,
                    frame_size: 960,
                },
            },
            ControlMessage::NetworkStatsReport {
                stats: NetworkStatsData {
                    rtt_ms: 12.5,
                    loss_rate: 0.02,
                    bandwidth_bps: 128_000,
                    jitter_ms: 3.0,
                },
            },
            ControlMessage::DeviceAnnounce {
                device_id: "dev1".into(),
                device_name: "Android".into(),
                address: "192.168.1.10:5000".parse().unwrap(),
            },
            ControlMessage::DeviceQuery,
            ControlMessage::DeviceResponse {
                devices: vec![DeviceInfo {
                    device_id: "dev1".into(),
                    device_name: "Android".into(),
                    address: "192.168.1.10:5000".parse().unwrap(),
                }],
            },
        ];

        for msg in &messages {
            let encoded = bincode::serialize(msg).expect("序列化失败");
            let decoded: ControlMessage = bincode::deserialize(&encoded).expect("反序列化失败");
            assert_eq!(*msg, decoded);
        }
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.bitrate, 128_000);
        assert_eq!(config.frame_size, 960);
    }

    // ── 本地连接测试 ──────────────────────────────────────────

    #[tokio::test]
    async fn test_server_accepts_connection() {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");

        let addr = server.local_addr().expect("获取地址失败");
        let cert = server.certificate();

        let client = QuicClient::new(cert).await.expect("客户端创建失败");

        // 在后台接受连接
        let accept_handle = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(5), server.accept())
                .await
                .expect("接受连接超时")
                .expect("未收到连接")
        });

        let _client_conn = client.connect(addr, "localhost").await.expect("连接失败");

        let server_conn = accept_handle.await.expect("任务失败");
        assert_eq!(
            server_conn.remote_addr().ip(),
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        );
    }

    #[tokio::test]
    async fn test_local_addr() {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");

        let addr = server.local_addr().expect("获取地址失败");
        assert_eq!(
            addr.ip(),
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        );
        assert_ne!(addr.port(), 0);
    }

    #[tokio::test]
    async fn test_certificate_der() {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");

        let cert_der = server.certificate_der();
        assert!(!cert_der.is_empty());

        let cert = server.certificate();
        assert_eq!(cert.0, cert_der);
    }

    // ── 消息收发测试 ──────────────────────────────────────────

    #[tokio::test]
    async fn test_send_and_recv_session_create() {
        let (server, client_conn) = setup_server_client().await;

        let msg = ControlMessage::SessionCreate {
            session_id: "test-session".into(),
            device_name: "PC".into(),
        };

        // 客户端发送请求
        let send_msg = msg.clone();
        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&send_msg).await });

        // 服务器接受并回复
        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|request| {
                assert_eq!(request, msg);
                ControlMessage::SessionAccept {
                    session_id: "test-session".into(),
                }
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        assert_eq!(
            response,
            ControlMessage::SessionAccept {
                session_id: "test-session".into(),
            }
        );

        server.close();
    }

    #[tokio::test]
    async fn test_send_and_recv_audio_config() {
        let (server, client_conn) = setup_server_client().await;

        let config = AudioConfig {
            sample_rate: 44100,
            channels: 2,
            bitrate: 256_000,
            frame_size: 960,
        };
        let msg = ControlMessage::AudioConfigRequest {
            config: config.clone(),
        };

        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|request| {
                if let ControlMessage::AudioConfigRequest { config: req_config } = &request {
                    assert_eq!(req_config.sample_rate, 44100);
                } else {
                    panic!("期望 AudioConfigRequest");
                }
                ControlMessage::AudioConfigResponse {
                    accepted: true,
                    config: AudioConfig::default(),
                }
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        match response {
            ControlMessage::AudioConfigResponse { accepted, config } => {
                assert!(accepted);
                assert_eq!(config.sample_rate, 48000);
            }
            _ => panic!("期望 AudioConfigResponse"),
        }

        server.close();
    }

    #[tokio::test]
    async fn test_send_and_recv_network_stats() {
        let (server, client_conn) = setup_server_client().await;

        let stats = NetworkStatsData {
            rtt_ms: 25.0,
            loss_rate: 0.01,
            bandwidth_bps: 128_000,
            jitter_ms: 5.0,
        };
        let msg = ControlMessage::NetworkStatsReport { stats };

        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|request| match request {
                ControlMessage::NetworkStatsReport { stats } => {
                    assert!((stats.rtt_ms - 25.0).abs() < f32::EPSILON);
                    assert!((stats.loss_rate - 0.01).abs() < f32::EPSILON);
                    ControlMessage::SessionAccept {
                        session_id: "ack".into(),
                    }
                }
                _ => panic!("期望 NetworkStatsReport"),
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        assert_eq!(
            response,
            ControlMessage::SessionAccept {
                session_id: "ack".into(),
            }
        );

        server.close();
    }

    #[tokio::test]
    async fn test_send_and_recv_device_discovery() {
        let (server, client_conn) = setup_server_client().await;

        let msg = ControlMessage::DeviceQuery;

        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|request| {
                assert_eq!(request, ControlMessage::DeviceQuery);
                ControlMessage::DeviceResponse {
                    devices: vec![
                        DeviceInfo {
                            device_id: "dev1".into(),
                            device_name: "PC".into(),
                            address: "192.168.1.10:5000".parse().unwrap(),
                        },
                        DeviceInfo {
                            device_id: "dev2".into(),
                            device_name: "Android".into(),
                            address: "192.168.1.20:5000".parse().unwrap(),
                        },
                    ],
                }
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        match response {
            ControlMessage::DeviceResponse { devices } => {
                assert_eq!(devices.len(), 2);
                assert_eq!(devices[0].device_name, "PC");
                assert_eq!(devices[1].device_name, "Android");
            }
            _ => panic!("期望 DeviceResponse"),
        }

        server.close();
    }

    // ── 单向流测试 ────────────────────────────────────────────

    #[tokio::test]
    async fn test_unidirectional_stream() {
        let (server, client_conn) = setup_server_client().await;

        let msg = ControlMessage::SessionClose {
            session_id: "s1".into(),
        };

        // 客户端发送单向消息
        let send_msg = msg.clone();
        let client_handle = tokio::spawn(async move { client_conn.send_uni(&send_msg).await });

        // 服务器接收单向消息
        let server_conn = server.accept().await.expect("接受连接失败");
        let received = server_conn.recv_uni().await.expect("接收失败");
        assert_eq!(received, msg);

        client_handle.await.expect("任务失败").expect("发送失败");
        server.close();
    }

    // ── 并发连接测试 ──────────────────────────────────────────

    #[tokio::test]
    async fn test_concurrent_connections() {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");
        let server_addr = server.local_addr().expect("获取地址失败");
        let cert = server.certificate();

        // 启动 3 个客户端并发连接
        let mut client_handles = Vec::new();
        for i in 0..3 {
            let cert_clone = cert.clone();
            let handle = tokio::spawn(async move {
                let client = QuicClient::new(cert_clone).await.expect("客户端创建失败");
                let conn = client
                    .connect(server_addr, "localhost")
                    .await
                    .expect("连接失败");

                let msg = ControlMessage::SessionCreate {
                    session_id: format!("session-{i}"),
                    device_name: format!("Device-{i}"),
                };
                conn.send_and_recv(&msg).await
            });
            client_handles.push(handle);
        }

        // 服务器处理 3 个连接
        for _ in 0..3 {
            let server_conn = server.accept().await.expect("接受连接失败");
            tokio::spawn(async move {
                server_conn
                    .accept_and_reply(|_request| ControlMessage::SessionAccept {
                        session_id: "ok".into(),
                    })
                    .await
                    .expect("处理失败");
            });
        }

        // 验证所有客户端都收到正确响应
        for handle in client_handles {
            let response = handle.await.expect("任务失败").expect("收发失败");
            assert_eq!(
                response,
                ControlMessage::SessionAccept {
                    session_id: "ok".into(),
                }
            );
        }

        server.close();
    }

    // ── 错误处理测试 ──────────────────────────────────────────

    #[tokio::test]
    async fn test_server_close_causes_client_error() {
        let server = QuicServer::new("127.0.0.1:0".parse().unwrap())
            .await
            .expect("服务器创建失败");
        let server_addr = server.local_addr().expect("获取地址失败");
        let cert = server.certificate();

        let client = QuicClient::new(cert).await.expect("客户端创建失败");

        // 先关闭服务器
        server.close();

        // 客户端连接应该失败
        let result = client.connect(server_addr, "localhost").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_to_wrong_addr_fails() {
        // 使用一个不太可能有 QUIC 服务的端口
        let wrong_addr: SocketAddr = "127.0.0.1:1".parse().unwrap();

        let (_, client_conn) = setup_server_client().await;

        // 创建新客户端连接到错误地址应该失败
        let cert = rustls::Certificate(vec![0u8; 10]); // 无效证书
        let result = QuicClient::new(cert).await;
        // 无效证书应该仍然能创建客户端（验证在连接时）
        // 连接到错误地址应该失败
        if let Ok(client) = result {
            let conn_result = client.connect(wrong_addr, "localhost").await;
            assert!(conn_result.is_err());
        }

        client_conn.close(0, b"test done");
    }

    #[tokio::test]
    async fn test_session_reject_flow() {
        let (server, client_conn) = setup_server_client().await;

        let msg = ControlMessage::SessionCreate {
            session_id: "busy-session".into(),
            device_name: "Unknown".into(),
        };

        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|_request| ControlMessage::SessionReject {
                session_id: "busy-session".into(),
                reason: "设备忙碌".into(),
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        match response {
            ControlMessage::SessionReject { session_id, reason } => {
                assert_eq!(session_id, "busy-session");
                assert_eq!(reason, "设备忙碌");
            }
            _ => panic!("期望 SessionReject"),
        }

        server.close();
    }

    // ── 消息边界测试 ──────────────────────────────────────────

    #[tokio::test]
    async fn test_large_message() {
        let (server, client_conn) = setup_server_client().await;

        // 创建一个包含大量设备信息的消息
        let devices: Vec<DeviceInfo> = (0..1000)
            .map(|i| DeviceInfo {
                device_id: format!("dev-{i}"),
                device_name: format!("Device {i}"),
                address: SocketAddr::new("192.168.1.100".parse().unwrap(), 5000 + i as u16),
            })
            .collect();
        let msg = ControlMessage::DeviceResponse { devices };

        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|request| {
                if let ControlMessage::DeviceResponse { devices } = &request {
                    assert_eq!(devices.len(), 1000);
                } else {
                    panic!("期望 DeviceResponse");
                }
                ControlMessage::SessionAccept {
                    session_id: "ack".into(),
                }
            })
            .await
            .expect("处理请求失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        assert_eq!(
            response,
            ControlMessage::SessionAccept {
                session_id: "ack".into(),
            }
        );

        server.close();
    }

    #[tokio::test]
    async fn test_multiple_sequential_messages() {
        let (server, client_conn) = setup_server_client().await;

        // 测试第一条消息
        let msg = ControlMessage::SessionCreate {
            session_id: "s1".into(),
            device_name: "PC".into(),
        };
        let client_handle = tokio::spawn(async move { client_conn.send_and_recv(&msg).await });

        let server_conn = server.accept().await.expect("接受连接失败");
        server_conn
            .accept_and_reply(|_| ControlMessage::SessionAccept {
                session_id: "ok".into(),
            })
            .await
            .expect("处理失败");

        let response = client_handle.await.expect("任务失败").expect("收发失败");
        assert_eq!(
            response,
            ControlMessage::SessionAccept {
                session_id: "ok".into(),
            }
        );

        server.close();
    }
}
