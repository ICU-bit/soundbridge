#pragma once

#include <soundbridge/types.h>

#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <wrl/client.h>

#include <atomic>

namespace soundbridge {

class WasapiRenderer {
public:
    WasapiRenderer();
    ~WasapiRenderer();

    WasapiRenderer(const WasapiRenderer&) = delete;
    WasapiRenderer& operator=(const WasapiRenderer&) = delete;

    bool initialize(const AudioFormat& format);
    void shutdown();

    bool start();
    void stop();

    bool render(const float* data, uint32_t frame_count);

    bool is_initialized() const { return initialized_; }
    bool is_running() const { return running_; }

private:
    bool init_com();
    bool find_render_device();
    bool init_audio_client();

    AudioFormat format_;
    bool initialized_ = false;
    std::atomic<bool> running_{false};

    Microsoft::WRL::ComPtr<IMMDeviceEnumerator> enumerator_;
    Microsoft::WRL::ComPtr<IMMDevice> device_;
    Microsoft::WRL::ComPtr<IAudioClient> audio_client_;
    Microsoft::WRL::ComPtr<IAudioRenderClient> render_client_;

    UINT32 buffer_frame_count_ = 0;
};

} // namespace soundbridge
