#include "audio_pipeline.h"
#include "audio/opus_codec.h"
#include "audio/webrtc_apm.h"

namespace soundbridge {

AudioPipeline::AudioPipeline(const PipelineConfig& config)
    : config_(config) {}

AudioPipeline::~AudioPipeline() {
    shutdown();
}

bool AudioPipeline::initialize() {
    if (initialized_) {
        return false;
    }

    codec_ = std::make_unique<OpusCodec>();
    if (!codec_->initialize(config_.format.sample_rate, config_.format.channels, config_.opus_bitrate)) {
        return false;
    }

    if (config_.enable_aec || config_.enable_ns || config_.enable_agc) {
        apm_ = std::make_unique<WebRtcApm>();
        if (!apm_->initialize(config_.format.sample_rate, config_.format.channels)) {
            return false;
        }

        apm_->set_echo_cancellation_enabled(config_.enable_aec);
        apm_->set_noise_suppression_enabled(config_.enable_ns);
        apm_->set_agc_enabled(config_.enable_agc);
    }

    initialized_ = true;
    return true;
}

void AudioPipeline::shutdown() {
    initialized_ = false;

    if (codec_) {
        codec_->shutdown();
        codec_.reset();
    }

    if (apm_) {
        apm_->shutdown();
        apm_.reset();
    }
}

std::vector<uint8_t> AudioPipeline::encode(const float* pcm, uint32_t frame_count) {
    if (!initialized_ || !codec_) {
        return {};
    }
    return codec_->encode(pcm, frame_count);
}

std::vector<float> AudioPipeline::decode(const uint8_t* data, size_t size) {
    if (!initialized_ || !codec_) {
        return {};
    }
    return codec_->decode(data, size);
}

void AudioPipeline::process_capture(float* data, uint32_t frame_count) {
    if (!initialized_ || !apm_) {
        return;
    }
    apm_->process_stream(data, data, frame_count);
}

void AudioPipeline::process_render(float* data, uint32_t frame_count) {
    if (!initialized_ || !apm_) {
        return;
    }
    apm_->process_reverse_stream(data, data, frame_count);
}

} // namespace soundbridge
