#pragma once

#include <soundbridge/audio_engine.h>
#include "audio_types.h"
#include "audio_pipeline.h"

#include <memory>
#include <thread>
#include <atomic>
#include <mutex>

namespace soundbridge {

class ITransport;
class AudioEngineImpl;

class SessionImpl final : public ISession {
public:
    SessionImpl();
    ~SessionImpl() override;

    SessionImpl(const SessionImpl&) = delete;
    SessionImpl& operator=(const SessionImpl&) = delete;

    bool connect(const SessionConfig& config) override;
    void disconnect() override;

    bool send_audio(const float* data, uint32_t frame_count) override;
    void set_receive_callback(AudioFrameCallback callback) override;

    AudioStreamState state() const override;

private:
    void receive_thread_func();
    void on_audio_captured(const float* data, uint32_t frame_count, uint8_t channels);

    SessionConfig config_;
    std::atomic<AudioStreamState> state_{AudioStreamState::Idle};

    std::unique_ptr<AudioPipeline> pipeline_;
    std::unique_ptr<ITransport> transport_;
    std::unique_ptr<IAudioEngine> engine_;

    AudioFrameCallback receive_callback_;
    std::mutex callback_mutex_;

    std::thread receive_thread_;
    std::atomic<bool> running_{false};
};

} // namespace soundbridge
