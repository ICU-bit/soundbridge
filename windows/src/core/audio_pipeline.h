#pragma once

#include "audio_types.h"

#include <functional>
#include <memory>
#include <vector>
#include <atomic>

namespace soundbridge {

class OpusCodec;
class WebRtcApm;

struct PipelineConfig {
    AudioFormat format;
    bool enable_aec = true;
    bool enable_ns = true;
    bool enable_agc = true;
    int opus_bitrate = 64000;
};

class AudioPipeline {
public:
    explicit AudioPipeline(const PipelineConfig& config);
    ~AudioPipeline();

    AudioPipeline(const AudioPipeline&) = delete;
    AudioPipeline& operator=(const AudioPipeline&) = delete;

    bool initialize();
    void shutdown();

    std::vector<uint8_t> encode(const float* pcm, uint32_t frame_count);
    std::vector<float> decode(const uint8_t* data, size_t size);

    void process_capture(float* data, uint32_t frame_count);
    void process_render(float* data, uint32_t frame_count);

    bool is_initialized() const { return initialized_; }

private:
    PipelineConfig config_;
    bool initialized_ = false;

    std::unique_ptr<OpusCodec> codec_;
    std::unique_ptr<WebRtcApm> apm_;
};

} // namespace soundbridge
