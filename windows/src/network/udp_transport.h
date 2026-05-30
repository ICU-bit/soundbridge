#pragma once

#include "transport_interface.h"
#include "srtp_context.h"

#include <winsock2.h>
#include <ws2tcpip.h>

#include <atomic>
#include <mutex>
#include <memory>
#include <thread>

namespace soundbridge {

class UdpTransport final : public ITransport {
public:
    UdpTransport();
    ~UdpTransport() override;

    UdpTransport(const UdpTransport&) = delete;
    UdpTransport& operator=(const UdpTransport&) = delete;

    bool connect(const NetworkEndpoint& endpoint) override;
    void disconnect() override;

    bool send(const uint8_t* data, size_t size) override;
    bool receive(uint8_t* buffer, size_t buffer_size, size_t& received) override;

    bool is_connected() const override { return connected_; }
    TransportType type() const override { return TransportType::UDP; }

    /// 启用 SRTP 加密
    bool enable_encryption(const CryptoKeys& keys, uint32_t ssrc);

    /// 禁用加密
    void disable_encryption();

    /// 是否已启用加密
    bool is_encrypted() const { return encryption_enabled_; }

    /// 获取 DTLS 状态
    DtlsState dtls_state() const;

private:
    bool init_winsock();

    bool connected_ = false;
    SOCKET socket_ = INVALID_SOCKET;
    sockaddr_in remote_addr_ = {};
    std::mutex send_mutex_;

    // SRTP 加密
    bool encryption_enabled_ = false;
    std::unique_ptr<SrtpContext> srtp_send_;
    std::unique_ptr<SrtpContext> srtp_recv_;
};

} // namespace soundbridge
