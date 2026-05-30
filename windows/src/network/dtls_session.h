#pragma once

#include <soundbridge/types.h>
#include "srtp_context.h"

#include <cstdint>
#include <array>
#include <vector>
#include <string>
#include <optional>
#include <functional>

namespace soundbridge {

/// DTLS 握手配置
struct DtlsConfig {
    uint32_t handshake_timeout_ms = 5000;
    uint32_t max_retries = 3;
    std::array<uint8_t, 32> cert_fingerprint{};
};

/// DTLS 会话
///
/// 管理 DTLS 握手状态机和会话密钥派生。
/// 提供简化的 DTLS 握手流程，适用于局域网音频传输场景。
class DtlsSession {
public:
    DtlsSession();
    ~DtlsSession();

    DtlsSession(const DtlsSession&) = delete;
    DtlsSession& operator=(const DtlsSession&) = delete;

    /// 使用指定配置初始化
    bool initialize(const DtlsConfig& config);

    /// 获取当前握手状态
    DtlsState state() const { return state_; }

    /// 获取配置
    const DtlsConfig& config() const { return config_; }

    /// 开始握手（客户端模式）
    bool start_handshake();

    /// 处理握手消息
    /// 返回需要发送的响应（如果有）
    bool process_handshake(const uint8_t* data, size_t len,
                           std::vector<uint8_t>& response);

    /// 完成握手
    bool complete_handshake();

    /// 重试握手
    bool retry_handshake();

    /// 获取当前重试次数
    uint32_t retry_count() const { return retry_count_; }

    /// 获取派生的密钥材料（握手完成后可用）
    const CryptoKeys* keys() const { return keys_.has_value() ? &keys_.value() : nullptr; }

    /// 生成自签名证书指纹
    static std::array<uint8_t, 32> generate_certificate();

private:
    std::vector<uint8_t> build_server_hello() const;

    DtlsConfig config_;
    DtlsState state_ = DtlsState::Idle;
    uint32_t retry_count_ = 0;
    std::optional<CryptoKeys> keys_;
};

/// HKDF-SHA1 密钥派生
CryptoKeys derive_session_keys(const uint8_t* master_secret, size_t secret_len,
                                const uint8_t* salt, size_t salt_len);

} // namespace soundbridge
