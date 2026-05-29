#include "include/opus_codec.h"
#include <opus/opus.h>

#include <android/log.h>
#include <cstring>

#define LOG_TAG "SoundBridge_OpusCodec"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

namespace soundbridge {

OpusEncoderWrapper::OpusEncoderWrapper(int32_t sample_rate, int32_t channels,
                                         int32_t bitrate, int32_t complexity)
    : sample_rate_(sample_rate)
    , channels_(channels)
    , bitrate_(bitrate)
    , complexity_(complexity)
    , encoder_(nullptr)
    , initialized_(false) {
    encode_buffer_.resize(4000);
}

OpusEncoderWrapper::~OpusEncoderWrapper() {
    release();
}

bool OpusEncoderWrapper::initialize() {
    int error;
    encoder_ = opus_encoder_create(sample_rate_, channels_, OPUS_APPLICATION_VOIP, &error);

    if (error != OPUS_OK || !encoder_) {
        LOGE("Failed to create Opus encoder: %s", opus_strerror(error));
        return false;
    }

    opus_encoder_ctl(encoder_, OPUS_SET_BITRATE(bitrate_));
    opus_encoder_ctl(encoder_, OPUS_SET_COMPLEXITY(complexity_));
    opus_encoder_ctl(encoder_, OPUS_SET_SIGNAL(OPUS_SIGNAL_VOICE));
    opus_encoder_ctl(encoder_, OPUS_SET_INBAND_FEC(1));
    opus_encoder_ctl(encoder_, OPUS_SET_PACKET_LOSS_PERC(5));

    initialized_ = true;
    LOGI("Opus encoder initialized: %dHz, %dch, %dbps",
         sample_rate_, channels_, bitrate_);

    return true;
}

void OpusEncoderWrapper::release() {
    if (encoder_) {
        opus_encoder_destroy(encoder_);
        encoder_ = nullptr;
    }
    initialized_ = false;
}

std::vector<uint8_t> OpusEncoderWrapper::encode(const int16_t* pcm_data, int32_t frame_size) {
    if (!initialized_ || !encoder_ || !pcm_data) {
        return {};
    }

    int32_t encoded_size = opus_encode(encoder_, pcm_data, frame_size,
                                        encode_buffer_.data(), encode_buffer_.size());

    if (encoded_size < 0) {
        LOGE("Opus encode error: %s", opus_strerror(encoded_size));
        return {};
    }

    return std::vector<uint8_t>(encode_buffer_.begin(),
                                 encode_buffer_.begin() + encoded_size);
}

int32_t OpusEncoderWrapper::getBitrate() const {
    return bitrate_;
}

bool OpusEncoderWrapper::setBitrate(int32_t bitrate) {
    if (!encoder_) return false;

    int error = opus_encoder_ctl(encoder_, OPUS_SET_BITRATE(bitrate));
    if (error == OPUS_OK) {
        bitrate_ = bitrate;
        return true;
    }
    return false;
}

OpusDecoderWrapper::OpusDecoderWrapper(int32_t sample_rate, int32_t channels)
    : sample_rate_(sample_rate)
    , channels_(channels)
    , decoder_(nullptr)
    , initialized_(false) {
    decode_buffer_.resize(5760);
}

OpusDecoderWrapper::~OpusDecoderWrapper() {
    release();
}

bool OpusDecoderWrapper::initialize() {
    int error;
    decoder_ = opus_decoder_create(sample_rate_, channels_, &error);

    if (error != OPUS_OK || !decoder_) {
        LOGE("Failed to create Opus decoder: %s", opus_strerror(error));
        return false;
    }

    initialized_ = true;
    LOGI("Opus decoder initialized: %dHz, %dch", sample_rate_, channels_);

    return true;
}

void OpusDecoderWrapper::release() {
    if (decoder_) {
        opus_decoder_destroy(decoder_);
        decoder_ = nullptr;
    }
    initialized_ = false;
}

std::vector<int16_t> OpusDecoderWrapper::decode(const uint8_t* opus_data,
                                                  int32_t data_size,
                                                  int32_t frame_size) {
    if (!initialized_ || !decoder_ || !opus_data || data_size <= 0) {
        return {};
    }

    if (frame_size > static_cast<int32_t>(decode_buffer_.size())) {
        decode_buffer_.resize(frame_size);
    }

    int32_t decoded_samples = opus_decode(decoder_, opus_data, data_size,
                                           decode_buffer_.data(), frame_size, 0);

    if (decoded_samples < 0) {
        LOGE("Opus decode error: %s", opus_strerror(decoded_samples));
        return {};
    }

    return std::vector<int16_t>(decode_buffer_.begin(),
                                 decode_buffer_.begin() + decoded_samples);
}

} // namespace soundbridge
