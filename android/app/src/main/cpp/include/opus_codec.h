#pragma once

#include <cstdint>
#include <vector>
#include <memory>

struct OpusEncoder;
struct OpusDecoder;

namespace soundbridge {

class OpusEncoderWrapper {
public:
    OpusEncoderWrapper(int32_t sample_rate, int32_t channels, int32_t bitrate, int32_t complexity);
    ~OpusEncoderWrapper();

    bool initialize();
    void release();

    std::vector<uint8_t> encode(const int16_t* pcm_data, int32_t frame_size);
    int32_t getBitrate() const;
    bool setBitrate(int32_t bitrate);

private:
    int32_t sample_rate_;
    int32_t channels_;
    int32_t bitrate_;
    int32_t complexity_;

    OpusEncoder* encoder_;
    bool initialized_;

    std::vector<uint8_t> encode_buffer_;
};

class OpusDecoderWrapper {
public:
    OpusDecoderWrapper(int32_t sample_rate, int32_t channels);
    ~OpusDecoderWrapper();

    bool initialize();
    void release();

    std::vector<int16_t> decode(const uint8_t* opus_data, int32_t data_size, int32_t frame_size);

private:
    int32_t sample_rate_;
    int32_t channels_;

    OpusDecoder* decoder_;
    bool initialized_;

    std::vector<int16_t> decode_buffer_;
};

} // namespace soundbridge
