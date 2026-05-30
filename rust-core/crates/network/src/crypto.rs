//! # DTLS/SRTP 加密层
//!
//! 实现音频流的端到端加密，防止窃听和篡改。
//!
//! ## 安全机制
//!
//! ### SRTP 加密
//! - **加密算法**：AES-128-CM（Counter Mode）加密音频帧载荷
//! - **认证标签**：HMAC-SHA1-80（截断为 10 字节），验证数据完整性
//! - **密钥轮换**：每 2^31 个包自动轮换会话密钥，防止密钥过度使用
//! - **密钥派生**：基于 AES-CM PRF 的 SRTP KDF（符合 RFC 3711）
//!
//! ### DTLS 握手
//! - 自签名证书生成与指纹交换
//! - 可配置的握手超时和重试机制
//! - 会话密钥派生（HKDF-SHA1）
//!
//! ## 架构
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                   应用层（RTP 数据包）                │
//! └──────────────┬───────────────────────┬───────────────┘
//!                │                       │
//!         ┌──────▼──────┐         ┌──────▼──────┐
//!         │ SrtpContext  │         │ DtlsSession │
//!         │  protect()   │         │  握手流程    │
//!         │  unprotect() │         │  密钥派生    │
//!         └──────┬──────┘         └──────┬──────┘
//!                │                       │
//!         ┌──────▼──────┐         ┌──────▼──────┐
//!         │ AES-128-CM  │         │  HKDF-SHA1  │
//!         │ HMAC-SHA1   │         │  证书指纹    │
//!         └─────────────┘         └─────────────┘
//! ```

use aes::Aes128;
use cipher::{KeyIvInit, StreamCipher};
use ctr::Ctr32BE;
use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::{NetworkError, Result};

/// AES-128-CTR 类型别名
type Aes128Ctr = Ctr32BE<Aes128>;

/// HMAC-SHA1 类型别名
type HmacSha1 = Hmac<Sha1>;

/// SRTP 认证标签长度（HMAC-SHA1-80 = 80 位 = 10 字节）
///
/// 每个 SRTP 数据包末尾附加此长度的认证标签用于完整性验证。
pub const SRTP_AUTH_TAG_LEN: usize = 10;

/// AES-128 密钥长度（128 位 = 16 字节）
pub const AES_128_KEY_LEN: usize = 16;

/// SRTP 会话盐值长度（112 位 = 14 字节）
pub const SRTP_SALT_LEN: usize = 14;

/// SRTP 主密钥长度（128 位 = 16 字节）
pub const SRTP_MASTER_KEY_LEN: usize = 16;

/// SRTP 主盐值长度（112 位 = 14 字节）
pub const SRTP_MASTER_SALT_LEN: usize = 14;

/// 密钥轮换阈值（2^31 包）
///
/// 每发送/接收此数量的包后自动重新派生会话密钥。
pub const KEY_ROTATION_THRESHOLD: u64 = 1u64 << 31;

/// DTLS 握手默认超时时间（毫秒）
pub const DTLS_HANDSHAKE_TIMEOUT_MS: u64 = 5000;

/// DTLS 握手默认最大重试次数
pub const DTLS_MAX_RETRIES: u32 = 3;

/// SRTP 密钥派生标签
const KDF_LABEL_CIPHER: u8 = 0x00;
const KDF_LABEL_AUTH: u8 = 0x01;
const KDF_LABEL_SALT: u8 = 0x02;

/// SRTP 密钥材料
///
/// 包含从 DTLS 握手派生的主密钥和主盐值，用于派生实际的会话加密密钥、
/// 认证密钥和会话盐值。
///
/// # 安全说明
///
/// `Debug` 实现会隐藏密钥内容（显示为 `[REDACTED]`），防止日志泄露。
///
/// # 示例
///
/// ```rust
/// use network::crypto::CryptoKeys;
///
/// // 随机生成新密钥
/// let keys = CryptoKeys::generate();
/// assert_ne!(keys.master_key, [0u8; 16]);
///
/// // 从已知字节构造（测试用）
/// let key = [0x42u8; 16];
/// let salt = [0x69u8; 14];
/// let keys = CryptoKeys::from_bytes(&key, &salt);
/// ```
#[derive(Clone)]
pub struct CryptoKeys {
    /// 主加密密钥（16 字节）
    pub master_key: [u8; SRTP_MASTER_KEY_LEN],
    /// 主盐值（14 字节）
    pub master_salt: [u8; SRTP_MASTER_SALT_LEN],
}

impl std::fmt::Debug for CryptoKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CryptoKeys")
            .field("master_key", &"[REDACTED]")
            .field("master_salt", &"[REDACTED]")
            .finish()
    }
}

impl CryptoKeys {
    /// 从密码学安全随机源生成新的密钥材料
    ///
    /// 使用操作系统提供的密码学安全随机数生成器（CSPRNG）填充
    /// 主密钥和主盐值。
    ///
    /// # Returns
    ///
    /// 包含随机主密钥和主盐值的 `CryptoKeys` 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::CryptoKeys;
    ///
    /// let keys = CryptoKeys::generate();
    /// // 每次生成的密钥都不同
    /// let keys2 = CryptoKeys::generate();
    /// assert_ne!(keys.master_key, keys2.master_key);
    /// ```
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut master_key = [0u8; SRTP_MASTER_KEY_LEN];
        let mut master_salt = [0u8; SRTP_MASTER_SALT_LEN];
        rng.fill_bytes(&mut master_key);
        rng.fill_bytes(&mut master_salt);
        Self {
            master_key,
            master_salt,
        }
    }

    /// 从原始字节构造密钥材料
    ///
    /// 用于测试场景或已知密钥的导入。
    ///
    /// # Arguments
    ///
    /// * `key` - 主加密密钥（16 字节）
    /// * `salt` - 主盐值（14 字节）
    ///
    /// # Returns
    ///
    /// 包含指定密钥材料的 `CryptoKeys` 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::CryptoKeys;
    ///
    /// let key = [0x01u8; 16];
    /// let salt = [0x02u8; 14];
    /// let keys = CryptoKeys::from_bytes(&key, &salt);
    /// assert_eq!(keys.master_key, key);
    /// assert_eq!(keys.master_salt, salt);
    /// ```
    pub fn from_bytes(key: &[u8; SRTP_MASTER_KEY_LEN], salt: &[u8; SRTP_MASTER_SALT_LEN]) -> Self {
        Self {
            master_key: *key,
            master_salt: *salt,
        }
    }

    /// 派生会话加密密钥
    fn derive_cipher_key(&self) -> Result<[u8; AES_128_KEY_LEN]> {
        srtp_kdf(&self.master_key, &self.master_salt, KDF_LABEL_CIPHER, 0)
    }

    /// 派生会话认证密钥
    fn derive_auth_key(&self) -> Result<[u8; AES_128_KEY_LEN]> {
        srtp_kdf(&self.master_key, &self.master_salt, KDF_LABEL_AUTH, 0)
    }

    /// 派生会话盐值
    fn derive_session_salt(&self) -> Result<[u8; SRTP_SALT_LEN]> {
        let result: [u8; SRTP_SALT_LEN] =
            srtp_kdf(&self.master_key, &self.master_salt, KDF_LABEL_SALT, 0)?;
        Ok(result)
    }
}

/// SRTP 密钥派生函数（KDF）
///
/// 基于 AES-CM PRF，符合 RFC 3711 Section 4.3。
///
/// - `master_key`: 主密钥
/// - `master_salt`: 主盐值
/// - `label`: 密钥派生标签（0x00=加密, 0x01=认证, 0x02=盐值）
/// - `index`: 包索引（通常为 0）
fn srtp_kdf<const N: usize>(
    master_key: &[u8; SRTP_MASTER_KEY_LEN],
    master_salt: &[u8; SRTP_MASTER_SALT_LEN],
    label: u8,
    index: u64,
) -> Result<[u8; N]> {
    // 构造 KDF IV: salt XOR (label << 48 | index << 16)
    // label 在 byte 7（从 0 开始），index 占 bytes 8-13
    let mut iv = [0u8; 16];
    // 复制盐值到 iv[2..16]
    iv[2..16].copy_from_slice(master_salt);
    // XOR label 到 iv[7]
    iv[7] ^= label;
    // XOR index（6 字节，大端）到 iv[8..14]
    let index_bytes = index.to_be_bytes();
    iv[8] ^= index_bytes[2];
    iv[9] ^= index_bytes[3];
    iv[10] ^= index_bytes[4];
    iv[11] ^= index_bytes[5];
    iv[12] ^= index_bytes[6];
    iv[13] ^= index_bytes[7];

    // 使用 AES-CM 生成密钥流
    let mut cipher = Aes128Ctr::new(master_key.into(), &iv.into());
    let mut result = [0u8; N];
    cipher.apply_keystream(&mut result);
    Ok(result)
}

/// SRTP 加密上下文
///
/// 管理 SRTP 加密和解密状态，包括会话密钥、包索引跟踪、认证标签生成与验证、
/// 以及自动密钥轮换。
///
/// 加密算法：AES-128-CTR（Counter Mode）
/// 认证算法：HMAC-SHA1-80（截断为 10 字节）
///
/// # 线程安全
///
/// `SrtpContext` 不是线程安全的。在多线程场景下，应使用 `Mutex<SrtpContext>` 包装。
///
/// # 示例
///
/// ```rust
/// use network::crypto::{CryptoKeys, SrtpContext};
///
/// let keys = CryptoKeys::generate();
/// let ssrc = 0x12345678;
/// let mut ctx = SrtpContext::new(keys, ssrc).unwrap();
///
/// // 构造 RTP 数据包（12 字节头 + 载荷）
/// let mut rtp = vec![0x80u8, 0x60, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
///                    0x12, 0x34, 0x56, 0x78];
/// rtp.extend_from_slice(b"audio payload");
///
/// // 加密
/// let encrypted = ctx.protect(&rtp).unwrap();
/// assert_eq!(encrypted.len(), rtp.len() + 10); // + 认证标签
///
/// // 解密（需要相同密钥的另一个上下文）
/// let keys2 = CryptoKeys::generate(); // 实际应使用相同密钥
/// ```
pub struct SrtpContext {
    /// 会话加密密钥
    cipher_key: [u8; AES_128_KEY_LEN],
    /// 会话认证密钥
    auth_key: [u8; AES_128_KEY_LEN],
    /// 会话盐值
    session_salt: [u8; SRTP_SALT_LEN],
    /// 发送方向的 SSRC
    ssrc: u32,
    /// 当前包索引
    packet_index: u64,
    /// 原始密钥材料（用于密钥轮换）
    keys: CryptoKeys,
}

impl std::fmt::Debug for SrtpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SrtpContext")
            .field("ssrc", &self.ssrc)
            .field("packet_index", &self.packet_index)
            .finish()
    }
}

impl SrtpContext {
    /// 创建新的 SRTP 加密上下文
    ///
    /// 从主密钥材料派生出会话加密密钥、认证密钥和会话盐值。
    ///
    /// # Arguments
    ///
    /// * `keys` - 从 DTLS 握手派生的主密钥材料
    /// * `ssrc` - RTP 同步源标识符（Synchronization Source identifier）
    ///
    /// # Returns
    ///
    /// 初始化完成的 `SrtpContext` 实例，包索引从 0 开始。
    ///
    /// # Errors
    ///
    /// 如果密钥派生失败，返回 [`NetworkError::CryptoError`]。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::{CryptoKeys, SrtpContext};
    ///
    /// let keys = CryptoKeys::generate();
    /// let ctx = SrtpContext::new(keys, 0x12345678).unwrap();
    /// assert_eq!(ctx.ssrc(), 0x12345678);
    /// assert_eq!(ctx.packet_index(), 0);
    /// ```
    pub fn new(keys: CryptoKeys, ssrc: u32) -> Result<Self> {
        let cipher_key = keys.derive_cipher_key()?;
        let auth_key = keys.derive_auth_key()?;
        let session_salt = keys.derive_session_salt()?;

        Ok(Self {
            cipher_key,
            auth_key,
            session_salt,
            ssrc,
            packet_index: 0,
            keys,
        })
    }

    /// 加密 RTP 数据包（SRTP protect）
    ///
    /// 对 RTP 数据包执行加密和认证：
    /// 1. 使用 AES-128-CTR 加密载荷部分（保留 RTP 头明文）
    /// 2. 计算 HMAC-SHA1-80 认证标签
    /// 3. 在数据包末尾附加 10 字节认证标签
    ///
    /// # Arguments
    ///
    /// * `rtp_packet` - 包含 RTP 头（至少 12 字节）和明文载荷的完整 RTP 数据包
    ///
    /// # Returns
    ///
    /// 加密后的 SRTP 数据包：`RTP 头 + 密文载荷 + 认证标签（10 字节）`
    ///
    /// # Errors
    ///
    /// - [`NetworkError::CryptoError`] — RTP 数据包长度不足 12 字节
    /// - [`NetworkError::CryptoError`] — RTP 头长度解析错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::{CryptoKeys, SrtpContext};
    ///
    /// let keys = CryptoKeys::generate();
    /// let mut ctx = SrtpContext::new(keys, 0x12345678).unwrap();
    ///
    /// // 构造最小 RTP 数据包（12 字节头 + 载荷）
    /// let rtp = vec![0x80u8, 0x60, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ///                0x12, 0x34, 0x56, 0x78, 0xAA, 0xBB, 0xCC];
    /// let encrypted = ctx.protect(&rtp).unwrap();
    /// assert_eq!(encrypted.len(), rtp.len() + 10);
    /// ```
    pub fn protect(&mut self, rtp_packet: &[u8]) -> Result<Vec<u8>> {
        if rtp_packet.len() < 12 {
            return Err(NetworkError::CryptoError(
                "RTP 数据包至少需要 12 字节头".to_string(),
            ));
        }

        let header_len = self.rtp_header_len(rtp_packet);
        if rtp_packet.len() < header_len {
            return Err(NetworkError::CryptoError(
                "RTP 数据包头长度不正确".to_string(),
            ));
        }

        let mut output = rtp_packet.to_vec();

        // 加密载荷部分（AES-128-CM）
        let iv = self.build_iv();
        let mut cipher = Aes128Ctr::new((&self.cipher_key).into(), (&iv).into());
        cipher.apply_keystream(&mut output[header_len..]);

        // 计算认证标签（HMAC-SHA1 over header + encrypted payload）
        let auth_tag = self.compute_auth_tag(&output);

        // 附加认证标签
        output.extend_from_slice(&auth_tag);

        // 递增包索引
        self.packet_index = self.packet_index.wrapping_add(1);

        // 检查密钥轮换
        if self.packet_index.is_multiple_of(KEY_ROTATION_THRESHOLD) && self.packet_index > 0 {
            self.rotate_keys()?;
        }

        Ok(output)
    }

    /// 解密 SRTP 数据包（SRTP unprotect）
    ///
    /// 对 SRTP 数据包执行认证验证和解密：
    /// 1. 分离末尾 10 字节认证标签
    /// 2. 验证 HMAC-SHA1-80 认证标签（常量时间比较，防时序攻击）
    /// 3. 使用 AES-128-CTR 解密载荷部分
    ///
    /// # Arguments
    ///
    /// * `srtp_packet` - 包含 RTP 头、密文载荷和认证标签的 SRTP 数据包
    ///
    /// # Returns
    ///
    /// 解密后的 RTP 数据包（不含认证标签）
    ///
    /// # Errors
    ///
    /// - [`NetworkError::CryptoError`] — 数据包长度不足（< 12 + 10 字节）
    /// - [`NetworkError::CryptoError`] — 认证标签验证失败（数据被篡改或密钥不匹配）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::{CryptoKeys, SrtpContext};
    ///
    /// let keys = CryptoKeys::generate();
    /// let ssrc = 0x12345678;
    /// let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
    /// let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();
    ///
    /// let rtp = vec![0x80u8, 0x60, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ///                0x12, 0x34, 0x56, 0x78, 0xAA, 0xBB];
    /// let encrypted = encrypt_ctx.protect(&rtp).unwrap();
    /// let decrypted = decrypt_ctx.unprotect(&encrypted).unwrap();
    /// assert_eq!(decrypted, rtp);
    /// ```
    pub fn unprotect(&mut self, srtp_packet: &[u8]) -> Result<Vec<u8>> {
        if srtp_packet.len() < 12 + SRTP_AUTH_TAG_LEN {
            return Err(NetworkError::CryptoError("SRTP 数据包长度不足".to_string()));
        }

        // 分离认证标签
        let auth_tag_start = srtp_packet.len() - SRTP_AUTH_TAG_LEN;
        let received_tag = &srtp_packet[auth_tag_start..];
        let packet_without_tag = &srtp_packet[..auth_tag_start];

        // 验证认证标签
        let computed_tag = self.compute_auth_tag(packet_without_tag);
        if !constant_time_eq(&computed_tag, received_tag) {
            return Err(NetworkError::CryptoError(
                "SRTP 认证标签验证失败".to_string(),
            ));
        }

        let header_len = self.rtp_header_len(packet_without_tag);
        let mut output = packet_without_tag.to_vec();

        // 解密载荷部分
        let iv = self.build_iv();
        let mut cipher = Aes128Ctr::new((&self.cipher_key).into(), (&iv).into());
        cipher.apply_keystream(&mut output[header_len..]);

        // 递增包索引
        self.packet_index = self.packet_index.wrapping_add(1);

        // 检查密钥轮换
        if self.packet_index.is_multiple_of(KEY_ROTATION_THRESHOLD) && self.packet_index > 0 {
            self.rotate_keys()?;
        }

        Ok(output)
    }

    /// 获取当前包索引
    ///
    /// 包索引在每次 `protect` 或 `unprotect` 调用后自动递增。
    /// 用于 SRTP IV 构造和密钥轮换判断。
    ///
    /// # Returns
    ///
    /// 当前已处理的数据包总数。
    pub fn packet_index(&self) -> u64 {
        self.packet_index
    }

    /// 获取关联的 SSRC（同步源标识符）
    ///
    /// # Returns
    ///
    /// 创建时指定的 RTP SSRC 值。
    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }

    /// 构造 AES-CM 初始化向量（IV）
    ///
    /// IV 构造符合 RFC 3711 Section 4.1.1：
    /// IV = (session_salt << 16) XOR (SSRC << 64 | packet_index << 16)
    fn build_iv(&self) -> [u8; 16] {
        let mut iv = [0u8; 16];
        // 复制盐值到 iv[2..16]（14 字节）
        iv[2..16].copy_from_slice(&self.session_salt);

        // XOR SSRC 到 iv[4..8]
        let ssrc_bytes = self.ssrc.to_be_bytes();
        iv[4] ^= ssrc_bytes[0];
        iv[5] ^= ssrc_bytes[1];
        iv[6] ^= ssrc_bytes[2];
        iv[7] ^= ssrc_bytes[3];

        // XOR packet_index（48 位）到 iv[8..14]
        let idx_bytes = self.packet_index.to_be_bytes();
        iv[8] ^= idx_bytes[2];
        iv[9] ^= idx_bytes[3];
        iv[10] ^= idx_bytes[4];
        iv[11] ^= idx_bytes[5];
        iv[12] ^= idx_bytes[6];
        iv[13] ^= idx_bytes[7];

        iv
    }

    /// 计算认证标签
    ///
    /// HMAC-SHA1 over（RTP 头 + 加密后的载荷），截断为 80 位（10 字节）
    fn compute_auth_tag(&self, data: &[u8]) -> [u8; SRTP_AUTH_TAG_LEN] {
        let mut mac =
            HmacSha1::new_from_slice(&self.auth_key).expect("HMAC-SHA1 密钥长度应始终有效");
        mac.update(data);
        let full_tag = mac.finalize().into_bytes();

        // 截断为 80 位（10 字节）
        let mut tag = [0u8; SRTP_AUTH_TAG_LEN];
        tag.copy_from_slice(&full_tag[..SRTP_AUTH_TAG_LEN]);
        tag
    }

    /// 计算 RTP 头长度（含 CSRC 和扩展头）
    fn rtp_header_len(&self, packet: &[u8]) -> usize {
        if packet.is_empty() {
            return 0;
        }
        let cc = (packet[0] & 0x0F) as usize;
        let mut len = 12 + cc * 4;

        // 检查扩展头
        if packet.len() >= 12 && (packet[0] & 0x10) != 0 && packet.len() >= len + 4 {
            let ext_len = u16::from_be_bytes([packet[len + 2], packet[len + 3]]) as usize;
            len += 4 + ext_len * 4;
        }

        len.min(packet.len())
    }

    /// 密钥轮换
    ///
    /// 每 2^31 个包后重新派生会话密钥，防止密钥过度使用。
    fn rotate_keys(&mut self) -> Result<()> {
        let rotation_index = self.packet_index / KEY_ROTATION_THRESHOLD;
        self.cipher_key = srtp_kdf(
            &self.keys.master_key,
            &self.keys.master_salt,
            KDF_LABEL_CIPHER,
            rotation_index,
        )?;
        self.auth_key = srtp_kdf(
            &self.keys.master_key,
            &self.keys.master_salt,
            KDF_LABEL_AUTH,
            rotation_index,
        )?;
        let new_salt: [u8; SRTP_SALT_LEN] = srtp_kdf(
            &self.keys.master_key,
            &self.keys.master_salt,
            KDF_LABEL_SALT,
            rotation_index,
        )?;
        self.session_salt = new_salt;
        Ok(())
    }
}

/// 常量时间比较
///
/// 防止时序攻击的认证标签比较。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// DTLS 握手状态
///
/// 表示 DTLS 握手流程的当前阶段。
///
/// # 状态转换
///
/// ```text
/// Idle → WaitingClientHello → ServerHelloSent → Established
///                                          ↘ Failed
/// ```
///
/// # 示例
///
/// ```rust
/// use network::crypto::DtlsState;
///
/// assert_eq!(DtlsState::Idle, DtlsState::Idle);
/// assert_ne!(DtlsState::Idle, DtlsState::Established);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtlsState {
    /// 未开始，等待发起握手
    Idle,
    /// 等待 ClientHello 消息
    WaitingClientHello,
    /// 已发送 ServerHello 响应
    ServerHelloSent,
    /// 握手完成，会话密钥已就绪
    Established,
    /// 握手失败（超时或重试耗尽）
    Failed,
}

/// DTLS 握手配置
///
/// 控制 DTLS 握手的超时、重试和证书行为。
///
/// # 示例
///
/// ```rust
/// use network::crypto::DtlsConfig;
///
/// // 使用默认配置
/// let config = DtlsConfig::default();
/// assert_eq!(config.handshake_timeout_ms, 5000);
/// assert_eq!(config.max_retries, 3);
///
/// // 自定义配置
/// let config = DtlsConfig {
///     handshake_timeout_ms: 10000,
///     max_retries: 5,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DtlsConfig {
    /// 握手超时时间（毫秒）
    pub handshake_timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 自签名证书指纹（32 字节 SHA-256 哈希）
    pub cert_fingerprint: [u8; 32],
}

impl Default for DtlsConfig {
    fn default() -> Self {
        Self {
            handshake_timeout_ms: DTLS_HANDSHAKE_TIMEOUT_MS,
            max_retries: DTLS_MAX_RETRIES,
            cert_fingerprint: [0u8; 32],
        }
    }
}

/// DTLS 会话
///
/// 管理 DTLS 握手状态机和会话密钥派生。
///
/// 提供简化的 DTLS 握手流程，适用于局域网音频传输场景。
/// 握手完成后可获取派生的 [`CryptoKeys`]，用于创建 [`SrtpContext`]。
///
/// > **注意**：这是一个简化的 DTLS 实现，用于局域网音频传输场景。
/// > 生产环境应使用完整的 DTLS 1.2/1.3 实现（如 rustls）。
///
/// # 示例
///
/// ```rust
/// use network::crypto::{DtlsSession, DtlsConfig, DtlsState};
///
/// // 使用默认配置创建会话
/// let mut session = DtlsSession::with_default_config();
/// assert_eq!(session.state(), DtlsState::Idle);
///
/// // 开始握手
/// session.start_handshake().unwrap();
/// assert_eq!(session.state(), DtlsState::WaitingClientHello);
///
/// // 处理握手消息（简化的模拟流程）
/// let response = session.process_handshake(&[]).unwrap();
/// assert!(response.is_some());
/// assert_eq!(session.state(), DtlsState::ServerHelloSent);
/// assert!(session.keys().is_some());
///
/// // 完成握手
/// session.complete_handshake().unwrap();
/// assert_eq!(session.state(), DtlsState::Established);
/// ```
pub struct DtlsSession {
    /// 配置
    config: DtlsConfig,
    /// 当前状态
    state: DtlsState,
    /// 握手重试次数
    retry_count: u32,
    /// 派生的密钥材料（握手完成后可用）
    keys: Option<CryptoKeys>,
}

impl std::fmt::Debug for DtlsSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DtlsSession")
            .field("state", &self.state)
            .field("retry_count", &self.retry_count)
            .field("has_keys", &self.keys.is_some())
            .finish()
    }
}

impl DtlsSession {
    /// 使用指定配置创建新的 DTLS 会话
    ///
    /// # Arguments
    ///
    /// * `config` - DTLS 握手配置
    ///
    /// # Returns
    ///
    /// 初始状态为 [`DtlsState::Idle`] 的新会话。
    pub fn new(config: DtlsConfig) -> Self {
        Self {
            config,
            state: DtlsState::Idle,
            retry_count: 0,
            keys: None,
        }
    }

    /// 使用默认配置创建 DTLS 会话
    ///
    /// 等效于 `DtlsSession::new(DtlsConfig::default())`。
    ///
    /// # Returns
    ///
    /// 使用默认超时（5 秒）和重试次数（3 次）的新会话。
    pub fn with_default_config() -> Self {
        Self::new(DtlsConfig::default())
    }

    /// 获取当前握手状态
    ///
    /// # Returns
    ///
    /// 当前 [`DtlsState`] 枚举值。
    pub fn state(&self) -> DtlsState {
        self.state
    }

    /// 获取 DTLS 配置的引用
    ///
    /// # Returns
    ///
    /// 当前会话使用的 [`DtlsConfig`] 配置。
    pub fn config(&self) -> &DtlsConfig {
        &self.config
    }

    /// 开始握手（客户端模式）
    ///
    /// 将状态从 [`DtlsState::Idle`] 转换为 [`DtlsState::WaitingClientHello`]，
    /// 并重置重试计数器。
    ///
    /// # Errors
    ///
    /// 如果当前状态不是 `Idle`，返回 [`NetworkError::CryptoError`]。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::{DtlsSession, DtlsState};
    ///
    /// let mut session = DtlsSession::with_default_config();
    /// session.start_handshake().unwrap();
    /// assert_eq!(session.state(), DtlsState::WaitingClientHello);
    /// ```
    pub fn start_handshake(&mut self) -> Result<()> {
        if self.state != DtlsState::Idle {
            return Err(NetworkError::CryptoError(format!(
                "DTLS 非空闲状态无法开始握手: {:?}",
                self.state
            )));
        }
        self.state = DtlsState::WaitingClientHello;
        self.retry_count = 0;
        Ok(())
    }

    /// 处理握手消息（简化模拟流程）
    ///
    /// 模拟 DTLS 握手消息处理：
    /// - `WaitingClientHello` → 收到 ClientHello，生成密钥，发送 ServerHello
    /// - `ServerHelloSent` → 收到 Finished，握手完成
    /// - `Established` → 忽略（已建立）
    ///
    /// > **注意**：当前实现为简化版本，直接完成握手并生成密钥。
    /// > 真实实现应处理完整的 DTLS 握手消息。
    ///
    /// # Arguments
    ///
    /// * `_data` - 握手消息数据（当前实现忽略内容）
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<u8>))` — 需要发送的响应消息（ServerHello）
    /// - `Ok(None)` — 无需响应（握手完成或已建立）
    ///
    /// # Errors
    ///
    /// 如果当前状态不支持处理握手消息，返回 [`NetworkError::CryptoError`]。
    pub fn process_handshake(&mut self, _data: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.state {
            DtlsState::WaitingClientHello => {
                // 收到 ClientHello，发送 ServerHello
                self.state = DtlsState::ServerHelloSent;
                // 生成密钥材料
                let keys = CryptoKeys::generate();
                self.keys = Some(keys);
                // 返回 ServerHello 响应
                Ok(Some(self.build_server_hello()))
            }
            DtlsState::ServerHelloSent => {
                // 收到 Finished，握手完成
                self.state = DtlsState::Established;
                Ok(None)
            }
            DtlsState::Established => {
                // 已建立，忽略
                Ok(None)
            }
            _ => Err(NetworkError::CryptoError(format!(
                "DTLS 无效握手状态: {:?}",
                self.state
            ))),
        }
    }

    /// 完成握手（服务端响应后调用）
    ///
    /// 将状态从 [`DtlsState::ServerHelloSent`] 转换为 [`DtlsState::Established`]。
    ///
    /// # Errors
    ///
    /// 如果当前状态不是 `ServerHelloSent`，返回 [`NetworkError::CryptoError`]。
    pub fn complete_handshake(&mut self) -> Result<()> {
        if self.state != DtlsState::ServerHelloSent {
            return Err(NetworkError::CryptoError(format!(
                "DTLS 无法完成握手，当前状态: {:?}",
                self.state
            )));
        }
        self.state = DtlsState::Established;
        Ok(())
    }

    /// 重试握手
    ///
    /// 增加重试计数器并将状态重置为 [`DtlsState::WaitingClientHello`]。
    /// 如果已达到最大重试次数，将状态设为 [`DtlsState::Failed`]。
    ///
    /// # Returns
    ///
    /// - `true` — 还可以继续重试
    /// - `false` — 已达到最大重试次数，握手失败
    pub fn retry_handshake(&mut self) -> bool {
        if self.retry_count < self.config.max_retries {
            self.retry_count += 1;
            self.state = DtlsState::WaitingClientHello;
            true
        } else {
            self.state = DtlsState::Failed;
            false
        }
    }

    /// 获取派生的密钥材料
    ///
    /// 仅在握手到达 `ServerHelloSent` 或 `Established` 状态后可用。
    ///
    /// # Returns
    ///
    /// - `Some(&CryptoKeys)` — 握手已生成密钥
    /// - `None` — 握手尚未到达密钥生成阶段
    pub fn keys(&self) -> Option<&CryptoKeys> {
        self.keys.as_ref()
    }

    /// 获取当前重试次数
    ///
    /// # Returns
    ///
    /// 从 0 开始的握手重试计数。
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// 构建 ServerHello 响应（简化实现）
    fn build_server_hello(&self) -> Vec<u8> {
        // 简化的 ServerHello：魔数 + 证书指纹
        let mut msg = Vec::with_capacity(36);
        msg.extend_from_slice(b"DTLS-SB"); // 7 字节标识
        msg.push(0x01); // 版本
        msg.extend_from_slice(&self.config.cert_fingerprint);
        msg
    }

    /// 生成自签名证书指纹
    ///
    /// 使用密码学安全随机数生成 32 字节的证书指纹。
    /// 用于 DTLS 握手中的证书交换。
    ///
    /// > **注意**：当前实现使用随机字节作为简化证书指纹。
    /// > 生产环境应使用 X.509 证书的 SHA-256 指纹。
    ///
    /// # Returns
    ///
    /// 32 字节的随机证书指纹。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use network::crypto::DtlsSession;
    ///
    /// let cert1 = DtlsSession::generate_certificate();
    /// let cert2 = DtlsSession::generate_certificate();
    /// // 每次生成的指纹都不同
    /// assert_ne!(cert1, cert2);
    /// ```
    pub fn generate_certificate() -> [u8; 32] {
        use rand::RngCore;
        let mut fingerprint = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut fingerprint);
        fingerprint
    }
}

/// HKDF-SHA1 密钥派生
///
/// 从 DTLS 握手协商的主密钥和盐值派生 SRTP 会话密钥材料。
/// 使用 HKDF（HMAC-based Key Derivation Function, RFC 5869）。
///
/// # Arguments
///
/// * `master_secret` - DTLS 握手产生的主密钥
/// * `salt` - DTLS 握手协商的盐值
///
/// # Returns
///
/// 派生的 [`CryptoKeys`]，包含主密钥（16 字节）和主盐值（14 字节）。
///
/// # Errors
///
/// 如果 HKDF 扩展失败，返回 [`NetworkError::CryptoError`]。
///
/// # 示例
///
/// ```rust
/// use network::crypto::derive_session_keys;
///
/// let master_secret = [0x42u8; 32];
/// let salt = [0x69u8; 16];
/// let keys = derive_session_keys(&master_secret, &salt).unwrap();
///
/// // 相同输入产生相同输出
/// let keys2 = derive_session_keys(&master_secret, &salt).unwrap();
/// assert_eq!(keys.master_key, keys2.master_key);
/// ```
pub fn derive_session_keys(master_secret: &[u8], salt: &[u8]) -> Result<CryptoKeys> {
    use hkdf::Hkdf;

    let hk = Hkdf::<Sha1>::new(Some(salt), master_secret);

    let mut key_material = [0u8; SRTP_MASTER_KEY_LEN + SRTP_MASTER_SALT_LEN];
    hk.expand(b"SoundBridge SRTP", &mut key_material)
        .map_err(|e| NetworkError::CryptoError(format!("HKDF 密钥派生失败: {}", e)))?;

    let mut master_key = [0u8; SRTP_MASTER_KEY_LEN];
    let mut master_salt = [0u8; SRTP_MASTER_SALT_LEN];
    master_key.copy_from_slice(&key_material[..SRTP_MASTER_KEY_LEN]);
    master_salt.copy_from_slice(&key_material[SRTP_MASTER_KEY_LEN..]);

    Ok(CryptoKeys {
        master_key,
        master_salt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_crypto_keys_generate() {
        let keys = CryptoKeys::generate();
        assert_ne!(keys.master_key, [0u8; 16]);
        assert_ne!(keys.master_salt, [0u8; 14]);
    }

    #[test]
    fn test_crypto_keys_debug_redacted() {
        let keys = CryptoKeys::generate();
        let debug = format!("{:?}", keys);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains(&format!("{:?}", keys.master_key)));
    }

    #[test]
    fn test_srtp_encrypt_decrypt_roundtrip() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x12345678;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        let payload = b"Hello, SoundBridge audio frame!";
        let rtp = make_rtp_packet(ssrc, 1, payload);

        // 加密
        let encrypted = encrypt_ctx.protect(&rtp).unwrap();
        assert_ne!(&encrypted[12..12 + payload.len()], payload);
        assert_eq!(encrypted.len(), rtp.len() + SRTP_AUTH_TAG_LEN);

        // 解密
        let decrypted = decrypt_ctx.unprotect(&encrypted).unwrap();
        assert_eq!(&decrypted[12..], payload);
    }

    #[test]
    fn test_srtp_multiple_packets() {
        let keys = CryptoKeys::generate();
        let ssrc = 0xAABBCCDD;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        for seq in 0..10u16 {
            let payload = format!("frame_{}", seq);
            let rtp = make_rtp_packet(ssrc, seq, payload.as_bytes());

            let encrypted = encrypt_ctx.protect(&rtp).unwrap();
            let decrypted = decrypt_ctx.unprotect(&encrypted).unwrap();

            assert_eq!(&decrypted[12..], payload.as_bytes());
        }
    }

    #[test]
    fn test_srtp_wrong_key_rejected() {
        let keys1 = CryptoKeys::generate();
        let keys2 = CryptoKeys::generate();
        let ssrc = 0x11111111;
        let mut encrypt_ctx = SrtpContext::new(keys1, ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys2, ssrc).unwrap();

        let rtp = make_rtp_packet(ssrc, 1, b"secret audio");
        let encrypted = encrypt_ctx.protect(&rtp).unwrap();

        let result = decrypt_ctx.unprotect(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_srtp_auth_tag_tamper_detected() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x22222222;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        let rtp = make_rtp_packet(ssrc, 1, b"audio data");
        let mut encrypted = encrypt_ctx.protect(&rtp).unwrap();

        // 篡改认证标签
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;

        let result = decrypt_ctx.unprotect(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_srtp_payload_tamper_detected() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x33333333;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        let rtp = make_rtp_packet(ssrc, 1, b"audio data");
        let mut encrypted = encrypt_ctx.protect(&rtp).unwrap();

        // 篡改密文
        encrypted[12] ^= 0xFF;

        let result = decrypt_ctx.unprotect(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_srtp_short_packet_rejected() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x44444444;
        let mut ctx = SrtpContext::new(keys, ssrc).unwrap();

        // 太短的包
        let result = ctx.protect(&[0x80]);
        assert!(result.is_err());

        // 没有认证标签
        let result = ctx.unprotect(&[0x80; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_srtp_different_ssrc_different_output() {
        let keys = CryptoKeys::generate();
        let rtp = make_rtp_packet(0x11111111, 1, b"same payload");

        let mut ctx1 = SrtpContext::new(keys.clone(), 0x11111111).unwrap();
        let mut ctx2 = SrtpContext::new(keys, 0x22222222).unwrap();

        let enc1 = ctx1.protect(&rtp).unwrap();
        let enc2 = ctx2
            .protect(&make_rtp_packet(0x22222222, 1, b"same payload"))
            .unwrap();

        // 不同 SSRC 应产生不同密文
        assert_ne!(&enc1[12..22], &enc2[12..22]);
    }

    #[test]
    fn test_srtp_key_derivation() {
        let key = [0x01u8; 16];
        let salt = [0x02u8; 14];
        let keys = CryptoKeys::from_bytes(&key, &salt);

        let cipher_key = keys.derive_cipher_key().unwrap();
        let auth_key = keys.derive_auth_key().unwrap();
        let session_salt = keys.derive_session_salt().unwrap();

        // 不同标签应派生不同密钥
        assert_ne!(&cipher_key[..], &auth_key[..]);
        // 所有密钥应非零
        assert_ne!(cipher_key, [0u8; 16]);
        assert_ne!(auth_key, [0u8; 16]);
        assert_ne!(session_salt, [0u8; 14]);
    }

    #[test]
    fn test_srtp_kdf_deterministic() {
        let key = [0xABu8; 16];
        let salt = [0xCDu8; 14];
        let keys = CryptoKeys::from_bytes(&key, &salt);

        let k1 = keys.derive_cipher_key().unwrap();
        let k2 = keys.derive_cipher_key().unwrap();

        assert_eq!(k1, k2);
    }

    #[test]
    fn test_constant_time_eq() {
        let a = [0x01, 0x02, 0x03];
        let b = [0x01, 0x02, 0x03];
        let c = [0x01, 0x02, 0x04];

        assert!(constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, &c));
        assert!(!constant_time_eq(&a, &[0x01, 0x02])); // 长度不同
    }

    #[test]
    fn test_dtls_session_lifecycle() {
        let mut session = DtlsSession::with_default_config();

        assert_eq!(session.state(), DtlsState::Idle);

        session.start_handshake().unwrap();
        assert_eq!(session.state(), DtlsState::WaitingClientHello);

        let response = session.process_handshake(&[]).unwrap();
        assert!(response.is_some());
        assert_eq!(session.state(), DtlsState::ServerHelloSent);
        assert!(session.keys().is_some());

        session.complete_handshake().unwrap();
        assert_eq!(session.state(), DtlsState::Established);
    }

    #[test]
    fn test_dtls_retry() {
        let config = DtlsConfig {
            max_retries: 2,
            ..Default::default()
        };
        let mut session = DtlsSession::new(config);

        session.start_handshake().unwrap();

        // 模拟失败后重试
        session.state = DtlsState::Failed;
        // 手动重置到可重试状态
        session.state = DtlsState::WaitingClientHello;

        assert!(session.retry_handshake());
        assert_eq!(session.retry_count(), 1);

        assert!(session.retry_handshake());
        assert_eq!(session.retry_count(), 2);

        // 第三次应失败
        assert!(!session.retry_handshake());
        assert_eq!(session.state(), DtlsState::Failed);
    }

    #[test]
    fn test_dtls_max_retries() {
        let config = DtlsConfig {
            max_retries: 1,
            ..Default::default()
        };
        let mut session = DtlsSession::new(config);

        session.start_handshake().unwrap();

        // 第一次重试
        assert!(session.retry_handshake());
        assert_eq!(session.state(), DtlsState::WaitingClientHello);

        // 第二次重试应失败
        assert!(!session.retry_handshake());
        assert_eq!(session.state(), DtlsState::Failed);
    }

    #[test]
    fn test_dtls_invalid_state_handshake() {
        let mut session = DtlsSession::with_default_config();

        // Idle 状态下不能完成握手
        let result = session.complete_handshake();
        assert!(result.is_err());
    }

    #[test]
    fn test_dtls_start_handshake_twice() {
        let mut session = DtlsSession::with_default_config();

        session.start_handshake().unwrap();

        // 非 Idle 状态不能再次开始
        let result = session.start_handshake();
        assert!(result.is_err());
    }

    #[test]
    fn test_dtls_generate_certificate() {
        let cert1 = DtlsSession::generate_certificate();
        let cert2 = DtlsSession::generate_certificate();

        // 每次生成应不同
        assert_ne!(cert1, cert2);
    }

    #[test]
    fn test_derive_session_keys() {
        let master_secret = [0x42u8; 32];
        let salt = [0x69u8; 16];

        let keys = derive_session_keys(&master_secret, &salt).unwrap();

        assert_ne!(keys.master_key, [0u8; 16]);
        assert_ne!(keys.master_salt, [0u8; 14]);
    }

    #[test]
    fn test_derive_session_keys_deterministic() {
        let master_secret = [0x55u8; 32];
        let salt = [0xAAu8; 16];

        let keys1 = derive_session_keys(&master_secret, &salt).unwrap();
        let keys2 = derive_session_keys(&master_secret, &salt).unwrap();

        assert_eq!(keys1.master_key, keys2.master_key);
        assert_eq!(keys1.master_salt, keys2.master_salt);
    }

    #[test]
    fn test_derive_session_keys_different_input() {
        let keys1 = derive_session_keys(&[0x01u8; 32], &[0x02u8; 16]).unwrap();
        let keys2 = derive_session_keys(&[0x03u8; 32], &[0x04u8; 16]).unwrap();

        assert_ne!(keys1.master_key, keys2.master_key);
    }

    #[test]
    fn test_srtp_packet_index_tracking() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x55555555;
        let mut ctx = SrtpContext::new(keys, ssrc).unwrap();

        assert_eq!(ctx.packet_index(), 0);

        let rtp = make_rtp_packet(ssrc, 1, b"frame");
        ctx.protect(&rtp).unwrap();
        assert_eq!(ctx.packet_index(), 1);

        ctx.protect(&rtp).unwrap();
        assert_eq!(ctx.packet_index(), 2);
    }

    #[test]
    fn test_dtls_config_default() {
        let config = DtlsConfig::default();
        assert_eq!(config.handshake_timeout_ms, 5000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_dtls_state_debug() {
        assert_eq!(format!("{:?}", DtlsState::Idle), "Idle");
        assert_eq!(format!("{:?}", DtlsState::Established), "Established");
    }

    #[test]
    fn test_dtls_session_debug() {
        let session = DtlsSession::with_default_config();
        let debug = format!("{:?}", session);
        assert!(debug.contains("DtlsSession"));
        assert!(debug.contains("Idle"));
    }

    #[test]
    fn test_srtp_context_debug() {
        let keys = CryptoKeys::generate();
        let ctx = SrtpContext::new(keys, 0x12345678).unwrap();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("SrtpContext"));
        assert!(debug.contains("ssrc"));
    }

    #[test]
    fn test_rtp_header_len_basic() {
        let keys = CryptoKeys::generate();
        let ctx = SrtpContext::new(keys, 0).unwrap();

        // 基本 RTP 头（V=2, CC=0, X=0）
        let pkt = [0x80u8; 12];
        assert_eq!(ctx.rtp_header_len(&pkt), 12);
    }

    #[test]
    fn test_rtp_header_len_with_csrc() {
        let keys = CryptoKeys::generate();
        let ctx = SrtpContext::new(keys, 0).unwrap();

        // CC=2，头长度 = 12 + 2*4 = 20
        let mut pkt = vec![0x82u8; 20];
        pkt[0] = 0x82; // V=2, CC=2
        assert_eq!(ctx.rtp_header_len(&pkt), 20);
    }

    #[test]
    fn test_srtp_empty_payload() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x66666666;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        let rtp = make_rtp_packet(ssrc, 1, &[]);
        let encrypted = encrypt_ctx.protect(&rtp).unwrap();
        let decrypted = decrypt_ctx.unprotect(&encrypted).unwrap();

        assert_eq!(decrypted.len(), 12);
    }

    #[test]
    fn test_srtp_large_payload() {
        let keys = CryptoKeys::generate();
        let ssrc = 0x77777777;
        let mut encrypt_ctx = SrtpContext::new(keys.clone(), ssrc).unwrap();
        let mut decrypt_ctx = SrtpContext::new(keys, ssrc).unwrap();

        // 模拟 960 samples * 2 bytes = 1920 字节音频帧
        let payload = vec![0xABu8; 1920];
        let rtp = make_rtp_packet(ssrc, 1, &payload);

        let encrypted = encrypt_ctx.protect(&rtp).unwrap();
        let decrypted = decrypt_ctx.unprotect(&encrypted).unwrap();

        assert_eq!(&decrypted[12..], &payload[..]);
    }
}
