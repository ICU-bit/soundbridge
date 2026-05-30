#pragma once

#include <cstdint>
#include <cstddef>
#include <string>
#include <vector>
#include <functional>

namespace soundbridge {

enum class AudioSampleFormat : uint8_t {
    Unknown = 0,
    Int16,
    Float32
};

struct AudioFormat {
    uint32_t sample_rate = 48000;
    uint8_t channels = 2;
    AudioSampleFormat sample_format = AudioSampleFormat::Float32;
    uint16_t bits_per_sample = 32;
    uint32_t frame_size = 480;

    uint32_t bytes_per_frame() const {
        return channels * (bits_per_sample / 8);
    }

    uint32_t bytes_per_buffer() const {
        return frame_size * bytes_per_frame();
    }
};

enum class TransportType : uint8_t {
    UDP = 0,
    QUIC = 1
};

enum class EncryptionMode : uint8_t {
    None = 0,    // 明文传输（仅测试用）
    SRTP = 1     // AES-128-CM + HMAC-SHA1-80
};

enum class DtlsState : uint8_t {
    Idle = 0,
    WaitingClientHello,
    ServerHelloSent,
    Established,
    Failed
};

struct SecurityConfig {
    EncryptionMode encryption = EncryptionMode::SRTP;
    uint32_t handshake_timeout_ms = 5000;
    uint32_t max_retries = 3;
    bool enable_auth_tag = true;
};

enum class AudioStreamState : uint8_t {
    Idle = 0,
    Starting,
    Running,
    Pausing,
    Paused,
    Stopping,
    Stopped,
    Error
};

struct NetworkEndpoint {
    std::string address;
    uint16_t port = 0;
};

struct SessionConfig {
    NetworkEndpoint remote_endpoint;
    TransportType transport = TransportType::UDP;
    AudioFormat audio_format;
    bool enable_echo_cancellation = true;
    bool enable_noise_suppression = true;
    bool enable_agc = true;
    int opus_bitrate = 64000;
    SecurityConfig security;
};

using AudioFrameCallback = std::function<void(const float* data, uint32_t frame_count, uint8_t channels)>;
using StateChangeCallback = std::function<void(AudioStreamState new_state)>;
using ErrorCallback = std::function<void(int error_code, const std::string& message)>;

} // namespace soundbridge
