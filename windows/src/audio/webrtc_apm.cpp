#include "webrtc_apm.h"

#ifdef SOUNDBRIDGE_HAS_WEBRTC

#include <api/audio/audio_processing.h>
#include <api/audio/echo_canceller3_config.h>
#include <modules/audio_processing/audio_processing_impl.h>
#include <modules/audio_processing/include/audio_processing.h>

#include "log.h"

namespace soundbridge {

WebRtcApm::WebRtcApm() = default;

WebRtcApm::~WebRtcApm() {
    shutdown();
}

bool WebRtcApm::initialize(uint32_t sample_rate, uint8_t channels) {
    if (initialized_) {
        spdlog::warn("WebRtcApm already initialized");
        return false;
    }

    sample_rate_ = sample_rate;
    channels_ = channels;

    webrtc::AudioProcessing::Config config;
    config.echo_canceller.enabled = true;
    config.echo_canceller.mobile_mode = false;
    config.noise_suppression.enabled = true;
    config.noise_suppression.level = webrtc::AudioProcessing::Config::NoiseSuppression::Level::kHigh;
    config.gain_controller1.enabled = true;
    config.gain_controller1.mode = webrtc::AudioProcessing::Config::GainController1::kAdaptiveAnalog;
    config.gain_controller1.target_level_dbfs = 3;
    config.gain_controller1.compression_gain_db = 9;

    apm_ = webrtc::AudioProcessing::Create(config);

    if (!apm_) {
        spdlog::error("Failed to create AudioProcessing instance");
        return false;
    }

    webrtc::StreamConfig input_config(sample_rate, channels, false);
    webrtc::StreamConfig output_config(sample_rate, channels, false);

    int error = apm_->set_stream_delay_ms(0);
    if (error != webrtc::AudioProcessing::kNoError) {
        spdlog::warn("Failed to set stream delay: {}", error);
    }

    initialized_ = true;
    spdlog::info("WebRtcApm initialized: {}Hz, {} channels", sample_rate, channels);
    return true;
}

void WebRtcApm::shutdown() {
    apm_.reset();
    initialized_ = false;
    spdlog::info("WebRtcApm shutdown");
}

bool WebRtcApm::process_stream(const float* input, float* output, uint32_t frame_count) {
    if (!initialized_ || !apm_ || !input || !output) {
        return false;
    }

    webrtc::StreamConfig config(sample_rate_, channels_, false);

    const float* const src_ptr = input;
    float* const dst_ptr = output;

    int error = apm_->ProcessStream(
        &src_ptr,
        config,
        config,
        &dst_ptr
    );

    if (error != webrtc::AudioProcessing::kNoError) {
        spdlog::error("ProcessStream failed: {}", error);
        return false;
    }

    return true;
}

bool WebRtcApm::process_reverse_stream(const float* input, float* output, uint32_t frame_count) {
    if (!initialized_ || !apm_ || !input || !output) {
        return false;
    }

    webrtc::StreamConfig config(sample_rate_, channels_, false);

    const float* const src_ptr = input;
    float* const dst_ptr = output;

    int error = apm_->ProcessReverseStream(
        &src_ptr,
        config,
        config,
        &dst_ptr
    );

    if (error != webrtc::AudioProcessing::kNoError) {
        spdlog::error("ProcessReverseStream failed: {}", error);
        return false;
    }

    return true;
}

void WebRtcApm::set_echo_cancellation_enabled(bool enabled) {
    if (!apm_) {
        return;
    }

    webrtc::AudioProcessing::Config config = apm_->GetConfig();
    config.echo_canceller.enabled = enabled;
    apm_->ApplyConfig(config);
}

void WebRtcApm::set_noise_suppression_enabled(bool enabled) {
    if (!apm_) {
        return;
    }

    webrtc::AudioProcessing::Config config = apm_->GetConfig();
    config.noise_suppression.enabled = enabled;
    apm_->ApplyConfig(config);
}

void WebRtcApm::set_agc_enabled(bool enabled) {
    if (!apm_) {
        return;
    }

    webrtc::AudioProcessing::Config config = apm_->GetConfig();
    config.gain_controller1.enabled = enabled;
    apm_->ApplyConfig(config);
}

} // namespace soundbridge

#else // !SOUNDBRIDGE_HAS_WEBRTC

// Stub implementation when WebRTC is not available
namespace soundbridge {

WebRtcApm::WebRtcApm() = default;
WebRtcApm::~WebRtcApm() = default;

bool WebRtcApm::initialize(uint32_t sample_rate, uint8_t channels) {
    (void)sample_rate;
    (void)channels;
    return false;
}

void WebRtcApm::shutdown() {}

bool WebRtcApm::process_stream(const float* input, float* output, uint32_t frame_count) {
    // No-op passthrough when WebRTC is unavailable
    if (input && output && frame_count > 0) {
        std::copy(input, input + frame_count, output);
    }
    return false;
}

bool WebRtcApm::process_reverse_stream(const float* input, float* output, uint32_t frame_count) {
    if (input && output && frame_count > 0) {
        std::copy(input, input + frame_count, output);
    }
    return false;
}

void WebRtcApm::set_echo_cancellation_enabled(bool) {}
void WebRtcApm::set_noise_suppression_enabled(bool) {}
void WebRtcApm::set_agc_enabled(bool) {}

} // namespace soundbridge

#endif // SOUNDBRIDGE_HAS_WEBRTC
