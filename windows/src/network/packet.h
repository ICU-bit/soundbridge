#pragma once

#include <cstdint>
#include <vector>
#include <cstring>

namespace soundbridge {

#pragma pack(push, 1)

struct PacketHeader {
    uint32_t magic = 0x53424447;
    uint16_t version = 1;
    uint16_t type = 0;
    uint32_t sequence = 0;
    uint32_t timestamp = 0;
    uint32_t payload_size = 0;
    uint32_t checksum = 0;
};

#pragma pack(pop)

enum class PacketType : uint16_t {
    Audio = 0x0001,
    Control = 0x0002,
    Heartbeat = 0x0003,
    Ack = 0x0004
};

class PacketBuilder {
public:
    static std::vector<uint8_t> build(PacketType type, uint32_t sequence,
                                       const uint8_t* payload, size_t payload_size) {
        const size_t total_size = sizeof(PacketHeader) + payload_size;
        std::vector<uint8_t> packet(total_size);

        PacketHeader header;
        header.type = static_cast<uint16_t>(type);
        header.sequence = sequence;
        header.payload_size = static_cast<uint32_t>(payload_size);

        header.checksum = compute_checksum(payload, payload_size);

        std::memcpy(packet.data(), &header, sizeof(PacketHeader));
        if (payload && payload_size > 0) {
            std::memcpy(packet.data() + sizeof(PacketHeader), payload, payload_size);
        }

        return packet;
    }

    static bool parse(const uint8_t* data, size_t size,
                      PacketHeader& header, const uint8_t*& payload) {
        if (size < sizeof(PacketHeader)) {
            return false;
        }

        std::memcpy(&header, data, sizeof(PacketHeader));

        if (header.magic != 0x53424447) {
            return false;
        }

        if (sizeof(PacketHeader) + header.payload_size > size) {
            return false;
        }

        payload = data + sizeof(PacketHeader);

        const uint32_t checksum = compute_checksum(payload, header.payload_size);
        if (checksum != header.checksum) {
            return false;
        }

        return true;
    }

private:
    static uint32_t compute_checksum(const uint8_t* data, size_t size) {
        uint32_t checksum = 0;
        for (size_t i = 0; i < size; ++i) {
            checksum = (checksum << 5) + checksum + data[i];
        }
        return checksum;
    }
};

} // namespace soundbridge
