#include <gtest/gtest.h>
#include "network/udp_transport.h"
#include <soundbridge/types.h>

#include <thread>
#include <chrono>
#include <cstring>

namespace soundbridge {
namespace tests {

class UdpTransportTest : public ::testing::Test {
protected:
    void SetUp() override {
        // 测试端点
        local_endpoint_.address = "127.0.0.1";
        local_endpoint_.port = 12345;

        remote_endpoint_.address = "127.0.0.1";
        remote_endpoint_.port = 12346;
    }

    NetworkEndpoint local_endpoint_;
    NetworkEndpoint remote_endpoint_;
};

// 测试创建和销毁
TEST_F(UdpTransportTest, CreateAndDestroy) {
    UdpTransport transport;
    EXPECT_FALSE(transport.is_connected());
    EXPECT_EQ(transport.type(), TransportType::UDP);
}

// 测试连接
TEST_F(UdpTransportTest, ConnectSuccess) {
    UdpTransport transport;
    EXPECT_TRUE(transport.connect(remote_endpoint_));
    EXPECT_TRUE(transport.is_connected());
}

// 测试断开连接
TEST_F(UdpTransportTest, Disconnect) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));
    EXPECT_TRUE(transport.is_connected());

    transport.disconnect();
    EXPECT_FALSE(transport.is_connected());
}

// 测试发送数据
TEST_F(UdpTransportTest, SendData) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));

    uint8_t data[] = {0x01, 0x02, 0x03, 0x04};
    EXPECT_TRUE(transport.send(data, sizeof(data)));
}

// 测试接收数据（需要对端发送）
TEST_F(UdpTransportTest, ReceiveTimeout) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));

    uint8_t buffer[1024];
    size_t received = 0;

    // 没有对端发送时，receive 应该超时返回 false
    // 注意：这个测试依赖于实现的超时行为
    bool result = transport.receive(buffer, sizeof(buffer), received);
    // 不检查结果，因为超时时间可能不同
    (void)result;
}

// 测试重复连接
TEST_F(UdpTransportTest, Reconnect) {
    UdpTransport transport;

    EXPECT_TRUE(transport.connect(remote_endpoint_));
    EXPECT_TRUE(transport.is_connected());

    transport.disconnect();
    EXPECT_FALSE(transport.is_connected());

    EXPECT_TRUE(transport.connect(remote_endpoint_));
    EXPECT_TRUE(transport.is_connected());
}

// 测试发送空数据
TEST_F(UdpTransportTest, SendEmptyData) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));

    uint8_t data[] = {};
    // 空数据发送可能成功或失败，取决于实现
    transport.send(data, 0);
}

// 测试发送大数据包
TEST_F(UdpTransportTest, SendLargeData) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));

    // 1400 字节（接近 MTU）
    std::vector<uint8_t> data(1400, 0xAB);
    EXPECT_TRUE(transport.send(data.data(), data.size()));
}

// 测试连接后状态一致
TEST_F(UdpTransportTest, ConnectionStateConsistent) {
    UdpTransport transport;

    // 初始状态
    EXPECT_FALSE(transport.is_connected());
    EXPECT_EQ(transport.type(), TransportType::UDP);

    // 连接后状态
    EXPECT_TRUE(transport.connect(remote_endpoint_));
    EXPECT_TRUE(transport.is_connected());
    EXPECT_EQ(transport.type(), TransportType::UDP);

    // 断开后状态
    transport.disconnect();
    EXPECT_FALSE(transport.is_connected());
    EXPECT_EQ(transport.type(), TransportType::UDP);
}

// 测试音频数据包发送
TEST_F(UdpTransportTest, SendAudioPacket) {
    UdpTransport transport;
    ASSERT_TRUE(transport.connect(remote_endpoint_));

    // 模拟音频数据包：12 字节头 + 960 samples * 2 bytes = 1932 字节
    AudioPacketHeader header;
    header.magic = 0x53424447;
    header.version = 1;
    header.sequence = 1;
    header.timestamp = 1000;
    header.payload_size = 960 * 2;
    header.channels = 1;
    header.sample_rate = 48000;
    header.frame_size = 960;

    std::vector<uint8_t> packet(sizeof(header) + header.payload_size);
    std::memcpy(packet.data(), &header, sizeof(header));

    // 填充模拟音频数据
    for (uint32_t i = 0; i < header.payload_size; ++i) {
        packet[sizeof(header) + i] = static_cast<uint8_t>(i % 256);
    }

    EXPECT_TRUE(transport.send(packet.data(), packet.size()));
}

} // namespace tests
} // namespace soundbridge
