#include "include/audio_processor.h"

#include <android/log.h>
#include <cmath>
#include <algorithm>
#include <cstring>

#define LOG_TAG "SoundBridge_AudioProcessor"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

namespace soundbridge {

AudioProcessor::AudioProcessor(int32_t sample_rate, int32_t channels)
    : sample_rate_(sample_rate)
    , channels_(channels)
    , echo_cancellation_enabled_(true)
    , noise_suppression_enabled_(true)
    , gain_control_enabled_(true)
    , gain_factor_(1.0f)
    , capture_level_(0.0f)
    , playback_level_(0.0f)
    , apm_instance_(nullptr) {
}

AudioProcessor::~AudioProcessor() {
    release();
}

bool AudioProcessor::initialize() {
    echo_reference_buffer_.resize(sample_rate_ * channels_);

    LOGI("AudioProcessor initialized: %dHz, %dch", sample_rate_, channels_);
    return true;
}

void AudioProcessor::release() {
    echo_reference_buffer_.clear();
    LOGI("AudioProcessor released");
}

void AudioProcessor::processCapture(int16_t* data, int32_t num_frames) {
    if (!data || num_frames <= 0) return;

    capture_level_ = calculateRMS(data, num_frames);

    if (noise_suppression_enabled_) {
        applyNoiseSuppression(data, num_frames);
    }

    if (gain_control_enabled_) {
        applyGainControl(data, num_frames);
    }
}

void AudioProcessor::processPlayback(int16_t* data, int32_t num_frames) {
    if (!data || num_frames <= 0) return;

    playback_level_ = calculateRMS(data, num_frames);

    int32_t samples_to_copy = std::min(num_frames * channels_,
                                        static_cast<int32_t>(echo_reference_buffer_.size()));
    std::memcpy(echo_reference_buffer_.data(), data, samples_to_copy * sizeof(int16_t));
}

void AudioProcessor::setEchoCancellationEnabled(bool enabled) {
    echo_cancellation_enabled_ = enabled;
    LOGI("Echo cancellation: %s", enabled ? "enabled" : "disabled");
}

void AudioProcessor::setNoiseSuppressionEnabled(bool enabled) {
    noise_suppression_enabled_ = enabled;
    LOGI("Noise suppression: %s", enabled ? "enabled" : "disabled");
}

void AudioProcessor::setGainControlEnabled(bool enabled) {
    gain_control_enabled_ = enabled;
    LOGI("Gain control: %s", enabled ? "enabled" : "disabled");
}

float AudioProcessor::getCaptureLevel() const {
    return capture_level_;
}

float AudioProcessor::getPlaybackLevel() const {
    return playback_level_;
}

void AudioProcessor::applyNoiseSuppression(int16_t* data, int32_t num_frames) {
    const float noise_floor = 500.0f;
    const float alpha = 0.95f;
    static float smoothed_level = 0.0f;

    for (int32_t i = 0; i < num_frames * channels_; ++i) {
        float sample = static_cast<float>(data[i]);
        float abs_sample = std::abs(sample);

        smoothed_level = alpha * smoothed_level + (1.0f - alpha) * abs_sample;

        if (smoothed_level < noise_floor) {
            data[i] = static_cast<int16_t>(sample * 0.1f);
        }
    }
}

void AudioProcessor::applyGainControl(int16_t* data, int32_t num_frames) {
    const float target_level = 0.5f;
    const float alpha = 0.01f;

    float current_level = calculateRMS(data, num_frames);

    if (current_level > 0.001f) {
        float desired_gain = target_level / current_level;
        gain_factor_ = (1.0f - alpha) * gain_factor_ + alpha * desired_gain;
        gain_factor_ = std::min(gain_factor_, 10.0f);
        gain_factor_ = std::max(gain_factor_, 0.1f);
    }

    for (int32_t i = 0; i < num_frames * channels_; ++i) {
        float sample = static_cast<float>(data[i]) * gain_factor_;
        sample = std::min(sample, 32767.0f);
        sample = std::max(sample, -32768.0f);
        data[i] = static_cast<int16_t>(sample);
    }
}

void AudioProcessor::applyEchoCancellation(int16_t* capture, int16_t* playback,
                                            int32_t num_frames) {
    if (!echo_cancellation_enabled_ || echo_reference_buffer_.empty()) {
        return;
    }

    const float suppression_factor = 0.8f;

    for (int32_t i = 0; i < num_frames * channels_; ++i) {
        if (i < static_cast<int32_t>(echo_reference_buffer_.size())) {
            float echo_estimate = static_cast<float>(echo_reference_buffer_[i]) * suppression_factor;
            float sample = static_cast<float>(capture[i]) - echo_estimate;
            sample = std::min(sample, 32767.0f);
            sample = std::max(sample, -32768.0f);
            capture[i] = static_cast<int16_t>(sample);
        }
    }
}

float AudioProcessor::calculateRMS(const int16_t* data, int32_t num_frames) {
    if (!data || num_frames <= 0) return 0.0f;

    int64_t sum = 0;
    int32_t total_samples = num_frames * channels_;

    for (int32_t i = 0; i < total_samples; ++i) {
        sum += static_cast<int64_t>(data[i]) * data[i];
    }

    float rms = std::sqrt(static_cast<float>(sum) / total_samples);
    return std::min(1.0f, rms / 32768.0f);
}

} // namespace soundbridge
