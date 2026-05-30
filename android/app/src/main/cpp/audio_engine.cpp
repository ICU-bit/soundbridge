#include "include/audio_engine.h"
#include "include/audio_processor.h"

#include <android/log.h>
#include <cmath>
#include <cstring>

#define LOG_TAG "SoundBridge_AudioEngine"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

namespace soundbridge {

AudioEngine::AudioEngine()
    : state_(AudioEngineState::IDLE)
    , is_running_(false)
    , echo_cancellation_enabled_(true)
    , noise_suppression_enabled_(true)
    , gain_control_enabled_(true)
    , audio_level_(0.0f)
    , audio_mode_(0)
    , audio_stream_(nullptr) {
}

AudioEngine::~AudioEngine() {
    release();
}

bool AudioEngine::initialize(const AudioConfig& config) {
    std::lock_guard<std::mutex> lock(mutex_);

    if (state_ != AudioEngineState::IDLE) {
        LOGE("Engine already initialized");
        return false;
    }

    config_ = config;

    capture_buffer_ = std::make_unique<int16_t[]>(config_.buffer_size * config_.channels);
    playback_buffer_ = std::make_unique<int16_t[]>(config_.buffer_size * config_.channels);

    if (!capture_buffer_ || !playback_buffer_) {
        LOGE("Failed to allocate buffers");
        return false;
    }

    state_ = AudioEngineState::INITIALIZED;
    LOGI("Audio engine initialized: %dHz, %dch, %d samples",
         config_.sample_rate, config_.channels, config_.buffer_size);

    return true;
}

bool AudioEngine::start() {
    std::lock_guard<std::mutex> lock(mutex_);

    if (state_ != AudioEngineState::INITIALIZED) {
        LOGE("Engine not initialized");
        return false;
    }

    is_running_ = true;
    processing_thread_ = std::thread(&AudioEngine::audioProcessingThread, this);

    state_ = AudioEngineState::RUNNING;
    LOGI("Audio engine started");

    return true;
}

void AudioEngine::stop() {
    {
        std::lock_guard<std::mutex> lock(mutex_);
        is_running_ = false;
    }

    if (processing_thread_.joinable()) {
        processing_thread_.join();
    }

    state_ = AudioEngineState::STOPPED;
    LOGI("Audio engine stopped");
}

void AudioEngine::release() {
    stop();

    std::lock_guard<std::mutex> lock(mutex_);

    capture_buffer_.reset();
    playback_buffer_.reset();
    audio_callback_ = nullptr;

    state_ = AudioEngineState::IDLE;
    LOGI("Audio engine released");
}

AudioEngineState AudioEngine::getState() const {
    return state_;
}

float AudioEngine::getAudioLevel() const {
    return audio_level_.load();
}

void AudioEngine::setEchoCancellationEnabled(bool enabled) {
    echo_cancellation_enabled_ = enabled;
}

void AudioEngine::setNoiseSuppressionEnabled(bool enabled) {
    noise_suppression_enabled_ = enabled;
}

void AudioEngine::setGainControlEnabled(bool enabled) {
    gain_control_enabled_ = enabled;
}

void AudioEngine::setAudioMode(int mode) {
    if (mode < 0 || mode > 2) {
        LOGE("Invalid audio mode: %d, must be 0-2", mode);
        return;
    }
    audio_mode_ = mode;
    const char* modeNames[] = {"Balanced", "High Quality", "Low Latency"};
    LOGI("Audio mode set to: %s (%d)", modeNames[mode], mode);
}

void AudioEngine::setAudioDataCallback(AudioDataCallback callback) {
    std::lock_guard<std::mutex> lock(mutex_);
    audio_callback_ = std::move(callback);
}

void AudioEngine::audioProcessingThread() {
    LOGI("Audio processing thread started");

    while (is_running_) {
        if (capture_buffer_ && audio_callback_) {
            processAudioBuffer(capture_buffer_.get(), config_.buffer_size);
            audio_callback_(capture_buffer_.get(), config_.buffer_size);
        }

        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    LOGI("Audio processing thread stopped");
}

void AudioEngine::processAudioBuffer(int16_t* buffer, int32_t num_frames) {
    float level = calculateRMS(buffer, num_frames);
    audio_level_ = level;
}

float AudioEngine::calculateRMS(const int16_t* data, int32_t num_frames) {
    if (!data || num_frames <= 0) return 0.0f;

    int64_t sum = 0;
    for (int32_t i = 0; i < num_frames; ++i) {
        sum += static_cast<int64_t>(data[i]) * data[i];
    }

    float rms = std::sqrt(static_cast<float>(sum) / num_frames);
    return std::min(1.0f, rms / 32768.0f);
}

} // namespace soundbridge
