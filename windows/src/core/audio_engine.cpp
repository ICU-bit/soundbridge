#include "audio_engine.h"
#include "audio/wasapi_capture.h"
#include "audio/wasapi_renderer.h"
#include "audio/webrtc_apm.h"

#include "log.h"

namespace soundbridge {

AudioEngineImpl::AudioEngineImpl() = default;

AudioEngineImpl::~AudioEngineImpl() {
    shutdown();
}

bool AudioEngineImpl::initialize(const AudioFormat& format, bool exclusive) {
    if (capture_state_ != AudioStreamState::Idle) {
        spdlog::warn("AudioEngine already initialized");
        return false;
    }

    format_ = format;

    capture_ = std::make_unique<WasapiCapture>();
    if (!capture_->initialize(format, exclusive)) {
        spdlog::error("Failed to initialize WASAPI capture");
        return false;
    }

    renderer_ = std::make_unique<WasapiRenderer>();
    if (!renderer_->initialize(format, exclusive)) {
        spdlog::error("Failed to initialize WASAPI renderer");
        return false;
    }

    apm_ = std::make_unique<WebRtcApm>();
    if (!apm_->initialize(format.sample_rate, format.channels)) {
        spdlog::warn("Failed to initialize WebRTC APM, echo cancellation disabled");
    }

    const size_t buffer_frames = format.sample_rate / 10;
    capture_buffer_ = std::make_unique<AudioRingBuffer>(buffer_frames * format.channels);
    render_buffer_ = std::make_unique<AudioRingBuffer>(buffer_frames * format.channels);

    spdlog::info("AudioEngine initialized: {}Hz, {} channels, {} bit",
        format.sample_rate, format.channels, format.bits_per_sample);
    return true;
}

void AudioEngineImpl::shutdown() {
    running_ = false;

    if (capture_thread_.joinable()) {
        capture_thread_.join();
    }
    if (render_thread_.joinable()) {
        render_thread_.join();
    }

    if (capture_) {
        capture_->shutdown();
        capture_.reset();
    }
    if (renderer_) {
        renderer_->shutdown();
        renderer_.reset();
    }
    if (apm_) {
        apm_->shutdown();
        apm_.reset();
    }

    capture_buffer_.reset();
    render_buffer_.reset();

    capture_state_ = AudioStreamState::Idle;
    render_state_ = AudioStreamState::Idle;

    spdlog::info("AudioEngine shutdown");
}

bool AudioEngineImpl::start_capture() {
    if (!capture_ || capture_state_ != AudioStreamState::Idle) {
        return false;
    }

    capture_state_ = AudioStreamState::Starting;

    if (!capture_->start()) {
        capture_state_ = AudioStreamState::Error;
        spdlog::error("Failed to start WASAPI capture");
        return false;
    }

    running_ = true;
    capture_thread_ = std::thread(&AudioEngineImpl::capture_thread_func, this);
    capture_state_ = AudioStreamState::Running;

    spdlog::info("Audio capture started");
    return true;
}

void AudioEngineImpl::stop_capture() {
    if (capture_state_ != AudioStreamState::Running) {
        return;
    }

    capture_state_ = AudioStreamState::Stopping;
    running_ = false;

    if (capture_thread_.joinable()) {
        capture_thread_.join();
    }

    if (capture_) {
        capture_->stop();
    }

    capture_state_ = AudioStreamState::Idle;
    spdlog::info("Audio capture stopped");
}

bool AudioEngineImpl::start_render() {
    if (!renderer_ || render_state_ != AudioStreamState::Idle) {
        return false;
    }

    render_state_ = AudioStreamState::Starting;

    if (!renderer_->start()) {
        render_state_ = AudioStreamState::Error;
        spdlog::error("Failed to start WASAPI renderer");
        return false;
    }

    render_state_ = AudioStreamState::Running;
    spdlog::info("Audio render started");
    return true;
}

void AudioEngineImpl::stop_render() {
    if (render_state_ != AudioStreamState::Running) {
        return;
    }

    render_state_ = AudioStreamState::Stopping;

    if (renderer_) {
        renderer_->stop();
    }

    render_state_ = AudioStreamState::Idle;
    spdlog::info("Audio render stopped");
}

void AudioEngineImpl::set_capture_callback(AudioFrameCallback callback) {
    std::lock_guard<std::mutex> lock(callback_mutex_);
    capture_callback_ = std::move(callback);
}

void AudioEngineImpl::render_audio(const float* data, uint32_t frame_count) {
    if (render_state_ != AudioStreamState::Running || !renderer_) {
        return;
    }

    if (apm_ && apm_->is_initialized()) {
        std::vector<float> processed(frame_count * format_.channels);
        apm_->process_reverse_stream(data, processed.data(), frame_count);
        renderer_->render(processed.data(), frame_count);
    } else {
        renderer_->render(data, frame_count);
    }
}

AudioStreamState AudioEngineImpl::capture_state() const {
    return capture_state_;
}

AudioStreamState AudioEngineImpl::render_state() const {
    return render_state_;
}

void AudioEngineImpl::capture_thread_func() {
    const uint32_t frame_size = format_.frame_size;
    std::vector<float> buffer(frame_size * format_.channels);

    while (running_) {
        uint32_t captured = 0;
        if (!capture_->read(buffer.data(), frame_size, captured)) {
            spdlog::error("Capture read failed");
            break;
        }

        if (captured == 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }

        if (apm_ && apm_->is_initialized()) {
            std::vector<float> processed(captured * format_.channels);
            apm_->process_stream(buffer.data(), processed.data(), captured);

            std::lock_guard<std::mutex> lock(callback_mutex_);
            if (capture_callback_) {
                capture_callback_(processed.data(), captured, format_.channels);
            }
        } else {
            std::lock_guard<std::mutex> lock(callback_mutex_);
            if (capture_callback_) {
                capture_callback_(buffer.data(), captured, format_.channels);
            }
        }
    }
}

void AudioEngineImpl::render_thread_func() {
    while (running_) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
}

SOUNDBRIDGE_API std::unique_ptr<IAudioEngine> SOUNDBRIDGE_CALL create_audio_engine() {
    return std::make_unique<AudioEngineImpl>();
}

} // namespace soundbridge
