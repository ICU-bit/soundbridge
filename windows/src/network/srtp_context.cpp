#include "srtp_context.h"

// Windows CNG APIs
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <bcrypt.h>

#include <algorithm>
#include <cstring>
#include <random>

#pragma comment(lib, "bcrypt.lib")

namespace soundbridge {

// ──────────────────────────────── AES-128-ECB 辅助 ────────────────────────────────

/// 使用 BCrypt 进行 AES-128-ECB 单块加密
static bool aes_128_ecb_encrypt(const uint8_t key[16], const uint8_t in[16], uint8_t out[16]) {
    BCRYPT_ALG_HANDLE alg = nullptr;
    BCRYPT_KEY_HANDLE key_handle = nullptr;
    bool success = false;

    if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_AES_ALGORITHM, nullptr, 0) != 0) {
        return false;
    }

    if (BCryptSetProperty(alg, BCRYPT_CHAINING_MODE,
                          reinterpret_cast<PUCHAR>(const_cast<wchar_t*>(BCRYPT_CHAIN_MODE_ECB)),
                          sizeof(BCRYPT_CHAIN_MODE_ECB), 0) != 0) {
        BCryptCloseAlgorithmProvider(alg, 0);
        return false;
    }

    if (BCryptGenerateSymmetricKey(alg, &key_handle, nullptr, 0,
                                   const_cast<PUCHAR>(key), 16, 0) == 0) {
        ULONG out_size = 0;
        if (BCryptEncrypt(key_handle, const_cast<PUCHAR>(in), 16,
                          nullptr, nullptr, 0, out, 16, &out_size, 0) == 0) {
            success = true;
        }
        BCryptDestroyKey(key_handle);
    }

    BCryptCloseAlgorithmProvider(alg, 0);
    return success;
}

/// AES-128-CTR 加密/解密（ECB + XOR 手动实现 CTR 模式）
static void aes_128_ctr_crypt(const uint8_t key[16], uint8_t iv[16],
                               const uint8_t* input, uint8_t* output, size_t len) {
    uint8_t counter_block[16];
    uint8_t keystream[16];

    // 复制 IV 作为初始计数器
    std::memcpy(counter_block, iv, 16);

    size_t offset = 0;
    while (offset < len) {
        // 加密计数器块生成密钥流
        aes_128_ecb_encrypt(key, counter_block, keystream);

        // XOR 密钥流与输入
        size_t block_len = std::min<size_t>(16, len - offset);
        for (size_t i = 0; i < block_len; ++i) {
            output[offset + i] = input[offset + i] ^ keystream[i];
        }

        offset += block_len;

        // 递增计数器（大端序，从最后一个字节开始进位）
        for (int i = 15; i >= 0; --i) {
            if (++counter_block[i] != 0) break;
        }
    }
}

/// 使用 BCrypt 计算 HMAC-SHA1
static bool hmac_sha1(const uint8_t* key, size_t key_len,
                       const uint8_t* data, size_t data_len,
                       uint8_t out[20]) {
    BCRYPT_ALG_HANDLE alg = nullptr;
    BCRYPT_HASH_HANDLE hash = nullptr;
    bool success = false;

    if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA1_ALGORITHM,
                                    nullptr, BCRYPT_ALG_HANDLE_HMAC_FLAG) != 0) {
        return false;
    }

    if (BCryptCreateHash(alg, &hash, nullptr, 0,
                         const_cast<PUCHAR>(key), static_cast<ULONG>(key_len), 0) == 0) {
        if (BCryptHashData(hash, const_cast<PUCHAR>(data),
                           static_cast<ULONG>(data_len), 0) == 0) {
            if (BCryptFinishHash(hash, out, 20, 0) == 0) {
                success = true;
            }
        }
        BCryptDestroyHash(hash);
    }

    BCryptCloseAlgorithmProvider(alg, 0);
    return success;
}

// ──────────────────────────────── CryptoKeys ────────────────────────────────

CryptoKeys CryptoKeys::generate() {
    CryptoKeys keys;
    std::random_device rd;
    for (auto& b : keys.master_key) b = static_cast<uint8_t>(rd());
    for (auto& b : keys.master_salt) b = static_cast<uint8_t>(rd());
    return keys;
}

CryptoKeys CryptoKeys::from_bytes(
    const uint8_t(&key)[SRTP_MASTER_KEY_LEN],
    const uint8_t(&salt)[SRTP_MASTER_SALT_LEN])
{
    CryptoKeys keys;
    std::memcpy(keys.master_key.data(), key, SRTP_MASTER_KEY_LEN);
    std::memcpy(keys.master_salt.data(), salt, SRTP_MASTER_SALT_LEN);
    return keys;
}

template<size_t N>
std::array<uint8_t, N> CryptoKeys::srtp_kdf(uint8_t label, uint64_t index) const {
    // 构造 KDF IV: salt XOR (label << 48 | index << 16)
    uint8_t iv[16] = {};
    // 复制盐值到 iv[2..16]
    std::memcpy(iv + 2, master_salt.data(), SRTP_MASTER_SALT_LEN);
    // XOR label 到 iv[7]
    iv[7] ^= label;
    // XOR index（6 字节，大端）到 iv[8..14]
    iv[8] ^= static_cast<uint8_t>(index >> 40);
    iv[9] ^= static_cast<uint8_t>(index >> 32);
    iv[10] ^= static_cast<uint8_t>(index >> 24);
    iv[11] ^= static_cast<uint8_t>(index >> 16);
    iv[12] ^= static_cast<uint8_t>(index >> 8);
    iv[13] ^= static_cast<uint8_t>(index);

    // 使用 AES-CTR 生成密钥流
    std::array<uint8_t, N> result{};
    uint8_t zero[N] = {};
    aes_128_ctr_crypt(master_key.data(), iv, zero, result.data(), N);
    return result;
}

std::array<uint8_t, AES_128_KEY_LEN> CryptoKeys::derive_cipher_key() const {
    return srtp_kdf<AES_128_KEY_LEN>(KDF_LABEL_CIPHER, 0);
}

std::array<uint8_t, AES_128_KEY_LEN> CryptoKeys::derive_auth_key() const {
    return srtp_kdf<AES_128_KEY_LEN>(KDF_LABEL_AUTH, 0);
}

std::array<uint8_t, SRTP_SALT_LEN> CryptoKeys::derive_session_salt() const {
    return srtp_kdf<SRTP_SALT_LEN>(KDF_LABEL_SALT, 0);
}

// ──────────────────────────────── SrtpContext ────────────────────────────────

SrtpContext::~SrtpContext() {
    // 清零敏感密钥材料
    std::memset(cipher_key_.data(), 0, cipher_key_.size());
    std::memset(auth_key_.data(), 0, auth_key_.size());
    std::memset(session_salt_.data(), 0, session_salt_.size());
}

bool SrtpContext::initialize(const CryptoKeys& keys, uint32_t ssrc) {
    keys_ = keys;
    ssrc_ = ssrc;
    packet_index_ = 0;

    // 派生会话密钥
    cipher_key_ = keys.derive_cipher_key();
    auth_key_ = keys.derive_auth_key();
    session_salt_ = keys.derive_session_salt();

    initialized_ = true;
    return true;
}

bool SrtpContext::protect(const uint8_t* rtp_packet, size_t rtp_len,
                          std::vector<uint8_t>& output) {
    if (!initialized_ || !rtp_packet || rtp_len < 12) {
        return false;
    }

    // 计算 RTP 头长度
    size_t header_len = 12;
    if (rtp_len >= 12) {
        uint8_t cc = rtp_packet[0] & 0x0F;
        header_len = 12 + cc * 4;
        // 检查扩展头
        if ((rtp_packet[0] & 0x10) != 0 && rtp_len >= header_len + 4) {
            uint16_t ext_len = (static_cast<uint16_t>(rtp_packet[header_len + 2]) << 8) |
                               rtp_packet[header_len + 3];
            header_len += 4 + ext_len * 4;
        }
        header_len = std::min(header_len, rtp_len);
    }

    // 准备输出：RTP 头 + 密文载荷 + 认证标签
    output.resize(rtp_len + SRTP_AUTH_TAG_LEN);

    // 复制 RTP 头
    std::memcpy(output.data(), rtp_packet, header_len);

    // AES-128-CTR 加密载荷
    uint8_t iv[16];
    if (!build_iv(iv)) return false;

    size_t payload_len = rtp_len - header_len;
    if (payload_len > 0) {
        aes_128_ctr_crypt(cipher_key_.data(), iv,
                          rtp_packet + header_len,
                          output.data() + header_len,
                          payload_len);
    }

    // 计算认证标签（HMAC-SHA1 over header + encrypted payload）
    if (!compute_auth_tag(output.data(), rtp_len,
                          output.data() + rtp_len)) {
        return false;
    }

    // 递增包索引
    packet_index_++;

    // 检查密钥轮换
    if (packet_index_ % KEY_ROTATION_THRESHOLD == 0 && packet_index_ > 0) {
        if (!rotate_keys()) return false;
    }

    return true;
}

bool SrtpContext::unprotect(const uint8_t* srtp_packet, size_t srtp_len,
                            std::vector<uint8_t>& output) {
    if (!initialized_ || !srtp_packet || srtp_len < 12 + SRTP_AUTH_TAG_LEN) {
        return false;
    }

    // 分离认证标签
    size_t auth_tag_start = srtp_len - SRTP_AUTH_TAG_LEN;
    const uint8_t* received_tag = srtp_packet + auth_tag_start;
    size_t packet_len = auth_tag_start;

    // 验证认证标签
    uint8_t computed_tag[SRTP_AUTH_TAG_LEN];
    if (!compute_auth_tag(srtp_packet, packet_len, computed_tag)) {
        return false;
    }

    if (!constant_time_eq(computed_tag, received_tag, SRTP_AUTH_TAG_LEN)) {
        return false;
    }

    // 计算 RTP 头长度
    size_t header_len = 12;
    if (packet_len >= 12) {
        uint8_t cc = srtp_packet[0] & 0x0F;
        header_len = 12 + cc * 4;
        if ((srtp_packet[0] & 0x10) != 0 && packet_len >= header_len + 4) {
            uint16_t ext_len = (static_cast<uint16_t>(srtp_packet[header_len + 2]) << 8) |
                               srtp_packet[header_len + 3];
            header_len += 4 + ext_len * 4;
        }
        header_len = std::min(header_len, packet_len);
    }

    // 准备输出
    output.resize(packet_len);

    // 复制 RTP 头
    std::memcpy(output.data(), srtp_packet, header_len);

    // AES-128-CTR 解密载荷
    uint8_t iv[16];
    if (!build_iv(iv)) return false;

    size_t payload_len = packet_len - header_len;
    if (payload_len > 0) {
        aes_128_ctr_crypt(cipher_key_.data(), iv,
                          srtp_packet + header_len,
                          output.data() + header_len,
                          payload_len);
    }

    // 递增包索引
    packet_index_++;

    // 检查密钥轮换
    if (packet_index_ % KEY_ROTATION_THRESHOLD == 0 && packet_index_ > 0) {
        if (!rotate_keys()) return false;
    }

    return true;
}

bool SrtpContext::build_iv(uint8_t iv[16]) const {
    std::memset(iv, 0, 16);

    // 复制盐值到 iv[2..16]（14 字节）
    std::memcpy(iv + 2, session_salt_.data(), SRTP_SALT_LEN);

    // XOR SSRC 到 iv[4..8]
    iv[4] ^= static_cast<uint8_t>(ssrc_ >> 24);
    iv[5] ^= static_cast<uint8_t>(ssrc_ >> 16);
    iv[6] ^= static_cast<uint8_t>(ssrc_ >> 8);
    iv[7] ^= static_cast<uint8_t>(ssrc_);

    // XOR packet_index（48 位）到 iv[8..14]
    iv[8] ^= static_cast<uint8_t>(packet_index_ >> 40);
    iv[9] ^= static_cast<uint8_t>(packet_index_ >> 32);
    iv[10] ^= static_cast<uint8_t>(packet_index_ >> 24);
    iv[11] ^= static_cast<uint8_t>(packet_index_ >> 16);
    iv[12] ^= static_cast<uint8_t>(packet_index_ >> 8);
    iv[13] ^= static_cast<uint8_t>(packet_index_);

    return true;
}

bool SrtpContext::compute_auth_tag(const uint8_t* data, size_t len,
                                   uint8_t tag[SRTP_AUTH_TAG_LEN]) const {
    if (!data) return false;

    uint8_t full_hash[20];
    if (!hmac_sha1(auth_key_.data(), auth_key_.size(), data, len, full_hash)) {
        return false;
    }

    // 截断为 80 位（10 字节）
    std::memcpy(tag, full_hash, SRTP_AUTH_TAG_LEN);
    return true;
}

bool SrtpContext::rotate_keys() {
    uint64_t rotation_index = packet_index_ / KEY_ROTATION_THRESHOLD;

    cipher_key_ = keys_.srtp_kdf<AES_128_KEY_LEN>(KDF_LABEL_CIPHER, rotation_index);
    auth_key_ = keys_.srtp_kdf<AES_128_KEY_LEN>(KDF_LABEL_AUTH, rotation_index);
    session_salt_ = keys_.srtp_kdf<SRTP_SALT_LEN>(KDF_LABEL_SALT, rotation_index);

    return true;
}

// ──────────────────────────────── 工具函数 ────────────────────────────────

bool constant_time_eq(const uint8_t* a, const uint8_t* b, size_t len) {
    uint8_t diff = 0;
    for (size_t i = 0; i < len; ++i) {
        diff |= a[i] ^ b[i];
    }
    return diff == 0;
}

} // namespace soundbridge
