#include "udp_transport.h"
#include "packet.h"

#include "log.h"

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
    encryption_enabled_ = false;
    srtp_send_.reset();
    srtp_recv_.reset();

    spdlog::info("UdpTransport disconnected");
}

bool UdpTransport::send(const uint8_t* data, size_t size) {
    if (!connected_ || socket_ == INVALID_SOCKET || !data || size == 0) {
        return false;
    }

    std::lock_guard<std::mutex> lock(send_mutex_);

    static uint32_t sequence = 0;
    auto packet = PacketBuilder::build(PacketType::Audio, sequence++, data, size);

    // 如果启用加密，对整个数据包进行 SRTP 加密
    const uint8_t* send_data = packet.data();
    size_t send_size = packet.size();
    std::vector<uint8_t> encrypted;

    if (encryption_enabled_ && srtp_send_) {
        if (!srtp_send_->protect(packet.data(), packet.size(), encrypted)) {
            spdlog::error("SRTP protect failed");
            return false;
        }
        send_data = encrypted.data();
        send_size = encrypted.size();
    }

    const int sent = sendto(
        socket_,
        reinterpret_cast<const char*>(send_data),
        static_cast<int>(send_size),
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

    // 如果启用加密，对接收到的数据进行 SRTP 解密
    if (encryption_enabled_ && srtp_recv_) {
        std::vector<uint8_t> decrypted;
        if (!srtp_recv_->unprotect(buffer, received, decrypted)) {
            spdlog::error("SRTP unprotect failed");
            received = 0;
            return false;
        }

        // 将解密后的数据复制回 buffer
        size_t copy_len = std::min(decrypted.size(), buffer_size);
        std::memcpy(buffer, decrypted.data(), copy_len);
        received = copy_len;
    }

    return true;
}

bool UdpTransport::enable_encryption(const CryptoKeys& keys, uint32_t ssrc) {
    srtp_send_ = std::make_unique<SrtpContext>();
    srtp_recv_ = std::make_unique<SrtpContext>();

    if (!srtp_send_->initialize(keys, ssrc)) {
        spdlog::error("Failed to initialize SRTP send context");
        srtp_send_.reset();
        srtp_recv_.reset();
        return false;
    }

    if (!srtp_recv_->initialize(keys, ssrc)) {
        spdlog::error("Failed to initialize SRTP recv context");
        srtp_send_.reset();
        srtp_recv_.reset();
        return false;
    }

    encryption_enabled_ = true;
    spdlog::info("SRTP encryption enabled, SSRC={:#x}", ssrc);
    return true;
}

void UdpTransport::disable_encryption() {
    encryption_enabled_ = false;
    srtp_send_.reset();
    srtp_recv_.reset();
    spdlog::info("SRTP encryption disabled");
}

DtlsState UdpTransport::dtls_state() const {
    // 当前实现：如果加密已启用则返回 Established
    if (encryption_enabled_) {
        return DtlsState::Established;
    }
    return DtlsState::Idle;
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
