#include "quic_transport.h"
#include "packet.h"

#include "log.h"

namespace soundbridge {

QuicTransport::QuicTransport() = default;

QuicTransport::~QuicTransport() {
    disconnect();
}

bool QuicTransport::connect(const NetworkEndpoint& endpoint) {
    if (connected_) {
        spdlog::warn("QuicTransport already connected");
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

    int timeout = 2000;
    setsockopt(socket_, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char*>(&timeout), sizeof(timeout));

    remote_addr_.sin_family = AF_INET;
    remote_addr_.sin_port = htons(endpoint.port);

    if (inet_pton(AF_INET, endpoint.address.c_str(), &remote_addr_.sin_addr) != 1) {
        spdlog::error("Invalid address: {}", endpoint.address);
        closesocket(socket_);
        socket_ = INVALID_SOCKET;
        return false;
    }

    if (!handshake()) {
        closesocket(socket_);
        socket_ = INVALID_SOCKET;
        return false;
    }

    connected_ = true;
    spdlog::info("QuicTransport connected to {}:{}", endpoint.address, endpoint.port);
    return true;
}

void QuicTransport::disconnect() {
    if (connected_ && socket_ != INVALID_SOCKET) {
        auto packet = PacketBuilder::build(PacketType::Control, 0, nullptr, 0);
        sendto(
            socket_,
            reinterpret_cast<const char*>(packet.data()),
            static_cast<int>(packet.size()),
            0,
            reinterpret_cast<const sockaddr*>(&remote_addr_),
            sizeof(remote_addr_)
        );
    }

    if (socket_ != INVALID_SOCKET) {
        closesocket(socket_);
        socket_ = INVALID_SOCKET;
    }

    connected_ = false;
    send_sequence_ = 0;
    recv_sequence_ = 0;
    spdlog::info("QuicTransport disconnected");
}

bool QuicTransport::send(const uint8_t* data, size_t size) {
    if (!connected_ || socket_ == INVALID_SOCKET || !data || size == 0) {
        return false;
    }

    std::lock_guard<std::mutex> lock(send_mutex_);

    auto packet = PacketBuilder::build(PacketType::Audio, send_sequence_++, data, size);

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

bool QuicTransport::receive(uint8_t* buffer, size_t buffer_size, size_t& received) {
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

    if (received >= sizeof(PacketHeader)) {
        PacketHeader header;
        const uint8_t* payload = nullptr;
        if (PacketBuilder::parse(buffer, received, header, payload)) {
            if (header.type == static_cast<uint16_t>(PacketType::Ack)) {
                return receive(buffer, buffer_size, received);
            }
            recv_sequence_ = header.sequence;
        }
    }

    return true;
}

bool QuicTransport::init_winsock() {
    WSADATA wsa_data;
    const int result = WSAStartup(MAKEWORD(2, 2), &wsa_data);
    if (result != 0) {
        spdlog::error("WSAStartup failed: {}", result);
        return false;
    }
    return true;
}

bool QuicTransport::handshake() {
    auto syn = PacketBuilder::build(PacketType::Control, 0, nullptr, 0);

    const int sent = sendto(
        socket_,
        reinterpret_cast<const char*>(syn.data()),
        static_cast<int>(syn.size()),
        0,
        reinterpret_cast<const sockaddr*>(&remote_addr_),
        sizeof(remote_addr_)
    );

    if (sent == SOCKET_ERROR) {
        spdlog::error("Handshake send failed: {}", WSAGetLastError());
        return false;
    }

    uint8_t buffer[1024];
    sockaddr_in from_addr = {};
    int from_len = sizeof(from_addr);

    for (int retry = 0; retry < 3; ++retry) {
        const int result = recvfrom(
            socket_,
            reinterpret_cast<char*>(buffer),
            sizeof(buffer),
            0,
            reinterpret_cast<sockaddr*>(&from_addr),
            &from_len
        );

        if (result > 0) {
            PacketHeader header;
            const uint8_t* payload = nullptr;
            if (PacketBuilder::parse(buffer, static_cast<size_t>(result), header, payload)) {
                if (header.type == static_cast<uint16_t>(PacketType::Ack)) {
                    spdlog::info("Handshake completed");
                    return true;
                }
            }
        }
    }

    spdlog::error("Handshake timeout");
    return false;
}

} // namespace soundbridge
