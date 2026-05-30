#pragma once

#include <cstdint>
#include <memory>
#include <vector>

#ifdef SOUNDBRIDGE_HAS_WEBRTC
namespace webrtc {
class AudioProcessing;
struct StreamConfig;
}
#endif

namespace soundbridge {

class WebRtcApm {
public:
    WebRtcApm();
    ~WebRtcApm();

    WebRtcApm(const WebRtcApm&) = delete;
    WebRtcApm& operator=(const WebRtcApm&) = delete;

    bool initialize(uint32_t sample_rate, uint8_t channels);
    void shutdown();

    bool process_stream(const float* input, float* output, uint32_t frame_count);
    bool process_reverse_stream(const float* input, float* output, uint32_t frame_count);

    void set_echo_cancellation_enabled(bool enabled);
    void set_noise_suppression_enabled(bool enabled);
    void set_agc_enabled(bool enabled);

    bool is_initialized() const { return initialized_; }

private:
    bool initialized_ = false;
    uint32_t sample_rate_ = 0;
    uint8_t channels_ = 0;

#ifdef SOUNDBRIDGE_HAS_WEBRTC
    std::unique_ptr<webrtc::AudioProcessing> apm_;
#endif
};

} // namespace soundbridge
