#pragma once

#include <cstdint>
#include <cstddef>
#include <array>
#include <vector>

namespace soundbridge {

// SRTP 常量（与 Rust 实现一致）
constexpr size_t SRTP_AUTH_TAG_LEN = 10;      // HMAC-SHA1-80 = 80 位 = 10 字节
constexpr size_t AES_128_KEY_LEN = 16;        // AES-128 密钥长度
constexpr size_t SRTP_SALT_LEN = 14;          // SRTP 会话盐值长度
constexpr size_t SRTP_MASTER_KEY_LEN = 16;    // SRTP 主密钥长度
constexpr size_t SRTP_MASTER_SALT_LEN = 14;   // SRTP 主盐值长度
constexpr uint64_t KEY_ROTATION_THRESHOLD = 1ULL << 31;  // 密钥轮换阈值

// KDF 标签
constexpr uint8_t KDF_LABEL_CIPHER = 0x00;
constexpr uint8_t KDF_LABEL_AUTH = 0x01;
constexpr uint8_t KDF_LABEL_SALT = 0x02;

/// SRTP 密钥材料
struct CryptoKeys {
    std::array<uint8_t, SRTP_MASTER_KEY_LEN> master_key{};
    std::array<uint8_t, SRTP_MASTER_SALT_LEN> master_salt{};

    /// 生成随机密钥
    static CryptoKeys generate();

    /// 从字节构造
    static CryptoKeys from_bytes(
        const uint8_t(&key)[SRTP_MASTER_KEY_LEN],
        const uint8_t(&salt)[SRTP_MASTER_SALT_LEN]
    );

    /// 派生会话加密密钥
    std::array<uint8_t, AES_128_KEY_LEN> derive_cipher_key() const;

    /// 派生会话认证密钥
    std::array<uint8_t, AES_128_KEY_LEN> derive_auth_key() const;

    /// 派生会话盐值
    std::array<uint8_t, SRTP_SALT_LEN> derive_session_salt() const;

    /// SRTP 密钥派生函数（KDF）
    template<size_t N>
    std::array<uint8_t, N> srtp_kdf(uint8_t label, uint64_t index) const;
};

/// SRTP 加密上下文
///
/// 实现 AES-128-CM 加密 + HMAC-SHA1-80 认证
class SrtpContext {
public:
    SrtpContext() = default;
    ~SrtpContext();

    SrtpContext(const SrtpContext&) = delete;
    SrtpContext& operator=(const SrtpContext&) = delete;

    /// 从密钥材料初始化
    bool initialize(const CryptoKeys& keys, uint32_t ssrc);

    /// 加密 RTP 数据包（protect）
    /// 输入: RTP 头 + 明文载荷
    /// 输出: RTP 头 + 密文载荷 + 认证标签(10字节)
    bool protect(const uint8_t* rtp_packet, size_t rtp_len,
                 std::vector<uint8_t>& output);

    /// 解密 SRTP 数据包（unprotect）
    /// 输入: RTP 头 + 密文载荷 + 认证标签(10字节)
    /// 输出: RTP 头 + 明文载荷
    bool unprotect(const uint8_t* srtp_packet, size_t srtp_len,
                   std::vector<uint8_t>& output);

    bool is_initialized() const { return initialized_; }
    uint64_t packet_index() const { return packet_index_; }
    uint32_t ssrc() const { return ssrc_; }

private:
    bool build_iv(uint8_t iv[16]) const;
    bool compute_auth_tag(const uint8_t* data, size_t len,
                          uint8_t tag[SRTP_AUTH_TAG_LEN]) const;
    bool rotate_keys();

    // Windows CNG 句柄
    void* aes_handle_ = nullptr;    // BCRYPT_ALG_HANDLE for AES
    void* hmac_handle_ = nullptr;   // BCRYPT_ALG_HANDLE for HMAC-SHA1

    // 会话密钥
    std::array<uint8_t, AES_128_KEY_LEN> cipher_key_{};
    std::array<uint8_t, AES_128_KEY_LEN> auth_key_{};
    std::array<uint8_t, SRTP_SALT_LEN> session_salt_{};

    // 原始密钥材料（用于轮换）
    CryptoKeys keys_;

    uint32_t ssrc_ = 0;
    uint64_t packet_index_ = 0;
    bool initialized_ = false;
};

/// 常量时间比较（防时序攻击）
bool constant_time_eq(const uint8_t* a, const uint8_t* b, size_t len);

} // namespace soundbridge
