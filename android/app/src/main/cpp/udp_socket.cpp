#include "include/udp_socket.h"

#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <fcntl.h>
#include <cstring>
#include <cerrno>

#include <android/log.h>

#define LOG_TAG "SoundBridge_UdpSocket"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

namespace soundbridge {

UdpSocket::UdpSocket()
    : socket_fd_(-1)
    , is_open_(false)
    , is_receiving_(false) {
}

UdpSocket::~UdpSocket() {
    close();
}

bool UdpSocket::bind(uint16_t port) {
    socket_fd_ = socket(AF_INET, SOCK_DGRAM, 0);
    if (socket_fd_ < 0) {
        LOGE("Failed to create socket: %s", strerror(errno));
        return false;
    }

    int flags = fcntl(socket_fd_, F_GETFL, 0);
    fcntl(socket_fd_, F_SETFL, flags | O_NONBLOCK);

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons(port);

    if (::bind(socket_fd_, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        LOGE("Failed to bind socket to port %d: %s", port, strerror(errno));
        ::close(socket_fd_);
        socket_fd_ = -1;
        return false;
    }

    is_open_ = true;
    LOGI("Socket bound to port %d", port);
    return true;
}

bool UdpSocket::sendTo(const uint8_t* data, int32_t size,
                        const std::string& address, uint16_t port) {
    if (!is_open_ || socket_fd_ < 0) {
        LOGE("Socket not open");
        return false;
    }

    std::lock_guard<std::mutex> lock(send_mutex_);

    struct sockaddr_in dest_addr;
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(port);

    if (inet_pton(AF_INET, address.c_str(), &dest_addr.sin_addr) <= 0) {
        LOGE("Invalid address: %s", address.c_str());
        return false;
    }

    ssize_t sent = sendto(socket_fd_, data, size, 0,
                           (struct sockaddr*)&dest_addr, sizeof(dest_addr));

    if (sent < 0) {
        if (errno != EAGAIN && errno != EWOULDBLOCK) {
            LOGE("sendto failed: %s", strerror(errno));
        }
        return false;
    }

    return true;
}

std::vector<uint8_t> UdpSocket::receiveFrom(int32_t buffer_size,
                                              std::string& sender_address,
                                              uint16_t& sender_port) {
    if (!is_open_ || socket_fd_ < 0) {
        return {};
    }

    std::vector<uint8_t> buffer(buffer_size);
    struct sockaddr_in sender_addr;
    socklen_t addr_len = sizeof(sender_addr);

    ssize_t received = recvfrom(socket_fd_, buffer.data(), buffer_size, 0,
                                 (struct sockaddr*)&sender_addr, &addr_len);

    if (received < 0) {
        if (errno != EAGAIN && errno != EWOULDBLOCK) {
            LOGE("recvfrom failed: %s", strerror(errno));
        }
        return {};
    }

    char ip_str[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &sender_addr.sin_addr, ip_str, INET_ADDRSTRLEN);
    sender_address = ip_str;
    sender_port = ntohs(sender_addr.sin_port);

    buffer.resize(received);
    return buffer;
}

void UdpSocket::close() {
    stopReceiving();

    if (socket_fd_ >= 0) {
        ::close(socket_fd_);
        socket_fd_ = -1;
    }
    is_open_ = false;
    LOGI("Socket closed");
}

bool UdpSocket::isOpen() const {
    return is_open_;
}

void UdpSocket::startReceiving(int32_t buffer_size, DataReceivedCallback callback) {
    if (is_receiving_) return;

    is_receiving_ = true;
    receive_thread_ = std::thread(&UdpSocket::receiveThreadFunc, this,
                                   buffer_size, std::move(callback));
}

void UdpSocket::stopReceiving() {
    is_receiving_ = false;
    if (receive_thread_.joinable()) {
        receive_thread_.join();
    }
}

void UdpSocket::receiveThreadFunc(int32_t buffer_size, DataReceivedCallback callback) {
    LOGI("Receive thread started");

    while (is_receiving_ && is_open_) {
        std::string sender_address;
        uint16_t sender_port;

        auto data = receiveFrom(buffer_size, sender_address, sender_port);
        if (!data.empty() && callback) {
            callback(data.data(), data.size(), sender_address, sender_port);
        }

        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }

    LOGI("Receive thread stopped");
}

} // namespace soundbridge
