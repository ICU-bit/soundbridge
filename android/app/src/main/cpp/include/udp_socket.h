#pragma once

#include <cstdint>
#include <string>
#include <vector>
#include <functional>
#include <thread>
#include <atomic>
#include <mutex>

namespace soundbridge {

class UdpSocket {
public:
    UdpSocket();
    ~UdpSocket();

    bool bind(uint16_t port);
    bool sendTo(const uint8_t* data, int32_t size, const std::string& address, uint16_t port);
    std::vector<uint8_t> receiveFrom(int32_t buffer_size, std::string& sender_address, uint16_t& sender_port);

    void close();
    bool isOpen() const;

    using DataReceivedCallback = std::function<void(const uint8_t* data, int32_t size,
                                                     const std::string& address, uint16_t port)>;
    void startReceiving(int32_t buffer_size, DataReceivedCallback callback);
    void stopReceiving();

private:
    int socket_fd_;
    std::atomic<bool> is_open_;
    std::atomic<bool> is_receiving_;

    std::thread receive_thread_;
    std::mutex send_mutex_;

    void receiveThreadFunc(int32_t buffer_size, DataReceivedCallback callback);
};

} // namespace soundbridge
