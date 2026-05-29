#include "udp_transport.h"
#include "packet.h"

#include <spdlog/spdlog.h>

#pragma comment(lib, "ws2_32.lib")

namespace soundbridge {

UdpTransport::UdpTransport() = default;

UdpTransport::~UdpTransport() {
    disconnect();
}

bool UdpTransport::connect(const NetworkEndpoint& endpoint) {
    if (connected_) {
        spdlog::warn("UdpTransport already connected");
        return false;
    }

    if (!init_winsock()) {
        return false;
    }

    socket_ = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (socket_ == INVALID_SOCKET) {
        spdlog::error("Failed to create socket: {}", WSAGetLastError());
        return false;
    }

    int timeout = 1000;
    setsockopt(socket_, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char*>(&timeout), sizeof(timeout));

    remote_addr_.sin_family = AF_INET;
    remote_addr_.sin_port = htons(endpoint.port);

    if (inet_pton(AF_INET, endpoint.address.c_str(), &remote_addr_.sin_addr) != 1) {
        spdlog::error("Invalid address: {}", endpoint.address);
        closesocket(socket_);
        socket_ = INVALID_SOCKET;
        return false;
    }

    connected_ = true;
    spdlog::info("UdpTransport connected to {}:{}", endpoint.address, endpoint.port);
    return true;
}

void UdpTransport::disconnect() {
    if (socket_ != INVALID_SOCKET) {
        closesocket(socket_);
        socket_ = INVALID_SOCKET;
    }

    connected_ = false;
    spdlog::info("UdpTransport disconnected");
}

bool UdpTransport::send(const uint8_t* data, size_t size) {
    if (!connected_ || socket_ == INVALID_SOCKET || !data || size == 0) {
        return false;
    }

    std::lock_guard<std::mutex> lock(send_mutex_);

    static uint32_t sequence = 0;
    auto packet = PacketBuilder::build(PacketType::Audio, sequence++, data, size);

    const int sent = sendto(
        socket_,
        reinterpret_cast<const char*>(packet.data()),
        static_cast<int>(packet.size()),
        0,
        reinterpret_cast<const sockaddr*>(&remote_addr_),
        sizeof(remote_addr_)
    );

    if (sent == SOCKET_ERROR) {
        spdlog::error("sendto failed: {}", WSAGetLastError());
        return false;
    }

    return true;
}

bool UdpTransport::receive(uint8_t* buffer, size_t buffer_size, size_t& received) {
    if (!connected_ || socket_ == INVALID_SOCKET || !buffer) {
        received = 0;
        return false;
    }

    sockaddr_in from_addr = {};
    int from_len = sizeof(from_addr);

    const int result = recvfrom(
        socket_,
        reinterpret_cast<char*>(buffer),
        static_cast<int>(buffer_size),
        0,
        reinterpret_cast<sockaddr*>(&from_addr),
        &from_len
    );

    if (result == SOCKET_ERROR) {
        const int error = WSAGetLastError();
        if (error != WSAETIMEDOUT) {
            spdlog::error("recvfrom failed: {}", error);
        }
        received = 0;
        return false;
    }

    received = static_cast<size_t>(result);
    return true;
}

bool UdpTransport::init_winsock() {
    WSADATA wsa_data;
    const int result = WSAStartup(MAKEWORD(2, 2), &wsa_data);
    if (result != 0) {
        spdlog::error("WSAStartup failed: {}", result);
        return false;
    }
    return true;
}

} // namespace soundbridge
