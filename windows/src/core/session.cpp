#include "session.h"
#include "audio/network/udp_transport.h"
#include "audio/network/quic_transport.h"
#include "audio/network/dtls_session.h"

#include "log.h"

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

    // 如果配置了加密，执行 DTLS 握手
    if (config.security.encryption == EncryptionMode::SRTP) {
        dtls_session_ = std::make_unique<DtlsSession>();
        DtlsConfig dtls_config;
        dtls_config.handshake_timeout_ms = config.security.handshake_timeout_ms;
        dtls_config.max_retries = config.security.max_retries;
        dtls_config.cert_fingerprint = DtlsSession::generate_certificate();

        if (!dtls_session_->initialize(dtls_config)) {
            spdlog::error("Failed to initialize DTLS session");
            state_ = AudioStreamState::Error;
            return false;
        }

        // 启动握手
        if (!dtls_session_->start_handshake()) {
            spdlog::error("Failed to start DTLS handshake");
            state_ = AudioStreamState::Error;
            return false;
        }

        // 模拟握手完成（实际应通过网络交换消息）
        std::vector<uint8_t> response;
        dtls_session_->process_handshake(nullptr, 0, response);
        dtls_session_->complete_handshake();

        // 启用 SRTP 加密
        if (dtls_session_->keys()) {
            auto* udp = dynamic_cast<UdpTransport*>(transport_.get());
            if (udp) {
                if (!udp->enable_encryption(*dtls_session_->keys(), 0x12345678)) {
                    spdlog::error("Failed to enable SRTP encryption");
                    state_ = AudioStreamState::Error;
                    return false;
                }
                encryption_enabled_ = true;
                spdlog::info("DTLS/SRTP encryption enabled");
            }
        }
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

    dtls_session_.reset();
    encryption_enabled_ = false;

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

bool SessionImpl::enable_encryption() {
    if (encryption_enabled_) {
        return true;
    }

    if (!transport_) {
        return false;
    }

    auto* udp = dynamic_cast<UdpTransport*>(transport_.get());
    if (!udp) {
        return false;
    }

    // 生成新的密钥材料
    auto keys = CryptoKeys::generate();
    if (!udp->enable_encryption(keys, 0x12345678)) {
        return false;
    }

    encryption_enabled_ = true;
    return true;
}

void SessionImpl::disable_encryption() {
    if (!encryption_enabled_) {
        return;
    }

    auto* udp = dynamic_cast<UdpTransport*>(transport_.get());
    if (udp) {
        udp->disable_encryption();
    }

    encryption_enabled_ = false;
}

bool SessionImpl::is_encrypted() const {
    return encryption_enabled_;
}

DtlsState SessionImpl::dtls_state() const {
    if (dtls_session_) {
        return dtls_session_->state();
    }
    return DtlsState::Idle;
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
