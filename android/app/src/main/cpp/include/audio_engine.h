#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <atomic>
#include <thread>
#include <mutex>

namespace soundbridge {

enum class AudioEngineState {
    IDLE,
    INITIALIZED,
    RUNNING,
    STOPPED,
    ERROR
};

struct AudioConfig {
    int32_t sample_rate;
    int32_t channels;
    int32_t buffer_size;
    int32_t bit_depth;
};

class AudioEngine {
public:
    AudioEngine();
    ~AudioEngine();

    bool initialize(const AudioConfig& config);
    bool start();
    void stop();
    void release();

    AudioEngineState getState() const;
    float getAudioLevel() const;

    void setEchoCancellationEnabled(bool enabled);
    void setNoiseSuppressionEnabled(bool enabled);
    void setGainControlEnabled(bool enabled);

    // Audio mode: 0=BALANCED, 1=HIGH_QUALITY, 2=LOW_LATENCY
    void setAudioMode(int mode);
    int getAudioMode() const;

    // Mix ratio: PC volume and phone volume (0.0~1.0)
    void setMixRatio(float pcVolume, float phoneVolume);
    void getMixRatio(float& pcVolume, float& phoneVolume) const;

    using AudioDataCallback = std::function<void(const int16_t* data, int32_t num_frames)>;
    void setAudioDataCallback(AudioDataCallback callback);

private:
    void audioProcessingThread();
    void processAudioBuffer(int16_t* buffer, int32_t num_frames);
    float calculateRMS(const int16_t* data, int32_t num_frames);

    AudioConfig config_;
    AudioEngineState state_;

    std::atomic<bool> is_running_;
    std::atomic<bool> echo_cancellation_enabled_;
    std::atomic<bool> noise_suppression_enabled_;
    std::atomic<bool> gain_control_enabled_;

    std::thread processing_thread_;
    std::mutex mutex_;

    AudioDataCallback audio_callback_;

    std::unique_ptr<int16_t[]> capture_buffer_;
    std::unique_ptr<int16_t[]> playback_buffer_;

    std::atomic<float> audio_level_;

    int audio_mode_;  // 0=BALANCED, 1=HIGH_QUALITY, 2=LOW_LATENCY
    float mix_pc_volume_;   // PC 音量 (0.0~1.0)
    float mix_phone_volume_; // 手机音量 (0.0~1.0)

    void* audio_stream_;
};

} // namespace soundbridge
