#pragma once

#include "transport_interface.h"

#include <winsock2.h>
#include <ws2tcpip.h>

#include <atomic>
#include <mutex>
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

private:
    bool init_winsock();

    bool connected_ = false;
    SOCKET socket_ = INVALID_SOCKET;
    sockaddr_in remote_addr_ = {};
    std::mutex send_mutex_;
};

} // namespace soundbridge
