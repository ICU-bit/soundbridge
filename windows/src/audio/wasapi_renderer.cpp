#include "wasapi_renderer.h"

#include <functiondiscoverykeys_devpkey.h>
#include <spdlog/spdlog.h>

namespace soundbridge {

WasapiRenderer::WasapiRenderer() = default;

WasapiRenderer::~WasapiRenderer() {
    shutdown();
}

bool WasapiRenderer::initialize(const AudioFormat& format) {
    if (initialized_) {
        spdlog::warn("WasapiRenderer already initialized");
        return false;
    }

    format_ = format;

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
    spdlog::info("WasapiRenderer initialized");
    return true;
}

void WasapiRenderer::shutdown() {
    stop();

    render_client_.Reset();
    audio_client_.Reset();
    device_.Reset();
    enumerator_.Reset();

    initialized_ = false;
    spdlog::info("WasapiRenderer shutdown");
}

bool WasapiRenderer::start() {
    if (!initialized_ || running_) {
        return false;
    }

    HRESULT hr = audio_client_->Start();
    if (FAILED(hr)) {
        spdlog::error("Failed to start render client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    running_ = true;
    spdlog::info("WasapiRenderer started");
    return true;
}

void WasapiRenderer::stop() {
    if (!running_) {
        return;
    }

    running_ = false;

    if (audio_client_) {
        audio_client_->Stop();
    }

    spdlog::info("WasapiRenderer stopped");
}

bool WasapiRenderer::render(const float* data, uint32_t frame_count) {
    if (!initialized_ || !running_ || !render_client_) {
        return false;
    }

    UINT32 padding = 0;
    HRESULT hr = audio_client_->GetCurrentPadding(&padding);
    if (FAILED(hr)) {
        return false;
    }

    UINT32 available = buffer_frame_count_ - padding;
    UINT32 frames_to_write = std::min(frame_count, available);

    if (frames_to_write == 0) {
        return true;
    }

    BYTE* buffer = nullptr;
    hr = render_client_->GetBuffer(frames_to_write, &buffer);
    if (FAILED(hr)) {
        spdlog::error("GetBuffer failed: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    const uint32_t samples = frames_to_write * format_.channels;
    std::memcpy(buffer, data, samples * sizeof(float));

    hr = render_client_->ReleaseBuffer(frames_to_write, 0);
    if (FAILED(hr)) {
        spdlog::error("ReleaseBuffer failed: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    return true;
}

bool WasapiRenderer::init_com() {
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

bool WasapiRenderer::find_render_device() {
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

bool WasapiRenderer::init_audio_client() {
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

    // 低延迟模式：50ms 缓冲区（5000000 * 100ns = 50ms）
    // 共享模式下最小安全值，平衡稳定性和延迟
    const REFERENCE_TIME buffer_duration = 5000000;

    hr = audio_client_->Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        0,
        buffer_duration,
        0,
        &wfx,
        nullptr
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to initialize audio client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    hr = audio_client_->GetBufferSize(&buffer_frame_count_);
    if (FAILED(hr)) {
        spdlog::error("Failed to get buffer size: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    hr = audio_client_->GetService(
        __uuidof(IAudioRenderClient),
        reinterpret_cast<void**>(render_client_.GetAddressOf())
    );

    if (FAILED(hr)) {
        spdlog::error("Failed to get render client: 0x{:08X}", static_cast<uint32_t>(hr));
        return false;
    }

    return true;
}

} // namespace soundbridge
