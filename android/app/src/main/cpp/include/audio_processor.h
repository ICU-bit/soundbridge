#pragma once

#include <cstdint>
#include <memory>
#include <vector>

namespace soundbridge {

class AudioProcessor {
public:
    AudioProcessor(int32_t sample_rate, int32_t channels);
    ~AudioProcessor();

    bool initialize();
    void release();

    void processCapture(int16_t* data, int32_t num_frames);
    void processPlayback(int16_t* data, int32_t num_frames);

    void setEchoCancellationEnabled(bool enabled);
    void setNoiseSuppressionEnabled(bool enabled);
    void setGainControlEnabled(bool enabled);

    float getCaptureLevel() const;
    float getPlaybackLevel() const;

private:
    void applyNoiseSuppression(int16_t* data, int32_t num_frames);
    void applyGainControl(int16_t* data, int32_t num_frames);
    void applyEchoCancellation(int16_t* capture, int16_t* playback, int32_t num_frames);
    float calculateRMS(const int16_t* data, int32_t num_frames);

    int32_t sample_rate_;
    int32_t channels_;

    bool echo_cancellation_enabled_;
    bool noise_suppression_enabled_;
    bool gain_control_enabled_;

    float gain_factor_;
    float capture_level_;
    float playback_level_;

    std::vector<int16_t> echo_reference_buffer_;

    void* apm_instance_;
};

} // namespace soundbridge
