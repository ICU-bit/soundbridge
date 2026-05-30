#pragma once

#include "export.h"
#include "types.h"

#include <memory>

namespace soundbridge {

class IAudioEngine {
public:
    virtual ~IAudioEngine() = default;

    virtual bool initialize(const AudioFormat& format, bool exclusive = false) = 0;
    virtual void shutdown() = 0;

    virtual bool start_capture() = 0;
    virtual void stop_capture() = 0;

    virtual bool start_render() = 0;
    virtual void stop_render() = 0;

    virtual void set_capture_callback(AudioFrameCallback callback) = 0;
    virtual void render_audio(const float* data, uint32_t frame_count) = 0;

    virtual AudioStreamState capture_state() const = 0;
    virtual AudioStreamState render_state() const = 0;
};

class ISession {
public:
    virtual ~ISession() = default;

    virtual bool connect(const SessionConfig& config) = 0;
    virtual void disconnect() = 0;

    virtual bool send_audio(const float* data, uint32_t frame_count) = 0;
    virtual void set_receive_callback(AudioFrameCallback callback) = 0;

    virtual AudioStreamState state() const = 0;

    /// 启用 DTLS/SRTP 加密
    virtual bool enable_encryption() = 0;

    /// 禁用加密
    virtual void disable_encryption() = 0;

    /// 获取加密状态
    virtual bool is_encrypted() const = 0;

    /// 获取 DTLS 握手状态
    virtual DtlsState dtls_state() const = 0;
};

SOUNDBRIDGE_API std::unique_ptr<IAudioEngine> SOUNDBRIDGE_CALL create_audio_engine();
SOUNDBRIDGE_API std::unique_ptr<ISession> SOUNDBRIDGE_CALL create_session();

} // namespace soundbridge
