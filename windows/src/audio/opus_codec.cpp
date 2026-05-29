#include "opus_codec.h"

#include <spdlog/spdlog.h>

namespace soundbridge {

OpusCodec::OpusCodec() = default;

OpusCodec::~OpusCodec() {
    shutdown();
}

bool OpusCodec::initialize(uint32_t sample_rate, uint8_t channels, int bitrate) {
    if (initialized_) {
        spdlog::warn("OpusCodec already initialized");
        return false;
    }

    sample_rate_ = sample_rate;
    channels_ = channels;
    bitrate_ = bitrate;

    int error = 0;
    encoder_ = opus_encoder_create(
        static_cast<opus_int32>(sample_rate),
        channels,
        OPUS_APPLICATION_VOIP,
        &error
    );

    if (error != OPUS_OK || !encoder_) {
        spdlog::error("Failed to create Opus encoder: {}", opus_strerror(error));
        return false;
    }

    error = opus_encoder_ctl(encoder_, OPUS_SET_BITRATE(bitrate));
    if (error != OPUS_OK) {
        spdlog::warn("Failed to set Opus bitrate: {}", opus_strerror(error));
    }

    error = opus_encoder_ctl(encoder_, OPUS_SET_COMPLEXITY(10));
    if (error != OPUS_OK) {
        spdlog::warn("Failed to set Opus complexity: {}", opus_strerror(error));
    }

    decoder_ = opus_decoder_create(
        static_cast<opus_int32>(sample_rate),
        channels,
        &error
    );

    if (error != OPUS_OK || !decoder_) {
        spdlog::error("Failed to create Opus decoder: {}", opus_strerror(error));
        opus_encoder_destroy(encoder_);
        encoder_ = nullptr;
        return false;
    }

    initialized_ = true;
    spdlog::info("OpusCodec initialized: {}Hz, {} channels, {} bps", sample_rate, channels, bitrate);
    return true;
}

void OpusCodec::shutdown() {
    if (encoder_) {
        opus_encoder_destroy(encoder_);
        encoder_ = nullptr;
    }

    if (decoder_) {
        opus_decoder_destroy(decoder_);
        decoder_ = nullptr;
    }

    initialized_ = false;
    spdlog::info("OpusCodec shutdown");
}

std::vector<uint8_t> OpusCodec::encode(const float* pcm, uint32_t frame_count) {
    if (!initialized_ || !encoder_ || !pcm || frame_count == 0) {
        return {};
    }

    const int max_packet_size = 4000;
    std::vector<uint8_t> output(max_packet_size);

    const opus_int32 encoded_bytes = opus_encode_float(
        encoder_,
        pcm,
        static_cast<int>(frame_count),
        output.data(),
        max_packet_size
    );

    if (encoded_bytes < 0) {
        spdlog::error("Opus encode error: {}", opus_strerror(encoded_bytes));
        return {};
    }

    output.resize(static_cast<size_t>(encoded_bytes));
    return output;
}

std::vector<float> OpusCodec::decode(const uint8_t* data, size_t size) {
    if (!initialized_ || !decoder_ || !data || size == 0) {
        return {};
    }

    const int max_frame_size = 5760;
    std::vector<float> output(max_frame_size * channels_);

    const int decoded_samples = opus_decode_float(
        decoder_,
        data,
        static_cast<opus_int32>(size),
        output.data(),
        max_frame_size,
        0
    );

    if (decoded_samples < 0) {
        spdlog::error("Opus decode error: {}", opus_strerror(decoded_samples));
        return {};
    }

    output.resize(static_cast<size_t>(decoded_samples * channels_));
    return output;
}

bool OpusCodec::set_bitrate(int bitrate) {
    if (!initialized_ || !encoder_) {
        return false;
    }

    const int error = opus_encoder_ctl(encoder_, OPUS_SET_BITRATE(bitrate));
    if (error != OPUS_OK) {
        spdlog::error("Failed to set Opus bitrate: {}", opus_strerror(error));
        return false;
    }

    bitrate_ = bitrate;
    return true;
}

bool OpusCodec::set_complexity(int complexity) {
    if (!initialized_ || !encoder_) {
        return false;
    }

    const int error = opus_encoder_ctl(encoder_, OPUS_SET_COMPLEXITY(complexity));
    if (error != OPUS_OK) {
        spdlog::error("Failed to set Opus complexity: {}", opus_strerror(error));
        return false;
    }

    return true;
}

} // namespace soundbridge
