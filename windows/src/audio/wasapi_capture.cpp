#include "wasapi_capture.h"

#include <functiondiscoverykeys_devpkey.h>
#include "log.h"
#include <stdexcept>

#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "mmdevapi.lib")

namespace soundbridge {

WasapiCapture::WasapiCapture() = default;

WasapiCapture::~WasapiCapture() {
    shutdown();
}

bool WasapiCapture::initialize(const AudioFormat& format, bool exclusive) {
    if (initialized_) {
        spdlog::warn("WasapiCapture already initialized");
        return false;
    }

    format_ = format;
    exclusive_mode_ = exclusive;

    if (!init_com()) {
        return false;
    }

    if (!find_render_device()) {
        return false;
    }

    if (!init_audio_client()) {
        return false;
    }

    initialized_ = true;
    spdlog::info("WasapiCapture initialized (mode: {})", exclusive_mode_ ? "exclusive" : "shared");
    return true;
}

void WasapiCapture::shutdown() {
    stop();

    if (event_handle_) {
        CloseHandle(event_handle_);
        event_handle_ = nullptr;
    }

    if (wave_format_) {
        CoTaskMemFree(wave_format_);
        wave_format_ = nullptr;
    }

    capture_client_.Reset();
    audio_client_.Reset();
    device_.Reset();
    enumerator_.Reset();

    initialized_ = false;
    spdlog::info("WasapiCapture shutdown");
}

bool WasapiCapture::start() {
    if (!initialized_ || running_) {
        return false;
    }

    HRESULT hr = audio_client_->Start();
    if (FAILED(hr)) {
        spdlog::error("Failed to start audio client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    running_ = true;
    spdlog::info("WasapiCapture started");
    return true;
}

void WasapiCapture::stop() {
    if (!running_) {
        return;
    }

    running_ = false;

    if (audio_client_) {
        audio_client_->Stop();
    }

    spdlog::info("WasapiCapture stopped");
}

bool WasapiCapture::read(float* buffer, uint32_t frame_count, uint32_t& frames_read) {
    if (!initialized_ || !running_ || !capture_client_) {
        frames_read = 0;
        return false;
    }

    frames_read = 0;
    UINT32 packet_length = 0;
    HRESULT hr = capture_client_->GetNextPacketSize(&packet_length);

    if (FAILED(hr)) {
        spdlog::error("GetNextPacketSize failed: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    while (packet_length > 0 && frames_read < frame_count) {
        BYTE* data = nullptr;
        UINT32 num_frames = 0;
        DWORD flags = 0;

        hr = capture_client_->GetBuffer(&data, &num_frames, &flags, nullptr, nullptr);
        if (FAILED(hr)) {
            spdlog::error("GetBuffer failed: 0x{:08X}", static_cast<uint32_t>(hr));
            return false;
        }

        const uint32_t frames_to_copy = std::min(num_frames, frame_count - frames_read);
        const uint32_t samples_to_copy = frames_to_copy * format_.channels;

        if (flags & AUDCLNT_BUFFERFLAGS_SILENT) {
            std::memset(buffer + frames_read * format_.channels, 0, samples_to_copy * sizeof(float));
        } else if (format_.sample_format == AudioSampleFormat::Float32) {
            std::memcpy(buffer + frames_read * format_.channels, data, samples_to_copy * sizeof(float));
        } else if (format_.sample_format == AudioSampleFormat::Int16) {
            const int16_t* src = reinterpret_cast<const int16_t*>(data);
            float* dst = buffer + frames_read * format_.channels;
            for (uint32_t i = 0; i < samples_to_copy; ++i) {
                dst[i] = static_cast<float>(src[i]) / 32768.0f;
            }
        }

        frames_read += frames_to_copy;
        capture_client_->ReleaseBuffer(num_frames);

        hr = capture_client_->GetNextPacketSize(&packet_length);
        if (FAILED(hr)) {
            break;
        }
    }

    return true;
}

bool WasapiCapture::init_com() {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr) && hr != RPC_E_CHANGED_MODE) {
        spdlog::error("CoInitializeEx failed: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    hr = CoCreateInstance(
        __uuidof(MMDeviceEnumerator),
        nullptr,
        CLSCTX_ALL,
        __uuidof(IMMDeviceEnumerator),
        reinterpret_cast<void**>(enumerator_.GetAddressOf())
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to create device enumerator: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    return true;
}

bool WasapiCapture::find_render_device() {
    HRESULT hr = enumerator_->GetDefaultAudioEndpoint(
        eRender,
        eConsole,
        device_.GetAddressOf()
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to get default audio endpoint: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    return true;
}

bool WasapiCapture::init_audio_client() {
    HRESULT hr = device_->Activate(
        __uuidof(IAudioClient),
        CLSCTX_ALL,
        nullptr,
        reinterpret_cast<void**>(audio_client_.GetAddressOf())
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to activate audio client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    WAVEFORMATEX wfx = {};
    wfx.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    wfx.nChannels = format_.channels;
    wfx.nSamplesPerSec = format_.sample_rate;
    wfx.wBitsPerSample = 32;
    wfx.nBlockAlign = wfx.nChannels * wfx.wBitsPerSample / 8;
    wfx.nAvgByteSec = wfx.nSamplesPerSec * wfx.nBlockAlign;

    // 独占模式：尝试 10ms 缓冲区（100000 * 100ns = 10ms）
    // 共享模式：50ms 缓冲区（500000 * 100ns = 50ms）
    const REFERENCE_TIME exclusive_duration = 100000;  // 10ms
    const REFERENCE_TIME shared_duration = 500000;     // 50ms

    // 阶段 1：尝试独占模式
    if (exclusive_mode_) {
        WAVEFORMATEX* closest = nullptr;
        hr = audio_client_->IsFormatSupported(
            AUDCLNT_SHAREMODE_EXCLUSIVE,
            &wfx,
            &closest
        );

        if (hr == S_FALSE && closest) {
            wfx = *closest;
            CoTaskMemFree(closest);
        }

        if (SUCCEEDED(hr) || hr == S_FALSE) {
            hr = audio_client_->Initialize(
                AUDCLNT_SHAREMODE_EXCLUSIVE,
                AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                exclusive_duration,
                exclusive_duration,
                &wfx,
                nullptr
            );

            if (SUCCEEDED(hr)) {
                spdlog::info("WasapiCapture: exclusive mode enabled (10ms buffer)");
                goto success;
            }

            spdlog::warn("WasapiCapture: exclusive mode failed (0x{:08X}), falling back to shared",
                         static_cast<uint32_t>(hr));
            exclusive_mode_ = false;

            // 重新创建 audio client（独占模式失败后需要重新创建）
            audio_client_.Reset();
            hr = device_->Activate(
                __uuidof(IAudioClient),
                CLSCTX_ALL,
                nullptr,
                reinterpret_cast<void**>(audio_client_.GetAddressOf())
            );
            if (FAILED(hr)) {
                spdlog::error("Failed to re-activate audio client: 0x{:08X}", static_cast<uint32_t>(hr));
                return false;
            }
        } else {
            spdlog::warn("WasapiCapture: exclusive mode not supported, using shared");
            exclusive_mode_ = false;
        }
    }

    // 阶段 2：共享模式
    {
        WAVEFORMATEX* closest = nullptr;
        hr = audio_client_->IsFormatSupported(
            AUDCLNT_SHAREMODE_SHARED,
            &wfx,
            &closest
        );

        if (hr == S_FALSE && closest) {
            wfx = *closest;
            CoTaskMemFree(closest);
        } else if (FAILED(hr)) {
            spdlog::error("Format not supported: 0x{:08X}", static_cast<uint32_t>(hr));
            return false;
        }

        hr = audio_client_->Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            shared_duration,
            0,
            &wfx,
            nullptr
        );

        if (FAILED(hr)) {
            spdlog::error("Failed to initialize audio client: 0x{:08X}", static_cast<uint32_t>(hr));
            return false;
        }
    }

success:
    hr = audio_client_->GetService(
        __uuidof(IAudioCaptureClient),
        reinterpret_cast<void**>(capture_client_.GetAddressOf())
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to get capture client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    return true;
}

} // namespace soundbridge
