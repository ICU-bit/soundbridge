#include "dtls_session.h"

#include <windows.h>
#include <bcrypt.h>
#include <cstring>
#include <random>

#pragma comment(lib, "bcrypt.lib")

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
    if (!data || len < 8) {
        return false;
    }

    // Validate DTLS handshake header: must start with "DTLS-SB" marker
    static constexpr uint8_t kDtlsMagic[] = {'D', 'T', 'L', 'S', '-', 'S', 'B'};
    if (std::memcmp(data, kDtlsMagic, sizeof(kDtlsMagic)) != 0) {
        return false;
    }

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

namespace {

constexpr size_t SHA1_HASH_LEN = 20;

bool bcrypt_hmac_sha1(const uint8_t* key, size_t key_len,
                      const uint8_t* data, size_t data_len,
                      uint8_t out[SHA1_HASH_LEN]) {
    BCRYPT_ALG_HANDLE hAlg = nullptr;
    BCRYPT_HASH_HANDLE hHash = nullptr;
    bool ok = false;

    if (FAILED(BCryptOpenAlgorithmProvider(&hAlg, BCRYPT_SHA1_ALGORITHM,
                                           nullptr, BCRYPT_ALG_HANDLE_HMAC_FLAG))) {
        return false;
    }
    if (FAILED(BCryptCreateHash(hAlg, &hHash, nullptr, 0,
                                const_cast<PUCHAR>(key),
                                static_cast<ULONG>(key_len), 0))) {
        BCryptCloseAlgorithmProvider(hAlg, 0);
        return false;
    }
    if (FAILED(BCryptHashData(hHash, const_cast<PUCHAR>(data),
                              static_cast<ULONG>(data_len), 0))) {
        goto cleanup;
    }
    if (FAILED(BCryptFinishHash(hHash, out, SHA1_HASH_LEN, 0))) {
        goto cleanup;
    }
    ok = true;
cleanup:
    if (hHash) BCryptDestroyHash(hHash);
    BCryptCloseAlgorithmProvider(hAlg, 0);
    return ok;
}

} // anonymous namespace

CryptoKeys derive_session_keys(const uint8_t* master_secret, size_t secret_len,
                                const uint8_t* salt, size_t salt_len) {
    CryptoKeys keys;
    std::memset(keys.master_key.data(), 0, SRTP_MASTER_KEY_LEN);
    std::memset(keys.master_salt.data(), 0, SRTP_MASTER_SALT_LEN);

    // HKDF-SHA1 (RFC 5869)
    // Step 1: Extract — PRK = HMAC-SHA1(salt, IKM)
    uint8_t prk[SHA1_HASH_LEN];
    const uint8_t* extract_salt = salt;
    size_t extract_salt_len = salt_len;
    uint8_t default_salt[SHA1_HASH_LEN]{};

    if (salt_len == 0 || salt == nullptr) {
        std::memset(default_salt, 0, SHA1_HASH_LEN);
        extract_salt = default_salt;
        extract_salt_len = SHA1_HASH_LEN;
    }

    if (!bcrypt_hmac_sha1(extract_salt, extract_salt_len,
                          master_secret, secret_len, prk)) {
        return keys;
    }

    // Step 2: Expand — derive key and salt using info labels
    // info = "SRTP key" || 0x01 for master_key (16 bytes)
    const uint8_t key_info[] = {'S', 'R', 'T', 'P', ' ', 'k', 'e', 'y', 0x01};
    uint8_t okm_key[SHA1_HASH_LEN];
    if (!bcrypt_hmac_sha1(prk, SHA1_HASH_LEN,
                          key_info, sizeof(key_info), okm_key)) {
        return keys;
    }
    std::memcpy(keys.master_key.data(), okm_key, SRTP_MASTER_KEY_LEN);

    // info = "SRTP salt" || 0x02 for master_salt (14 bytes)
    const uint8_t salt_info[] = {'S', 'R', 'T', 'P', ' ', 's', 'a', 'l', 't', 0x02};
    uint8_t okm_salt[SHA1_HASH_LEN];
    if (!bcrypt_hmac_sha1(prk, SHA1_HASH_LEN,
                          salt_info, sizeof(salt_info), okm_salt)) {
        return keys;
    }
    std::memcpy(keys.master_salt.data(), okm_salt, SRTP_MASTER_SALT_LEN);

    return keys;
}

} // namespace soundbridge
