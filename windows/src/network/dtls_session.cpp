#include "dtls_session.h"

#include <cstring>
#include <random>

namespace soundbridge {

DtlsSession::DtlsSession() = default;
DtlsSession::~DtlsSession() = default;

bool DtlsSession::initialize(const DtlsConfig& config) {
    config_ = config;
    state_ = DtlsState::Idle;
    retry_count_ = 0;
    keys_.reset();
    return true;
}

bool DtlsSession::start_handshake() {
    if (state_ != DtlsState::Idle) {
        return false;
    }
    state_ = DtlsState::WaitingClientHello;
    retry_count_ = 0;
    return true;
}

bool DtlsSession::process_handshake(const uint8_t* data, size_t len,
                                     std::vector<uint8_t>& response) {
    (void)data;
    (void)len;

    switch (state_) {
        case DtlsState::WaitingClientHello:
            // 收到 ClientHello，发送 ServerHello
            state_ = DtlsState::ServerHelloSent;
            // 生成密钥材料
            keys_ = CryptoKeys::generate();
            // 返回 ServerHello 响应
            response = build_server_hello();
            return true;

        case DtlsState::ServerHelloSent:
            // 收到 Finished，握手完成
            state_ = DtlsState::Established;
            response.clear();
            return true;

        case DtlsState::Established:
            // 已建立，忽略
            response.clear();
            return true;

        default:
            return false;
    }
}

bool DtlsSession::complete_handshake() {
    if (state_ != DtlsState::ServerHelloSent) {
        return false;
    }
    state_ = DtlsState::Established;
    return true;
}

bool DtlsSession::retry_handshake() {
    if (retry_count_ < config_.max_retries) {
        retry_count_++;
        state_ = DtlsState::WaitingClientHello;
        return true;
    }
    state_ = DtlsState::Failed;
    return false;
}

std::array<uint8_t, 32> DtlsSession::generate_certificate() {
    std::array<uint8_t, 32> fingerprint{};
    std::random_device rd;
    for (auto& b : fingerprint) {
        b = static_cast<uint8_t>(rd());
    }
    return fingerprint;
}

std::vector<uint8_t> DtlsSession::build_server_hello() const {
    // 简化的 ServerHello：魔数 + 版本 + 证书指纹
    std::vector<uint8_t> msg;
    msg.reserve(7 + 1 + 32);

    // 标识 "DTLS-SB"
    msg.push_back('D');
    msg.push_back('T');
    msg.push_back('L');
    msg.push_back('S');
    msg.push_back('-');
    msg.push_back('S');
    msg.push_back('B');

    // 版本
    msg.push_back(0x01);

    // 证书指纹
    msg.insert(msg.end(), config_.cert_fingerprint.begin(), config_.cert_fingerprint.end());

    return msg;
}

CryptoKeys derive_session_keys(const uint8_t* master_secret, size_t secret_len,
                                const uint8_t* salt, size_t salt_len) {
    // 简化的 HKDF：使用 master_secret 和 salt 派生密钥
    // 实际应使用 HKDF-SHA1 (RFC 5869)
    CryptoKeys keys;

    // 简单的密钥派生：XOR 折叠
    std::memset(keys.master_key.data(), 0, SRTP_MASTER_KEY_LEN);
    std::memset(keys.master_salt.data(), 0, SRTP_MASTER_SALT_LEN);

    // 从 master_secret 派生 master_key
    for (size_t i = 0; i < secret_len; ++i) {
        keys.master_key[i % SRTP_MASTER_KEY_LEN] ^= master_secret[i];
    }

    // 从 salt 派生 master_salt
    for (size_t i = 0; i < salt_len; ++i) {
        keys.master_salt[i % SRTP_MASTER_SALT_LEN] ^= salt[i];
    }

    return keys;
}

} // namespace soundbridge
