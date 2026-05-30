#pragma once

#include <soundbridge/audio_engine.h>
#include "audio_types.h"

#include <memory>
#include <thread>
#include <atomic>
#include <mutex>

namespace soundbridge {

class WasapiCapture;
class WasapiRenderer;
class OpusCodec;
class WebRtcApm;

class AudioEngineImpl final : public IAudioEngine {
public:
    AudioEngineImpl();
    ~AudioEngineImpl() override;

    AudioEngineImpl(const AudioEngineImpl&) = delete;
    AudioEngineImpl& operator=(const AudioEngineImpl&) = delete;

    bool initialize(const AudioFormat& format, bool exclusive = false) override;
    void shutdown() override;

    bool start_capture() override;
    void stop_capture() override;

    bool start_render() override;
    void stop_render() override;

    void set_capture_callback(AudioFrameCallback callback) override;
    void render_audio(const float* data, uint32_t frame_count) override;

    AudioStreamState capture_state() const override;
    AudioStreamState render_state() const override;

private:
    void capture_thread_func();
    void render_thread_func();

    AudioFormat format_;
    std::atomic<AudioStreamState> capture_state_{AudioStreamState::Idle};
    std::atomic<AudioStreamState> render_state_{AudioStreamState::Idle};

    std::unique_ptr<WasapiCapture> capture_;
    std::unique_ptr<WasapiRenderer> renderer_;
    std::unique_ptr<WebRtcApm> apm_;

    AudioFrameCallback capture_callback_;
    std::mutex callback_mutex_;

    std::thread capture_thread_;
    std::thread render_thread_;
    std::atomic<bool> running_{false};

    std::unique_ptr<AudioRingBuffer> capture_buffer_;
    std::unique_ptr<AudioRingBuffer> render_buffer_;
};

} // namespace soundbridge
