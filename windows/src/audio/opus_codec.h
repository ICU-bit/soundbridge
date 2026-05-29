#pragma once

#include <opus/opus.h>

#include <cstdint>
#include <vector>
#include <memory>

namespace soundbridge {

class OpusCodec {
public:
    OpusCodec();
    ~OpusCodec();

    OpusCodec(const OpusCodec&) = delete;
    OpusCodec& operator=(const OpusCodec&) = delete;

    bool initialize(uint32_t sample_rate, uint8_t channels, int bitrate);
    void shutdown();

    std::vector<uint8_t> encode(const float* pcm, uint32_t frame_count);
    std::vector<float> decode(const uint8_t* data, size_t size);

    bool set_bitrate(int bitrate);
    bool set_complexity(int complexity);

    bool is_initialized() const { return initialized_; }

private:
    bool initialized_ = false;
    uint32_t sample_rate_ = 0;
    uint8_t channels_ = 0;
    int bitrate_ = 0;

    OpusEncoder* encoder_ = nullptr;
    OpusDecoder* decoder_ = nullptr;
};

} // namespace soundbridge
