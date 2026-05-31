//! # 会话握手协议模块
//!
//! 实现标准化的会话建立流程，支持能力协商、身份认证和会话生命周期管理。
//!
//! ## 握手流程
//!
//! ```text
//! Client                     Server
//!   |                           |
//!   |------- ClientHello ------>|  携带客户端能力声明 + ECDH 公钥
//!   |                           |
//!   |<------ ServerHello -------|  携带服务器能力 + 协商参数 + ECDH 公钥
//!   |                           |
//!   |------- KeyExchange ------>|  确认协商参数 + 最终公钥
//!   |                           |
//!   |<------ Finished ----------|  握手摘要确认
//!   |                           |
//!   |======= Established =======|  会话建立，开始心跳
//! ```
//!
//! ## 状态机
//!
//! ```text
//! Idle → ClientHelloSent → ServerHelloSent → KeyExchangeSent → Established → Closed
//! ```
//!
//! ## 能力协商
//!
//! 客户端和服务器在握手过程中交换 [`Capability`] 声明，服务器负责选择双方都支持的
//! 最优参数组合（传输协议、加密模式、Opus 配置）。
//!
//! ## 心跳机制
//!
//! 会话建立后，双方通过 [`HandshakeMessage::Heartbeat`] / [`HandshakeMessage::HeartbeatAck`]
//! 维持连接活跃度，并测量 RTT。
//!
//! ## 优雅断开
//!
//! 通过 [`HandshakeMessage::Disconnect`] 消息实现优雅断开，
//! 携带 [`DisconnectReason`] 说明断开原因。

use crate::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};

/// 握手默认超时时间（毫秒）
pub const HANDSHAKE_TIMEOUT_MS: u64 = 10_000;

/// 心跳默认间隔（毫秒）
pub const HEARTBEAT_INTERVAL_MS: u64 = 5_000;

/// 心跳默认超时时间（毫秒）
pub const HEARTBEAT_TIMEOUT_MS: u64 = 15_000;

/// 握手默认最大重试次数
pub const MAX_HANDSHAKE_RETRIES: u32 = 3;

/// 会话协议版本号
pub const SESSION_PROTOCOL_VERSION: u16 = 1;

// ──────────────────────────────── 状态机 ────────────────────────────────

/// 会话状态
///
/// 表示会话握手流程的当前阶段，驱动整个会话生命周期。
///
/// # 状态转换
///
/// ```text
/// Idle → ClientHelloSent → ServerHelloSent → KeyExchangeSent → Established → Closed
/// ```
///
/// # 示例
///
/// ```rust
/// use network::session::SessionState;
///
/// assert!(!SessionState::Idle.is_terminal());
/// assert!(SessionState::Established.is_terminal());
/// assert!(SessionState::Established.can_send_data());
/// assert!(!SessionState::Idle.can_send_data());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    /// 空闲，等待发起握手
    Idle,
    /// 客户端已发送 ClientHello，等待 ServerHello
    ClientHelloSent,
    /// 服务器已发送 ServerHello，等待 KeyExchange
    ServerHelloSent,
    /// 密钥交换已发送，等待 Finished
    KeyExchangeSent,
    /// 握手完成，连接已建立，可以收发数据
    Established,
    /// 会话已关闭（主动断开或超时）
    Closed,
}

impl SessionState {
    /// 判断是否为终态（Established 或 Closed）
    ///
    /// # Returns
    ///
    /// `true` 表示会话已到达最终状态，不再接受新的握手消息。
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Established | Self::Closed)
    }

    /// 判断是否可以发送音频数据
    ///
    /// # Returns
    ///
    /// `true` 仅当状态为 `Established`。
    pub fn can_send_data(&self) -> bool {
        *self == Self::Established
    }
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::ClientHelloSent => write!(f, "ClientHelloSent"),
            Self::ServerHelloSent => write!(f, "ServerHelloSent"),
            Self::KeyExchangeSent => write!(f, "KeyExchangeSent"),
            Self::Established => write!(f, "Established"),
            Self::Closed => write!(f, "Closed"),
        }
    }
}

// ──────────────────────────────── 能力协商 ────────────────────────────────

/// 传输协议类型
///
/// 用于能力协商阶段，双方声明支持的传输协议。
///
/// # 示例
///
/// ```rust
/// use network::session::TransportProtocol;
///
/// assert_eq!(format!("{}", TransportProtocol::Udp), "UDP");
/// assert_eq!(format!("{}", TransportProtocol::Quic), "QUIC");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TransportProtocol {
    /// UDP 传输（低延迟，适合实时音频流）
    #[default]
    Udp,
    /// QUIC 传输（可靠加密，适合控制信令）
    Quic,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Udp => write!(f, "UDP"),
            Self::Quic => write!(f, "QUIC"),
        }
    }
}

/// 加密模式
///
/// 用于能力协商阶段，双方声明支持的加密方式。
///
/// # 示例
///
/// ```rust
/// use network::session::EncryptionMode;
///
/// assert_eq!(format!("{}", EncryptionMode::Srtp), "SRTP");
/// assert_eq!(format!("{}", EncryptionMode::None), "None");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EncryptionMode {
    /// 无加密（明文传输，仅用于测试或可信网络）
    None,
    /// SRTP 加密（AES-128-CM + HMAC-SHA1-80，默认推荐）
    #[default]
    Srtp,
}

impl std::fmt::Display for EncryptionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Srtp => write!(f, "SRTP"),
        }
    }
}

/// Opus 编解码器配置
///
/// 描述音频编解码参数，在握手阶段由双方协商确定。
///
/// # 默认值
///
/// | 参数 | 默认值 | 说明 |
/// |------|--------|------|
/// | sample_rate | 48000 Hz | 采样率 |
/// | channels | 1 | 单声道 |
/// | bitrate | 128000 bps | 比特率 |
/// | frame_size | 960 samples | 帧大小（20ms@48kHz） |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpusConfig {
    /// 采样率（Hz）：8000, 12000, 16000, 24000, 48000
    pub sample_rate: u32,
    /// 通道数（1=单声道, 2=立体声）
    pub channels: u8,
    /// 比特率（bps）
    pub bitrate: u32,
    /// 帧大小（samples），通常为 960（20ms@48kHz）
    pub frame_size: u32,
}

impl Default for OpusConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            bitrate: 128_000,
            frame_size: 960,
        }
    }
}

/// 会话能力声明
///
/// 在握手阶段由客户端和服务器双方交换，声明各自支持的传输协议、
/// 加密模式和音频配置。服务器根据双方能力选择最优的协商参数。
///
/// # 示例
///
/// ```rust
/// use network::session::{Capability, TransportProtocol, EncryptionMode};
///
/// let cap = Capability::default();
/// assert!(cap.transport_protocols.contains(&TransportProtocol::Udp));
/// assert!(cap.encryption_modes.contains(&EncryptionMode::Srtp));
/// assert_eq!(cap.opus_config.sample_rate, 48000);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    /// 协议版本号
    pub version: u16,
    /// 支持的传输协议列表（按优先级排序）
    pub transport_protocols: Vec<TransportProtocol>,
    /// 支持的加密模式列表（按优先级排序）
    pub encryption_modes: Vec<EncryptionMode>,
    /// Opus 编解码器配置
    pub opus_config: OpusConfig,
    /// 设备唯一标识（UUID 格式）
    pub device_id: String,
    /// 设备显示名称
    pub device_name: String,
}

impl Default for Capability {
    fn default() -> Self {
        Self {
            version: SESSION_PROTOCOL_VERSION,
            transport_protocols: vec![TransportProtocol::Udp, TransportProtocol::Quic],
            encryption_modes: vec![EncryptionMode::Srtp, EncryptionMode::None],
            opus_config: OpusConfig::default(),
            device_id: String::new(),
            device_name: String::new(),
        }
    }
}

/// 协商结果
///
/// 服务器根据双方能力声明选择的最终参数，客户端确认后生效。
///
/// # 示例
///
/// ```rust
/// use network::session::{NegotiatedParams, TransportProtocol, EncryptionMode, OpusConfig};
///
/// let params = NegotiatedParams {
///     transport_protocol: TransportProtocol::Udp,
///     encryption_mode: EncryptionMode::Srtp,
///     opus_config: OpusConfig::default(),
/// };
/// assert_eq!(params.transport_protocol, TransportProtocol::Udp);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiatedParams {
    /// 选定的传输协议
    pub transport_protocol: TransportProtocol,
    /// 选定的加密模式
    pub encryption_mode: EncryptionMode,
    /// 选定的 Opus 配置
    pub opus_config: OpusConfig,
}

// ──────────────────────────────── 握手消息 ────────────────────────────────

/// ECDH 公钥
///
/// 用于密钥交换阶段。封装 X25519 公钥的 32 字节表示。
///
/// # 示例
///
/// ```rust
/// use network::session::EcdhPublicKey;
///
/// let (secret1, key1) = EcdhPublicKey::generate_keypair();
/// let (secret2, key2) = EcdhPublicKey::generate_keypair();
/// assert_ne!(key1, key2); // 每次生成的公钥都不同
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcdhPublicKey(pub [u8; 32]);

impl EcdhPublicKey {
    /// 生成 X25519 密钥对（临时私钥 + 对应公钥）
    ///
    /// # Returns
    ///
    /// `(EphemeralSecret, EcdhPublicKey)` 元组，私钥用于后续 DH 计算。
    pub fn generate_keypair() -> (EphemeralSecret, Self) {
        let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let public = X25519PublicKey::from(&secret);
        (secret, Self(public.to_bytes()))
    }

    /// 仅生成随机公钥（用于测试消息构造，不持有私钥）
    pub fn generate() -> Self {
        let (_secret, public) = Self::generate_keypair();
        public
    }

    /// 将公钥字节转换为 x25519_dalek::PublicKey
    fn to_x25519_public(&self) -> X25519PublicKey {
        X25519PublicKey::from(self.0)
    }
}

/// 握手消息
///
/// 定义会话握手过程中的所有消息类型，涵盖：
/// - 握手建立（ClientHello、ServerHello、KeyExchange、Finished）
/// - 心跳维持（Heartbeat、HeartbeatAck）
/// - 优雅断开（Disconnect）
///
/// 所有消息通过 [`Session::serialize_message`] / [`Session::deserialize_message`]
/// 进行 bincode 序列化/反序列化。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HandshakeMessage {
    /// 客户端发起握手，声明能力和 ECDH 公钥
    ClientHello {
        /// 会话 ID（UUID 格式）
        session_id: String,
        /// 客户端能力声明
        capabilities: Capability,
        /// 客户端 ECDH 公钥
        client_public_key: EcdhPublicKey,
    },
    /// 服务器响应，选择协商参数并返回 ECDH 公钥
    ServerHello {
        /// 会话 ID
        session_id: String,
        /// 服务器能力声明
        capabilities: Capability,
        /// 服务器选择的协商参数
        negotiated: NegotiatedParams,
        /// 服务器 ECDH 公钥
        server_public_key: EcdhPublicKey,
    },
    /// 客户端确认协商参数（密钥交换完成）
    KeyExchange {
        /// 会话 ID
        session_id: String,
        /// 客户端确认的协商参数
        negotiated: NegotiatedParams,
        /// 客户端最终公钥（可能更新）
        client_public_key: EcdhPublicKey,
    },
    /// 服务器发送握手完成确认（含握手摘要）
    Finished {
        /// 会话 ID
        session_id: String,
        /// 握手摘要（HMAC）
        handshake_hash: Vec<u8>,
    },
    /// 心跳消息（维持连接活跃度）
    Heartbeat {
        /// 会话 ID
        session_id: String,
        /// 心跳序列号（递增）
        sequence: u32,
        /// 发送时间戳（微秒）
        timestamp_us: u64,
    },
    /// 心跳响应
    HeartbeatAck {
        /// 会话 ID
        session_id: String,
        /// 对应的心跳序列号
        sequence: u32,
        /// 原始心跳时间戳（用于 RTT 计算）
        timestamp_us: u64,
    },
    /// 优雅断开请求
    Disconnect {
        /// 会话 ID
        session_id: String,
        /// 断开原因
        reason: DisconnectReason,
    },
}

/// 断开原因
///
/// 在 [`HandshakeMessage::Disconnect`] 消息中携带，说明会话断开的具体原因。
///
/// # 示例
///
/// ```rust
/// use network::session::DisconnectReason;
///
/// assert_eq!(format!("{}", DisconnectReason::UserInitiated), "User initiated");
/// assert_eq!(format!("{}", DisconnectReason::HeartbeatTimeout), "Heartbeat timeout");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// 用户主动断开
    UserInitiated,
    /// 心跳超时（对端无响应）
    HeartbeatTimeout,
    /// 握手失败（超时或重试耗尽）
    HandshakeFailed,
    /// 协议错误（消息格式或状态不匹配）
    ProtocolError,
    /// 关机（应用或系统关闭）
    Shutdown,
}

impl std::fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserInitiated => write!(f, "User initiated"),
            Self::HeartbeatTimeout => write!(f, "Heartbeat timeout"),
            Self::HandshakeFailed => write!(f, "Handshake failed"),
            Self::ProtocolError => write!(f, "Protocol error"),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

// ──────────────────────────────── 会话管理 ────────────────────────────────

/// 心跳统计
///
/// 跟踪心跳消息的发送、接收和丢失情况，用于网络质量评估。
#[derive(Debug, Clone, Default)]
pub struct HeartbeatStats {
    /// 已发送的心跳数
    pub sent: u32,
    /// 已收到响应的心跳数
    pub acked: u32,
    /// 丢失的心跳数（未收到响应）
    pub lost: u32,
    /// 最近一次 RTT（微秒）
    pub last_rtt_us: Option<u64>,
}

impl HeartbeatStats {
    /// 计算心跳丢失率
    ///
    /// # Returns
    ///
    /// 丢失率（0.0 ~ 1.0），未发送过心跳时返回 0.0。
    pub fn loss_rate(&self) -> f32 {
        if self.sent == 0 {
            return 0.0;
        }
        self.lost as f32 / self.sent as f32
    }
}

/// 会话统计信息
///
/// 收集会话生命周期内的关键指标。
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    /// 心跳统计
    pub heartbeat: HeartbeatStats,
    /// 会话建立时间
    pub started_at: Option<Instant>,
    /// 握手耗时（毫秒）
    pub handshake_duration_ms: Option<u64>,
}

/// 会话配置
///
/// 控制心跳间隔、超时时间和握手重试行为。
///
/// # 示例
///
/// ```rust
/// use network::session::SessionConfig;
///
/// let config = SessionConfig::default();
/// assert_eq!(config.heartbeat_interval_ms, 5000);
/// assert_eq!(config.heartbeat_timeout_ms, 15000);
/// assert_eq!(config.handshake_timeout_ms, 10000);
/// assert_eq!(config.max_handshake_retries, 3);
/// ```
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// 心跳发送间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 心跳超时时间（毫秒），超过此时间无响应认为对端离线
    pub heartbeat_timeout_ms: u64,
    /// 握手超时时间（毫秒）
    pub handshake_timeout_ms: u64,
    /// 最大握手重试次数
    pub max_handshake_retries: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: HEARTBEAT_INTERVAL_MS,
            heartbeat_timeout_ms: HEARTBEAT_TIMEOUT_MS,
            handshake_timeout_ms: HANDSHAKE_TIMEOUT_MS,
            max_handshake_retries: MAX_HANDSHAKE_RETRIES,
        }
    }
}

/// 会话角色
///
/// 决定会话在握手过程中的行为模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRole {
    /// 客户端（发起握手方）
    Client,
    /// 服务器（接受握手方）
    Server,
}

/// 会话句柄
///
/// 管理单个音频传输会话的完整生命周期：握手 → 建立 → 心跳维持 → 优雅断开。
///
/// # 生命周期
///
/// 1. **创建**：通过 [`Session::new_client`] 或 [`Session::new_server`] 创建
/// 2. **握手**：客户端调用 [`Session::initiate_handshake`]，双方交换握手消息
/// 3. **建立**：状态到达 `Established` 后可收发数据
/// 4. **心跳**：定期调用 [`Session::create_heartbeat`] 维持连接
/// 5. **断开**：调用 [`Session::initiate_disconnect`] 优雅断开
///
/// # 示例
///
/// ```rust,no_run
/// use network::session::*;
///
/// let session_id = generate_session_id();
/// let config = SessionConfig::default();
///
/// // 创建客户端会话
/// let mut client = Session::new_client(
///     session_id.clone(),
///     Capability::default(),
///     config.clone(),
/// );
///
/// // 发起握手
/// let client_hello = client.initiate_handshake().unwrap();
/// // ... 发送 client_hello 到服务器，接收 server_hello ...
/// ```
pub struct Session {
    /// 会话 ID（UUID 格式）
    session_id: String,
    /// 当前状态
    state: SessionState,
    /// 会话角色
    role: SessionRole,
    /// 本端能力
    local_capability: Capability,
    /// 远端能力
    remote_capability: Option<Capability>,
    /// 协商参数
    negotiated: Option<NegotiatedParams>,
    /// 本端 ECDH 公钥
    local_public_key: EcdhPublicKey,
    /// 本端 ECDH 临时私钥（用于 DH 计算，使用后清空）
    local_secret: Option<EphemeralSecret>,
    /// 远端 ECDH 公钥
    remote_public_key: Option<EcdhPublicKey>,
    /// 共享密钥（由 X25519 ECDH + HKDF 派生）
    shared_secret: Option<Vec<u8>>,
    /// 配置
    config: SessionConfig,
    /// 统计信息
    stats: SessionStats,
    /// 最后一次活动时间
    last_activity: Instant,
    /// 握手开始时间
    handshake_start: Option<Instant>,
    /// 心跳序列号
    heartbeat_sequence: u32,
    /// 是否已收到断开请求
    disconnect_requested: bool,
}

impl Session {
    // ── 工厂方法 ──────────────────────────────────────────────

    /// 创建客户端会话
    ///
    /// 客户端负责发起握手（发送 ClientHello）。
    ///
    /// # Arguments
    ///
    /// * `session_id` - 会话唯一标识（UUID 格式）
    /// * `capability` - 本端能力声明
    /// * `config` - 会话配置
    ///
    /// # Returns
    ///
    /// 初始状态为 `Idle` 的客户端会话。
    pub fn new_client(session_id: String, capability: Capability, config: SessionConfig) -> Self {
        let (secret, public_key) = EcdhPublicKey::generate_keypair();
        Self {
            session_id,
            state: SessionState::Idle,
            role: SessionRole::Client,
            local_capability: capability,
            remote_capability: None,
            negotiated: None,
            local_public_key: public_key,
            local_secret: Some(secret),
            remote_public_key: None,
            shared_secret: None,
            config,
            stats: SessionStats::default(),
            last_activity: Instant::now(),
            handshake_start: None,
            heartbeat_sequence: 0,
            disconnect_requested: false,
        }
    }

    /// 创建服务器会话
    ///
    /// 服务器等待客户端的 ClientHello 并响应 ServerHello。
    ///
    /// # Arguments
    ///
    /// * `session_id` - 会话唯一标识（可为空，从 ClientHello 获取）
    /// * `capability` - 本端能力声明
    /// * `config` - 会话配置
    ///
    /// # Returns
    ///
    /// 初始状态为 `Idle` 的服务器会话。
    pub fn new_server(session_id: String, capability: Capability, config: SessionConfig) -> Self {
        let (secret, public_key) = EcdhPublicKey::generate_keypair();
        Self {
            session_id,
            state: SessionState::Idle,
            role: SessionRole::Server,
            local_capability: capability,
            remote_capability: None,
            negotiated: None,
            local_public_key: public_key,
            local_secret: Some(secret),
            remote_public_key: None,
            shared_secret: None,
            config,
            stats: SessionStats::default(),
            last_activity: Instant::now(),
            handshake_start: None,
            heartbeat_sequence: 0,
            disconnect_requested: false,
        }
    }

    // ── 访问器 ──────────────────────────────────────────────

    /// 获取会话 ID
    ///
    /// # Returns
    ///
    /// UUID 格式的会话标识字符串。
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 获取当前会话状态
    ///
    /// # Returns
    ///
    /// 当前 [`SessionState`] 枚举值。
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// 获取会话角色
    ///
    /// # Returns
    ///
    /// [`SessionRole::Client`] 或 [`SessionRole::Server`]。
    pub fn role(&self) -> SessionRole {
        self.role
    }

    /// 获取本端能力声明
    pub fn local_capability(&self) -> &Capability {
        &self.local_capability
    }

    /// 获取远端能力声明
    ///
    /// # Returns
    ///
    /// - `Some(&Capability)` — 已收到远端能力
    /// - `None` — 尚未收到
    pub fn remote_capability(&self) -> Option<&Capability> {
        self.remote_capability.as_ref()
    }

    /// 获取协商参数
    ///
    /// # Returns
    ///
    /// - `Some(&NegotiatedParams)` — 协商已完成
    /// - `None` — 尚未完成协商
    pub fn negotiated(&self) -> Option<&NegotiatedParams> {
        self.negotiated.as_ref()
    }

    /// 获取会话统计信息
    pub fn stats(&self) -> &SessionStats {
        &self.stats
    }

    /// 判断会话是否已建立（状态为 `Established`）
    pub fn is_established(&self) -> bool {
        self.state == SessionState::Established
    }

    /// 判断会话是否已关闭（状态为 `Closed`）
    pub fn is_closed(&self) -> bool {
        self.state == SessionState::Closed
    }

    // ── 客户端握手 ────────────────────────────────────────────

    /// 发起握手（客户端），生成 ClientHello 消息
    ///
    /// 将状态从 `Idle` 转换为 `ClientHelloSent`，并生成包含本端能力声明
    /// 和 ECDH 公钥的 ClientHello 消息。
    ///
    /// # Returns
    ///
    /// 需要发送给服务器的 [`HandshakeMessage::ClientHello`] 消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 非客户端角色
    /// - [`NetworkError::ConnectionFailed`] — 当前状态不是 `Idle`
    pub fn initiate_handshake(&mut self) -> Result<HandshakeMessage> {
        if self.role != SessionRole::Client {
            return Err(NetworkError::ConnectionFailed(
                "Only client can initiate handshake".into(),
            ));
        }
        if self.state != SessionState::Idle {
            return Err(NetworkError::ConnectionFailed(format!(
                "Cannot initiate handshake in state: {}",
                self.state
            )));
        }

        self.handshake_start = Some(Instant::now());
        self.state = SessionState::ClientHelloSent;
        self.last_activity = Instant::now();

        Ok(HandshakeMessage::ClientHello {
            session_id: self.session_id.clone(),
            capabilities: self.local_capability.clone(),
            client_public_key: self.local_public_key.clone(),
        })
    }

    /// 处理 ServerHello（客户端）
    ///
    /// 验证服务器返回的协商参数，派生共享密钥，生成 KeyExchange 响应。
    ///
    /// # Arguments
    ///
    /// * `msg` - 服务器发来的 ServerHello 消息
    ///
    /// # Returns
    ///
    /// 需要发送回服务器的 [`HandshakeMessage::KeyExchange`] 消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 非客户端角色
    /// - [`NetworkError::ConnectionFailed`] — 当前状态不是 `ClientHelloSent`
    /// - [`NetworkError::ConnectionFailed`] — 会话 ID 不匹配
    /// - [`NetworkError::ConnectionFailed`] — 协商参数不被本端支持
    pub fn handle_server_hello(&mut self, msg: &HandshakeMessage) -> Result<HandshakeMessage> {
        if self.role != SessionRole::Client {
            return Err(NetworkError::ConnectionFailed(
                "Only client handles ServerHello".into(),
            ));
        }
        if self.state != SessionState::ClientHelloSent {
            return Err(NetworkError::ConnectionFailed(format!(
                "Expected ClientHelloSent state, got: {}",
                self.state
            )));
        }

        let (session_id, capabilities, negotiated, server_public_key) = match msg {
            HandshakeMessage::ServerHello {
                session_id,
                capabilities,
                negotiated,
                server_public_key,
            } => (session_id, capabilities, negotiated, server_public_key),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected ServerHello message".into(),
                ))
            }
        };

        // 验证会话 ID
        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        // 验证协商参数
        self.validate_negotiation(negotiated)?;

        self.remote_capability = Some(capabilities.clone());
        self.negotiated = Some(negotiated.clone());
        self.remote_public_key = Some(server_public_key.clone());
        self.derive_shared_secret()?;
        self.state = SessionState::KeyExchangeSent;
        self.last_activity = Instant::now();

        Ok(HandshakeMessage::KeyExchange {
            session_id: self.session_id.clone(),
            negotiated: negotiated.clone(),
            client_public_key: self.local_public_key.clone(),
        })
    }

    /// 处理 Finished（客户端）
    ///
    /// 收到服务器的 Finished 消息后，将状态设为 `Established`，
    /// 记录握手耗时和会话开始时间。
    ///
    /// # Arguments
    ///
    /// * `msg` - 服务器发来的 Finished 消息
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 非客户端角色
    /// - [`NetworkError::ConnectionFailed`] — 当前状态不是 `KeyExchangeSent`
    /// - [`NetworkError::ConnectionFailed`] — 会话 ID 不匹配
    pub fn handle_finished_client(&mut self, msg: &HandshakeMessage) -> Result<()> {
        if self.role != SessionRole::Client {
            return Err(NetworkError::ConnectionFailed(
                "Only client handles Finished from server".into(),
            ));
        }
        if self.state != SessionState::KeyExchangeSent {
            return Err(NetworkError::ConnectionFailed(format!(
                "Expected KeyExchangeSent state, got: {}",
                self.state
            )));
        }

        let session_id = match msg {
            HandshakeMessage::Finished { session_id, .. } => session_id,
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected Finished message".into(),
                ))
            }
        };

        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        self.state = SessionState::Established;
        self.stats.started_at = Some(Instant::now());
        if let Some(start) = self.handshake_start {
            self.stats.handshake_duration_ms = Some(start.elapsed().as_millis() as u64);
        }
        self.last_activity = Instant::now();

        Ok(())
    }

    // ── 服务器握手 ────────────────────────────────────────────

    /// 处理 ClientHello（服务器），生成 ServerHello 消息
    ///
    /// 接收客户端的能力声明，执行能力协商，派生共享密钥，
    /// 生成包含协商结果的 ServerHello 响应。
    ///
    /// # Arguments
    ///
    /// * `msg` - 客户端发来的 ClientHello 消息
    ///
    /// # Returns
    ///
    /// 需要发送回客户端的 [`HandshakeMessage::ServerHello`] 消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 非服务器角色
    /// - [`NetworkError::ConnectionFailed`] — 当前状态不是 `Idle`
    pub fn handle_client_hello(&mut self, msg: &HandshakeMessage) -> Result<HandshakeMessage> {
        if self.role != SessionRole::Server {
            return Err(NetworkError::ConnectionFailed(
                "Only server handles ClientHello".into(),
            ));
        }
        if self.state != SessionState::Idle {
            return Err(NetworkError::ConnectionFailed(format!(
                "Cannot handle ClientHello in state: {}",
                self.state
            )));
        }

        let (session_id, capabilities, client_public_key) = match msg {
            HandshakeMessage::ClientHello {
                session_id,
                capabilities,
                client_public_key,
            } => (session_id, capabilities, client_public_key),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected ClientHello message".into(),
                ))
            }
        };

        self.handshake_start = Some(Instant::now());
        self.session_id = session_id.clone();
        self.remote_capability = Some(capabilities.clone());
        self.remote_public_key = Some(client_public_key.clone());

        // 协商能力
        let negotiated = self.negotiate(capabilities);
        self.negotiated = Some(negotiated.clone());
        self.derive_shared_secret()?;

        self.state = SessionState::ServerHelloSent;
        self.last_activity = Instant::now();

        Ok(HandshakeMessage::ServerHello {
            session_id: self.session_id.clone(),
            capabilities: self.local_capability.clone(),
            negotiated,
            server_public_key: self.local_public_key.clone(),
        })
    }

    /// 处理 KeyExchange（服务器），生成 Finished 消息
    ///
    /// 验证客户端确认的协商参数一致性，将状态设为 `Established`，
    /// 生成包含握手摘要的 Finished 响应。
    ///
    /// # Arguments
    ///
    /// * `msg` - 客户端发来的 KeyExchange 消息
    ///
    /// # Returns
    ///
    /// 需要发送回客户端的 [`HandshakeMessage::Finished`] 消息。
    ///
    /// # Errors
    ///
    /// - [`NetworkError::ConnectionFailed`] — 非服务器角色
    /// - [`NetworkError::ConnectionFailed`] — 当前状态不是 `ServerHelloSent`
    /// - [`NetworkError::ConnectionFailed`] — 会话 ID 或协商参数不匹配
    pub fn handle_key_exchange(&mut self, msg: &HandshakeMessage) -> Result<HandshakeMessage> {
        if self.role != SessionRole::Server {
            return Err(NetworkError::ConnectionFailed(
                "Only server handles KeyExchange".into(),
            ));
        }
        if self.state != SessionState::ServerHelloSent {
            return Err(NetworkError::ConnectionFailed(format!(
                "Expected ServerHelloSent state, got: {}",
                self.state
            )));
        }

        let (session_id, negotiated, client_public_key) = match msg {
            HandshakeMessage::KeyExchange {
                session_id,
                negotiated,
                client_public_key,
            } => (session_id, negotiated, client_public_key),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected KeyExchange message".into(),
                ))
            }
        };

        // 验证会话 ID
        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        // 验证协商参数一致性
        if Some(negotiated) != self.negotiated.as_ref() {
            return Err(NetworkError::ConnectionFailed(
                "Negotiated params mismatch".into(),
            ));
        }

        // 更新客户端公钥（如果变更）
        self.remote_public_key = Some(client_public_key.clone());
        // 仅在共享密钥尚未派生时执行 DH（handle_client_hello 已消耗私钥）
        if self.shared_secret.is_none() {
            self.derive_shared_secret()?;
        }

        self.state = SessionState::Established;
        self.stats.started_at = Some(Instant::now());
        if let Some(start) = self.handshake_start {
            self.stats.handshake_duration_ms = Some(start.elapsed().as_millis() as u64);
        }
        self.last_activity = Instant::now();

        // 计算握手摘要
        let hash = self.compute_handshake_hash()?;

        Ok(HandshakeMessage::Finished {
            session_id: self.session_id.clone(),
            handshake_hash: hash,
        })
    }

    // ── 心跳 ────────────────────────────────────────────────

    /// 生成心跳消息
    ///
    /// 仅在 `Established` 状态下可用。自动递增心跳序列号并更新统计。
    ///
    /// # Returns
    ///
    /// 需要发送给对端的 [`HandshakeMessage::Heartbeat`] 消息。
    ///
    /// # Errors
    ///
    /// 如果当前状态不是 `Established`，返回 [`NetworkError::ConnectionFailed`]。
    pub fn create_heartbeat(&mut self) -> Result<HandshakeMessage> {
        if self.state != SessionState::Established {
            return Err(NetworkError::ConnectionFailed(format!(
                "Cannot send heartbeat in state: {}",
                self.state
            )));
        }

        self.heartbeat_sequence += 1;
        self.stats.heartbeat.sent += 1;
        self.last_activity = Instant::now();

        Ok(HandshakeMessage::Heartbeat {
            session_id: self.session_id.clone(),
            sequence: self.heartbeat_sequence,
            timestamp_us: current_timestamp_us(),
        })
    }

    /// 处理心跳消息，生成心跳响应
    ///
    /// 验证会话 ID 后，返回对应的 HeartbeatAck 消息。
    ///
    /// # Arguments
    ///
    /// * `msg` - 对端发来的 Heartbeat 消息
    ///
    /// # Returns
    ///
    /// 需要发送回对端的 [`HandshakeMessage::HeartbeatAck`] 消息。
    ///
    /// # Errors
    ///
    /// - 当前状态不是 `Established`
    /// - 会话 ID 不匹配
    pub fn handle_heartbeat(&mut self, msg: &HandshakeMessage) -> Result<HandshakeMessage> {
        if self.state != SessionState::Established {
            return Err(NetworkError::ConnectionFailed(format!(
                "Cannot handle heartbeat in state: {}",
                self.state
            )));
        }

        let (session_id, sequence, timestamp_us) = match msg {
            HandshakeMessage::Heartbeat {
                session_id,
                sequence,
                timestamp_us,
            } => (session_id, sequence, timestamp_us),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected Heartbeat message".into(),
                ))
            }
        };

        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        self.last_activity = Instant::now();

        Ok(HandshakeMessage::HeartbeatAck {
            session_id: self.session_id.clone(),
            sequence: *sequence,
            timestamp_us: *timestamp_us,
        })
    }

    /// 处理心跳响应，计算 RTT
    ///
    /// 更新心跳统计信息，计算并记录往返时延（RTT）。
    ///
    /// # Arguments
    ///
    /// * `msg` - 对端发来的 HeartbeatAck 消息
    ///
    /// # Errors
    ///
    /// - 会话 ID 不匹配
    pub fn handle_heartbeat_ack(&mut self, msg: &HandshakeMessage) -> Result<()> {
        let (session_id, _sequence, timestamp_us) = match msg {
            HandshakeMessage::HeartbeatAck {
                session_id,
                sequence,
                timestamp_us,
            } => (session_id, sequence, timestamp_us),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected HeartbeatAck message".into(),
                ))
            }
        };

        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        self.stats.heartbeat.acked += 1;
        let now_us = current_timestamp_us();
        if now_us > *timestamp_us {
            self.stats.heartbeat.last_rtt_us = Some(now_us - timestamp_us);
        }
        self.last_activity = Instant::now();

        Ok(())
    }

    /// 检查是否需要发送心跳
    ///
    /// 当距离上次活动时间超过心跳间隔时返回 `true`。
    ///
    /// # Returns
    ///
    /// `true` 表示应该立即发送心跳消息。
    pub fn should_send_heartbeat(&self) -> bool {
        if self.state != SessionState::Established {
            return false;
        }
        self.last_activity.elapsed() >= Duration::from_millis(self.config.heartbeat_interval_ms)
    }

    /// 检查心跳是否超时
    ///
    /// 当距离上次活动时间超过心跳超时时间时返回 `true`，
    /// 表示对端可能已离线。
    ///
    /// # Returns
    ///
    /// `true` 表示心跳超时，应考虑断开连接。
    pub fn check_heartbeat_timeout(&self) -> bool {
        if self.state != SessionState::Established {
            return false;
        }
        self.last_activity.elapsed() >= Duration::from_millis(self.config.heartbeat_timeout_ms)
    }

    /// 检查握手是否超时
    ///
    /// 当握手已开始且耗时超过配置的超时时间时返回 `true`。
    /// 终态（Established 或 Closed）下始终返回 `false`。
    ///
    /// # Returns
    ///
    /// `true` 表示握手超时，应放弃握手。
    pub fn check_handshake_timeout(&self) -> bool {
        if self.state.is_terminal() {
            return false;
        }
        if let Some(start) = self.handshake_start {
            start.elapsed() >= Duration::from_millis(self.config.handshake_timeout_ms)
        } else {
            false
        }
    }

    // ── 断开 ────────────────────────────────────────────────

    /// 发起优雅断开
    ///
    /// 生成 Disconnect 消息发送给对端。本端不会立即关闭，
    /// 等待对端确认或超时后自行关闭。
    ///
    /// # Arguments
    ///
    /// * `reason` - 断开原因
    ///
    /// # Returns
    ///
    /// 需要发送给对端的 [`HandshakeMessage::Disconnect`] 消息。
    ///
    /// # Errors
    ///
    /// 如果会话已关闭，返回 [`NetworkError::ConnectionFailed`]。
    pub fn initiate_disconnect(&mut self, reason: DisconnectReason) -> Result<HandshakeMessage> {
        if self.state == SessionState::Closed {
            return Err(NetworkError::ConnectionFailed(
                "Session already closed".into(),
            ));
        }

        self.disconnect_requested = true;

        Ok(HandshakeMessage::Disconnect {
            session_id: self.session_id.clone(),
            reason,
        })
    }

    /// 处理断开请求
    ///
    /// 收到对端的 Disconnect 消息后，将状态设为 `Closed`。
    ///
    /// # Arguments
    ///
    /// * `msg` - 对端发来的 Disconnect 消息
    ///
    /// # Errors
    ///
    /// - 会话 ID 不匹配
    pub fn handle_disconnect(&mut self, msg: &HandshakeMessage) -> Result<()> {
        let (session_id, reason) = match msg {
            HandshakeMessage::Disconnect { session_id, reason } => (session_id, reason),
            _ => {
                return Err(NetworkError::ConnectionFailed(
                    "Expected Disconnect message".into(),
                ))
            }
        };

        if *session_id != self.session_id {
            return Err(NetworkError::ConnectionFailed(format!(
                "Session ID mismatch: expected {}, got {}",
                self.session_id, session_id
            )));
        }

        tracing::info!(
            "Session {} disconnecting: reason={}",
            self.session_id,
            reason
        );

        self.disconnect_requested = true;
        self.state = SessionState::Closed;
        self.last_activity = Instant::now();

        Ok(())
    }

    /// 直接关闭会话
    ///
    /// 将状态设为 `Closed`，不发送 Disconnect 消息。
    /// 用于异常情况或已收到对端断开确认后。
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
    }

    // ── 序列化 ──────────────────────────────────────────────

    /// 序列化握手消息为字节
    ///
    /// 使用 bincode 序列化，用于网络传输。
    ///
    /// # Arguments
    ///
    /// * `msg` - 要序列化的握手消息
    ///
    /// # Returns
    ///
    /// 序列化后的字节数组。
    ///
    /// # Errors
    ///
    /// 如果序列化失败，返回 [`NetworkError::SerializationError`]。
    pub fn serialize_message(msg: &HandshakeMessage) -> Result<Vec<u8>> {
        bincode::serialize(msg).map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// 从字节反序列化握手消息
    ///
    /// # Arguments
    ///
    /// * `data` - 序列化的字节数据
    ///
    /// # Returns
    ///
    /// 反序列化后的握手消息。
    ///
    /// # Errors
    ///
    /// 如果反序列化失败，返回 [`NetworkError::SerializationError`]。
    pub fn deserialize_message(data: &[u8]) -> Result<HandshakeMessage> {
        bincode::deserialize(data).map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    // ── 内部方法 ──────────────────────────────────────────────

    /// 协商能力（服务器侧）
    fn negotiate(&self, client_cap: &Capability) -> NegotiatedParams {
        // 传输协议：选择双方都支持的最高优先级协议
        let transport = self.select_transport(&client_cap.transport_protocols);

        // 加密模式：选择双方都支持的最高优先级加密模式
        let encryption = self.select_encryption(&client_cap.encryption_modes);

        // Opus 配置：使用服务器的配置（或协商结果）
        let opus = self.local_capability.opus_config.clone();

        NegotiatedParams {
            transport_protocol: transport,
            encryption_mode: encryption,
            opus_config: opus,
        }
    }

    /// 选择传输协议
    fn select_transport(&self, client_protocols: &[TransportProtocol]) -> TransportProtocol {
        for proto in &self.local_capability.transport_protocols {
            if client_protocols.contains(proto) {
                return *proto;
            }
        }
        // 默认使用 UDP
        TransportProtocol::Udp
    }

    /// 选择加密模式
    fn select_encryption(&self, client_modes: &[EncryptionMode]) -> EncryptionMode {
        for mode in &self.local_capability.encryption_modes {
            if client_modes.contains(mode) {
                return *mode;
            }
        }
        // 默认使用 SRTP
        EncryptionMode::Srtp
    }

    /// 验证协商参数（客户端侧）
    fn validate_negotiation(&self, negotiated: &NegotiatedParams) -> Result<()> {
        if !self
            .local_capability
            .transport_protocols
            .contains(&negotiated.transport_protocol)
        {
            return Err(NetworkError::ConnectionFailed(format!(
                "Unsupported transport protocol: {}",
                negotiated.transport_protocol
            )));
        }
        if !self
            .local_capability
            .encryption_modes
            .contains(&negotiated.encryption_mode)
        {
            return Err(NetworkError::ConnectionFailed(format!(
                "Unsupported encryption mode: {}",
                negotiated.encryption_mode
            )));
        }
        Ok(())
    }

    /// 使用 X25519 ECDH 派生共享密钥
    ///
    /// 通过本地临时私钥和远端公钥执行 Diffie-Hellman 计算，
    /// 再使用 HKDF-SHA1 派生 32 字节共享密钥。
    fn derive_shared_secret(&mut self) -> Result<()> {
        let secret = self
            .local_secret
            .take()
            .ok_or_else(|| NetworkError::CryptoError("本地私钥不存在".into()))?;
        let remote_key = self
            .remote_public_key
            .as_ref()
            .ok_or_else(|| NetworkError::CryptoError("远端公钥不存在".into()))?;

        let shared = secret.diffie_hellman(&remote_key.to_x25519_public());

        if !shared.was_contributory() {
            return Err(NetworkError::CryptoError(
                "密钥交换非贡献性（低阶点攻击）".into(),
            ));
        }

        // 使用 HKDF 从原始共享密钥派生 32 字节会话密钥
        use hkdf::Hkdf;
        use sha1::Sha1;
        let hk = Hkdf::<Sha1>::new(Some(b"SoundBridge-ECDH"), shared.as_bytes());
        let mut derived = [0u8; 32];
        hk.expand(b"SoundBridge Session Key", &mut derived)
            .map_err(|e| NetworkError::CryptoError(format!("HKDF 密钥派生失败: {}", e)))?;

        self.shared_secret = Some(derived.to_vec());
        Ok(())
    }

    /// 计算握手摘要（HMAC-SHA1）
    ///
    /// 使用共享密钥对会话 ID 计算 HMAC-SHA1 摘要，
    /// 用于 Finished 消息的握手完整性验证。
    fn compute_handshake_hash(&self) -> Result<Vec<u8>> {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        if let Some(ref secret) = self.shared_secret {
            let mut mac = Hmac::<Sha1>::new_from_slice(secret)
                .map_err(|e| NetworkError::CryptoError(format!("HMAC key error: {e}")))?;
            mac.update(self.session_id.as_bytes());
            Ok(mac.finalize().into_bytes().to_vec())
        } else {
            Ok(vec![0u8; 20]) // SHA1 输出长度
        }
    }
}

// ──────────────────────────────── 工具函数 ────────────────────────────────

/// 生成 UUID v4 格式的会话 ID
///
/// 使用密码学安全随机数生成符合 UUID v4 规范的会话标识符。
///
/// # Returns
///
/// 格式为 `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx` 的 UUID 字符串。
///
/// # 示例
///
/// ```rust
/// use network::session::generate_session_id;
///
/// let id1 = generate_session_id();
/// let id2 = generate_session_id();
/// assert_ne!(id1, id2);
/// assert_eq!(id1.len(), 36); // UUID 格式长度
/// assert!(id1.contains('-'));
/// ```
pub fn generate_session_id() -> String {
    let bytes: [u8; 16] = rand::random();

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        (bytes[6] & 0x0f) | 0x40, bytes[7], // version 4
        (bytes[8] & 0x3f) | 0x80, bytes[9], // variant 1
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// 获取当前时间戳（微秒）
fn current_timestamp_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

// ──────────────────────────────── 测试 ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_capability(device_id: &str, device_name: &str) -> Capability {
        Capability {
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            ..Default::default()
        }
    }

    // ── 状态机测试 ──────────────────────────────────────────────

    #[test]
    fn test_session_state_display() {
        assert_eq!(format!("{}", SessionState::Idle), "Idle");
        assert_eq!(format!("{}", SessionState::Established), "Established");
        assert_eq!(format!("{}", SessionState::Closed), "Closed");
    }

    #[test]
    fn test_session_state_is_terminal() {
        assert!(!SessionState::Idle.is_terminal());
        assert!(!SessionState::ClientHelloSent.is_terminal());
        assert!(SessionState::Established.is_terminal());
        assert!(SessionState::Closed.is_terminal());
    }

    #[test]
    fn test_session_state_can_send_data() {
        assert!(!SessionState::Idle.can_send_data());
        assert!(!SessionState::ClientHelloSent.can_send_data());
        assert!(SessionState::Established.can_send_data());
        assert!(!SessionState::Closed.can_send_data());
    }

    // ── 能力协商测试 ──────────────────────────────────────────────

    #[test]
    fn test_transport_protocol_display() {
        assert_eq!(format!("{}", TransportProtocol::Udp), "UDP");
        assert_eq!(format!("{}", TransportProtocol::Quic), "QUIC");
    }

    #[test]
    fn test_encryption_mode_display() {
        assert_eq!(format!("{}", EncryptionMode::None), "None");
        assert_eq!(format!("{}", EncryptionMode::Srtp), "SRTP");
    }

    #[test]
    fn test_default_capability() {
        let cap = Capability::default();
        assert_eq!(cap.version, SESSION_PROTOCOL_VERSION);
        assert_eq!(cap.transport_protocols[0], TransportProtocol::Udp);
        assert_eq!(cap.encryption_modes[0], EncryptionMode::Srtp);
        assert_eq!(cap.opus_config.sample_rate, 48000);
    }

    #[test]
    fn test_negotiation_selects_matching() {
        let server = Session::new_server(
            "test".into(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );

        let client_cap = Capability {
            transport_protocols: vec![TransportProtocol::Quic, TransportProtocol::Udp],
            encryption_modes: vec![EncryptionMode::None, EncryptionMode::Srtp],
            ..default_capability("c1", "Client")
        };

        let negotiated = server.negotiate(&client_cap);
        // 服务器优先 UDP，客户端支持 UDP → 选 UDP
        assert_eq!(negotiated.transport_protocol, TransportProtocol::Udp);
        // 服务器优先 SRTP，客户端有 Srtp（在第二位）→ 选 Srtp
        assert_eq!(negotiated.encryption_mode, EncryptionMode::Srtp);
    }

    #[test]
    fn test_negotiation_no_common_transport_defaults() {
        let server = Session::new_server(
            "test".into(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );

        let client_cap = Capability {
            transport_protocols: vec![], // empty
            encryption_modes: vec![EncryptionMode::Srtp],
            ..default_capability("c1", "Client")
        };

        let negotiated = server.negotiate(&client_cap);
        assert_eq!(negotiated.transport_protocol, TransportProtocol::Udp);
    }

    #[test]
    fn test_validate_negotiation_rejects_unsupported() {
        let client = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        let bad_negotiated = NegotiatedParams {
            transport_protocol: TransportProtocol::Quic,
            encryption_mode: EncryptionMode::Srtp,
            opus_config: OpusConfig::default(),
        };

        // 客户端默认支持 [Udp, Quic]，所以 Quic 应该通过
        assert!(client.validate_negotiation(&bad_negotiated).is_ok());

        // 测试不支持的加密模式
        let client_no_none = Session::new_client(
            "test".into(),
            Capability {
                encryption_modes: vec![EncryptionMode::Srtp],
                ..default_capability("c1", "Client")
            },
            SessionConfig::default(),
        );

        let bad_encryption = NegotiatedParams {
            transport_protocol: TransportProtocol::Udp,
            encryption_mode: EncryptionMode::None,
            opus_config: OpusConfig::default(),
        };

        assert!(client_no_none
            .validate_negotiation(&bad_encryption)
            .is_err());
    }

    // ── 完整握手流程测试 ──────────────────────────────────────────────

    #[test]
    fn test_full_handshake_flow() {
        let session_id = generate_session_id();
        let config = SessionConfig::default();

        let mut client = Session::new_client(
            session_id.clone(),
            default_capability("client-1", "PC"),
            config.clone(),
        );
        let mut server = Session::new_server(
            String::new(), // 服务器从 ClientHello 获取 session_id
            default_capability("server-1", "Phone"),
            config,
        );

        // 1. Client 发起握手
        let client_hello = client.initiate_handshake().unwrap();
        assert_eq!(client.state(), SessionState::ClientHelloSent);

        // 2. Server 处理 ClientHello
        let server_hello = server.handle_client_hello(&client_hello).unwrap();
        assert_eq!(server.state(), SessionState::ServerHelloSent);
        assert!(server.negotiated().is_some());

        // 3. Client 处理 ServerHello
        let key_exchange = client.handle_server_hello(&server_hello).unwrap();
        assert_eq!(client.state(), SessionState::KeyExchangeSent);
        assert!(client.negotiated().is_some());

        // 4. Server 处理 KeyExchange
        let finished = server.handle_key_exchange(&key_exchange).unwrap();
        assert_eq!(server.state(), SessionState::Established);
        assert!(server.is_established());

        // 5. Client 处理 Finished
        client.handle_finished_client(&finished).unwrap();
        assert_eq!(client.state(), SessionState::Established);
        assert!(client.is_established());

        // 验证协商结果一致
        assert_eq!(client.negotiated(), server.negotiated());

        // 验证共享密钥已派生且双方一致
        assert!(client.shared_secret.is_some());
        assert!(server.shared_secret.is_some());
        assert_eq!(client.shared_secret, server.shared_secret);
    }

    #[test]
    fn test_handshake_wrong_state() {
        let mut client = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        // 未发起握手就处理 ServerHello 应该失败
        let fake_msg = HandshakeMessage::ServerHello {
            session_id: "test".into(),
            capabilities: default_capability("s1", "Server"),
            negotiated: NegotiatedParams {
                transport_protocol: TransportProtocol::Udp,
                encryption_mode: EncryptionMode::Srtp,
                opus_config: OpusConfig::default(),
            },
            server_public_key: EcdhPublicKey::generate(),
        };

        assert!(client.handle_server_hello(&fake_msg).is_err());
    }

    #[test]
    fn test_server_cannot_initiate() {
        let mut server = Session::new_server(
            "test".into(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );

        assert!(server.initiate_handshake().is_err());
    }

    #[test]
    fn test_handshake_session_id_mismatch() {
        let mut client = Session::new_client(
            "session-A".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        client.initiate_handshake().unwrap();

        let wrong_hello = HandshakeMessage::ServerHello {
            session_id: "session-B".into(),
            capabilities: default_capability("s1", "Server"),
            negotiated: NegotiatedParams {
                transport_protocol: TransportProtocol::Udp,
                encryption_mode: EncryptionMode::Srtp,
                opus_config: OpusConfig::default(),
            },
            server_public_key: EcdhPublicKey::generate(),
        };

        assert!(client.handle_server_hello(&wrong_hello).is_err());
    }

    // ── 心跳测试 ──────────────────────────────────────────────

    #[test]
    fn test_heartbeat_flow() {
        let session_id = generate_session_id();
        let config = SessionConfig::default();

        let mut client = Session::new_client(
            session_id.clone(),
            default_capability("c1", "Client"),
            config.clone(),
        );
        let mut server =
            Session::new_server(String::new(), default_capability("s1", "Server"), config);

        // 完成握手
        complete_handshake(&mut client, &mut server);

        // 客户端发送心跳
        let heartbeat = client.create_heartbeat().unwrap();
        match &heartbeat {
            HandshakeMessage::Heartbeat { sequence, .. } => assert_eq!(*sequence, 1),
            _ => panic!("Expected Heartbeat"),
        }

        // 服务器处理心跳
        let ack = server.handle_heartbeat(&heartbeat).unwrap();
        match &ack {
            HandshakeMessage::HeartbeatAck { sequence, .. } => assert_eq!(*sequence, 1),
            _ => panic!("Expected HeartbeatAck"),
        }

        // 客户端处理心跳响应
        client.handle_heartbeat_ack(&ack).unwrap();
        assert_eq!(client.stats().heartbeat.acked, 1);
    }

    #[test]
    fn test_heartbeat_wrong_state() {
        let mut client = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        // 未建立连接时不能发送心跳
        assert!(client.create_heartbeat().is_err());
    }

    #[test]
    fn test_heartbeat_loss_rate() {
        let mut stats = HeartbeatStats::default();
        assert_eq!(stats.loss_rate(), 0.0);

        stats.sent = 10;
        stats.lost = 3;
        assert!((stats.loss_rate() - 0.3).abs() < f32::EPSILON);
    }

    // ── 优雅断开测试 ──────────────────────────────────────────────

    #[test]
    fn test_graceful_disconnect() {
        let session_id = generate_session_id();
        let config = SessionConfig::default();

        let mut client = Session::new_client(
            session_id.clone(),
            default_capability("c1", "Client"),
            config.clone(),
        );
        let mut server =
            Session::new_server(String::new(), default_capability("s1", "Server"), config);

        complete_handshake(&mut client, &mut server);

        // 客户端发起断开
        let disconnect = client
            .initiate_disconnect(DisconnectReason::UserInitiated)
            .unwrap();

        // 服务器处理断开
        server.handle_disconnect(&disconnect).unwrap();
        assert!(server.is_closed());

        // 客户端也关闭
        client.close();
        assert!(client.is_closed());
    }

    #[test]
    fn test_disconnect_reason_display() {
        assert_eq!(
            format!("{}", DisconnectReason::UserInitiated),
            "User initiated"
        );
        assert_eq!(
            format!("{}", DisconnectReason::HeartbeatTimeout),
            "Heartbeat timeout"
        );
        assert_eq!(
            format!("{}", DisconnectReason::HandshakeFailed),
            "Handshake failed"
        );
    }

    #[test]
    fn test_disconnect_already_closed() {
        let mut session = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        session.close();
        assert!(session
            .initiate_disconnect(DisconnectReason::UserInitiated)
            .is_err());
    }

    // ── 超时检测测试 ──────────────────────────────────────────────

    #[test]
    fn test_handshake_timeout_check() {
        let config = SessionConfig {
            handshake_timeout_ms: 0, // 立即超时
            ..Default::default()
        };

        let mut client =
            Session::new_client("test".into(), default_capability("c1", "Client"), config);

        client.initiate_handshake().unwrap();

        // handshake_timeout_ms=0 意味着立即超时
        // 但需要至少 1ms 的时间流逝才能触发
        std::thread::sleep(Duration::from_millis(1));
        assert!(client.check_handshake_timeout());
    }

    #[test]
    fn test_heartbeat_timeout_check() {
        let config = SessionConfig {
            heartbeat_timeout_ms: 0, // 立即超时
            ..Default::default()
        };

        let mut client =
            Session::new_client("test".into(), default_capability("c1", "Client"), config);
        let mut server = Session::new_server(
            String::new(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );

        complete_handshake(&mut client, &mut server);

        std::thread::sleep(Duration::from_millis(1));
        assert!(client.check_heartbeat_timeout());
    }

    #[test]
    fn test_should_send_heartbeat() {
        let config = SessionConfig {
            heartbeat_interval_ms: 0, // 立即触发
            ..Default::default()
        };

        let mut client =
            Session::new_client("test".into(), default_capability("c1", "Client"), config);
        let mut server = Session::new_server(
            String::new(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );

        complete_handshake(&mut client, &mut server);

        std::thread::sleep(Duration::from_millis(1));
        assert!(client.should_send_heartbeat());
    }

    // ── 序列化测试 ──────────────────────────────────────────────

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = HandshakeMessage::ClientHello {
            session_id: "test-session".into(),
            capabilities: default_capability("c1", "Client"),
            client_public_key: EcdhPublicKey([0xAB; 32]),
        };

        let bytes = Session::serialize_message(&msg).unwrap();
        let decoded = Session::deserialize_message(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_all_message_types_serialize() {
        let messages = vec![
            HandshakeMessage::ClientHello {
                session_id: "s1".into(),
                capabilities: Capability::default(),
                client_public_key: EcdhPublicKey::generate(),
            },
            HandshakeMessage::ServerHello {
                session_id: "s1".into(),
                capabilities: Capability::default(),
                negotiated: NegotiatedParams {
                    transport_protocol: TransportProtocol::Udp,
                    encryption_mode: EncryptionMode::Srtp,
                    opus_config: OpusConfig::default(),
                },
                server_public_key: EcdhPublicKey::generate(),
            },
            HandshakeMessage::KeyExchange {
                session_id: "s1".into(),
                negotiated: NegotiatedParams {
                    transport_protocol: TransportProtocol::Udp,
                    encryption_mode: EncryptionMode::Srtp,
                    opus_config: OpusConfig::default(),
                },
                client_public_key: EcdhPublicKey::generate(),
            },
            HandshakeMessage::Finished {
                session_id: "s1".into(),
                handshake_hash: vec![0x42; 32],
            },
            HandshakeMessage::Heartbeat {
                session_id: "s1".into(),
                sequence: 1,
                timestamp_us: 1234567890,
            },
            HandshakeMessage::HeartbeatAck {
                session_id: "s1".into(),
                sequence: 1,
                timestamp_us: 1234567890,
            },
            HandshakeMessage::Disconnect {
                session_id: "s1".into(),
                reason: DisconnectReason::UserInitiated,
            },
        ];

        for msg in messages {
            let bytes = Session::serialize_message(&msg).unwrap();
            let decoded = Session::deserialize_message(&bytes).unwrap();
            assert_eq!(msg, decoded);
        }
    }

    #[test]
    fn test_serialize_invalid_data() {
        let result = Session::deserialize_message(&[0xFF, 0xFE, 0xFD]);
        assert!(result.is_err());
    }

    // ── UUID 生成测试 ──────────────────────────────────────────────

    #[test]
    fn test_generate_session_id_format() {
        let id = generate_session_id();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);

        // 验证 version 4 (第13-14字符应为 '4')
        assert_eq!(parts[2].as_bytes()[0], b'4');
    }

    #[test]
    fn test_generate_session_id_unique() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
    }

    // ── ECDH 模拟测试 ──────────────────────────────────────────────

    #[test]
    fn test_ecdh_public_key_generation() {
        let (_s1, key1) = EcdhPublicKey::generate_keypair();
        let (_s2, key2) = EcdhPublicKey::generate_keypair();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_shared_secret_derivation() {
        // 模拟完整的 ECDH 密钥交换
        let (alice_secret, alice_public) = EcdhPublicKey::generate_keypair();
        let (bob_secret, bob_public) = EcdhPublicKey::generate_keypair();

        // Alice 侧
        let mut client = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );
        // 替换为已知密钥对
        client.local_secret = Some(alice_secret);
        client.local_public_key = alice_public.clone();
        client.remote_public_key = Some(bob_public.clone());
        client.derive_shared_secret().unwrap();

        // Bob 侧
        let mut server = Session::new_server(
            "test".into(),
            default_capability("s1", "Server"),
            SessionConfig::default(),
        );
        server.local_secret = Some(bob_secret);
        server.local_public_key = bob_public;
        server.remote_public_key = Some(alice_public);
        server.derive_shared_secret().unwrap();

        // 双方应派生出相同的共享密钥
        assert!(client.shared_secret.is_some());
        assert!(server.shared_secret.is_some());
        assert_eq!(client.shared_secret, server.shared_secret);
        assert_eq!(client.shared_secret.as_ref().unwrap().len(), 32);
    }

    // ── 边界情况测试 ──────────────────────────────────────────────

    #[test]
    fn test_client_cannot_handle_heartbeat_before_established() {
        let mut client = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        let heartbeat = HandshakeMessage::Heartbeat {
            session_id: "test".into(),
            sequence: 1,
            timestamp_us: 123456,
        };

        assert!(client.handle_heartbeat(&heartbeat).is_err());
    }

    #[test]
    fn test_close_session() {
        let mut session = Session::new_client(
            "test".into(),
            default_capability("c1", "Client"),
            SessionConfig::default(),
        );

        assert!(!session.is_closed());
        session.close();
        assert!(session.is_closed());
        assert_eq!(session.state(), SessionState::Closed);
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.heartbeat_interval_ms, HEARTBEAT_INTERVAL_MS);
        assert_eq!(config.heartbeat_timeout_ms, HEARTBEAT_TIMEOUT_MS);
        assert_eq!(config.handshake_timeout_ms, HANDSHAKE_TIMEOUT_MS);
        assert_eq!(config.max_handshake_retries, MAX_HANDSHAKE_RETRIES);
    }

    #[test]
    fn test_session_stats_default() {
        let stats = SessionStats::default();
        assert_eq!(stats.heartbeat.sent, 0);
        assert_eq!(stats.heartbeat.acked, 0);
        assert_eq!(stats.heartbeat.lost, 0);
        assert!(stats.started_at.is_none());
        assert!(stats.handshake_duration_ms.is_none());
    }

    // ── 辅助函数 ──────────────────────────────────────────────

    /// 完成完整握手流程（测试辅助）
    fn complete_handshake(client: &mut Session, server: &mut Session) {
        let client_hello = client.initiate_handshake().unwrap();
        let server_hello = server.handle_client_hello(&client_hello).unwrap();
        let key_exchange = client.handle_server_hello(&server_hello).unwrap();
        let finished = server.handle_key_exchange(&key_exchange).unwrap();
        client.handle_finished_client(&finished).unwrap();
    }
}
