#pragma once

#include "transport_interface.h"

#include <winsock2.h>
#include <ws2tcpip.h>

#include <atomic>
#include <mutex>

namespace soundbridge {

class QuicTransport final : public ITransport {
public:
    QuicTransport();
    ~QuicTransport() override;

    QuicTransport(const QuicTransport&) = delete;
    QuicTransport& operator=(const QuicTransport&) = delete;

    bool connect(const NetworkEndpoint& endpoint) override;
    void disconnect() override;

    bool send(const uint8_t* data, size_t size) override;
    bool receive(uint8_t* buffer, size_t buffer_size, size_t& received) override;

    bool is_connected() const override { return connected_; }
    TransportType type() const override { return TransportType::QUIC; }

private:
    bool init_winsock();
    bool handshake();

    bool connected_ = false;
    SOCKET socket_ = INVALID_SOCKET;
    sockaddr_in remote_addr_ = {};
    std::mutex send_mutex_;

    uint32_t send_sequence_ = 0;
    uint32_t recv_sequence_ = 0;
};

} // namespace soundbridge
