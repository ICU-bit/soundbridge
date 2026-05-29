#include "session.h"
#include "audio/network/udp_transport.h"
#include "audio/network/quic_transport.h"

#include <spdlog/spdlog.h>

namespace soundbridge {

SessionImpl::SessionImpl() = default;

SessionImpl::~SessionImpl() {
    disconnect();
}

bool SessionImpl::connect(const SessionConfig& config) {
    if (state_ != AudioStreamState::Idle) {
        spdlog::warn("Session already connected");
        return false;
    }

    config_ = config;
    state_ = AudioStreamState::Starting;

    PipelineConfig pipeline_config;
    pipeline_config.format = config.audio_format;
    pipeline_config.enable_aec = config.enable_echo_cancellation;
    pipeline_config.enable_ns = config.enable_noise_suppression;
    pipeline_config.enable_agc = config.enable_agc;
    pipeline_config.opus_bitrate = config.opus_bitrate;

    pipeline_ = std::make_unique<AudioPipeline>(pipeline_config);
    if (!pipeline_->initialize()) {
        spdlog::error("Failed to initialize audio pipeline");
        state_ = AudioStreamState::Error;
        return false;
    }

    if (config.transport == TransportType::QUIC) {
        transport_ = std::make_unique<QuicTransport>();
    } else {
        transport_ = std::make_unique<UdpTransport>();
    }

    if (!transport_->connect(config.remote_endpoint)) {
        spdlog::error("Failed to connect to remote endpoint");
        state_ = AudioStreamState::Error;
        return false;
    }

    engine_ = create_audio_engine();
    if (!engine_->initialize(config.audio_format)) {
        spdlog::error("Failed to initialize audio engine");
        state_ = AudioStreamState::Error;
        return false;
    }

    engine_->set_capture_callback(
        [this](const float* data, uint32_t frame_count, uint8_t channels) {
            on_audio_captured(data, frame_count, channels);
        }
    );

    running_ = true;
    receive_thread_ = std::thread(&SessionImpl::receive_thread_func, this);

    engine_->start_capture();
    engine_->start_render();

    state_ = AudioStreamState::Running;
    spdlog::info("Session connected to {}:{}", config.remote_endpoint.address, config.remote_endpoint.port);
    return true;
}

void SessionImpl::disconnect() {
    if (state_ == AudioStreamState::Idle) {
        return;
    }

    state_ = AudioStreamState::Stopping;
    running_ = false;

    if (engine_) {
        engine_->stop_capture();
        engine_->stop_render();
        engine_->shutdown();
        engine_.reset();
    }

    if (transport_) {
        transport_->disconnect();
        transport_.reset();
    }

    if (pipeline_) {
        pipeline_->shutdown();
        pipeline_.reset();
    }

    if (receive_thread_.joinable()) {
        receive_thread_.join();
    }

    state_ = AudioStreamState::Idle;
    spdlog::info("Session disconnected");
}

bool SessionImpl::send_audio(const float* data, uint32_t frame_count) {
    if (state_ != AudioStreamState::Running || !pipeline_ || !transport_) {
        return false;
    }

    auto encoded = pipeline_->encode(data, frame_count);
    if (encoded.empty()) {
        return false;
    }

    return transport_->send(encoded.data(), encoded.size());
}

void SessionImpl::set_receive_callback(AudioFrameCallback callback) {
    std::lock_guard<std::mutex> lock(callback_mutex_);
    receive_callback_ = std::move(callback);
}

AudioStreamState SessionImpl::state() const {
    return state_;
}

void SessionImpl::receive_thread_func() {
    std::vector<uint8_t> buffer(4096);

    while (running_) {
        size_t received = 0;
        if (!transport_->receive(buffer.data(), buffer.size(), received)) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }

        if (received == 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }

        auto decoded = pipeline_->decode(buffer.data(), received);
        if (decoded.empty()) {
            continue;
        }

        if (engine_) {
            engine_->render_audio(decoded.data(), static_cast<uint32_t>(decoded.size() / config_.audio_format.channels));
        }

        std::lock_guard<std::mutex> lock(callback_mutex_);
        if (receive_callback_) {
            receive_callback_(decoded.data(),
                static_cast<uint32_t>(decoded.size() / config_.audio_format.channels),
                config_.audio_format.channels);
        }
    }
}

void SessionImpl::on_audio_captured(const float* data, uint32_t frame_count, uint8_t channels) {
    send_audio(data, frame_count);
}

SOUNDBRIDGE_API std::unique_ptr<ISession> SOUNDBRIDGE_CALL create_session() {
    return std::make_unique<SessionImpl>();
}

} // namespace soundbridge
