#pragma once

#include <soundbridge/types.h>

#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <wrl/client.h>

#include <atomic>
#include <functional>
#include <memory>

namespace soundbridge {

class WasapiCapture {
public:
    WasapiCapture();
    ~WasapiCapture();

    WasapiCapture(const WasapiCapture&) = delete;
    WasapiCapture& operator=(const WasapiCapture&) = delete;

    /// 初始化音频采集
    /// @param format 音频格式
    /// @param exclusive 独占模式（低延迟，失败时自动回退共享模式）
    bool initialize(const AudioFormat& format, bool exclusive = false);
    void shutdown();

    bool start();
    void stop();

    bool read(float* buffer, uint32_t frame_count, uint32_t& frames_read);

    bool is_initialized() const { return initialized_; }
    bool is_running() const { return running_; }

private:
    bool init_com();
    bool find_render_device();
    bool init_audio_client();

    AudioFormat format_;
    bool initialized_ = false;
    bool exclusive_mode_ = false;
    std::atomic<bool> running_{false};

    Microsoft::WRL::ComPtr<IMMDeviceEnumerator> enumerator_;
    Microsoft::WRL::ComPtr<IMMDevice> device_;
    Microsoft::WRL::ComPtr<IAudioClient> audio_client_;
    Microsoft::WRL::ComPtr<IAudioCaptureClient> capture_client_;

    WAVEFORMATEX* wave_format_ = nullptr;
    HANDLE event_handle_ = nullptr;
};

} // namespace soundbridge
