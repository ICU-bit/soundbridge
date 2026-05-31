#include <jni.h>
#include <android/log.h>
#include <atomic>
#include <mutex>
#include <string>

#include "include/audio_engine.h"
#ifdef SOUNDBRIDGE_HAS_OPUS
#include "include/opus_codec.h"
#endif
#include "include/udp_socket.h"

// ── Rust FFI integration ──
#ifdef SOUNDBRIDGE_USE_RUST_FFI
#include "soundbridge.h"
#endif

#define LOG_TAG "SoundBridge_JNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

static soundbridge::AudioEngine* getEngine(jlong handle) {
    return reinterpret_cast<soundbridge::AudioEngine*>(handle);
}

#ifdef SOUNDBRIDGE_HAS_OPUS
static soundbridge::OpusEncoderWrapper* getEncoder(jlong handle) {
    return reinterpret_cast<soundbridge::OpusEncoderWrapper*>(handle);
}

static soundbridge::OpusDecoderWrapper* getDecoder(jlong handle) {
    return reinterpret_cast<soundbridge::OpusDecoderWrapper*>(handle);
}
#endif

static soundbridge::UdpSocket* getSocket(jlong handle) {
    return reinterpret_cast<soundbridge::UdpSocket*>(handle);
}

// ── Rust FFI handle helpers ──
#ifdef SOUNDBRIDGE_USE_RUST_FFI
static void* getRustEngine(jlong handle) {
    return reinterpret_cast<void*>(static_cast<uintptr_t>(handle));
}
static jlong fromRustHandle(void* ptr) {
    return static_cast<jlong>(reinterpret_cast<uintptr_t>(ptr));
}
#endif

extern "C" {

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeInit(
        JNIEnv* env, jobject thiz, jint sampleRate, jint channels, jint bufferSize) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    // Rust FFI path: create engine via Rust core library
    void* engine = sb_engine_create();
    if (!engine) {
        const char* err = sb_last_error();
        LOGE("Failed to create Rust engine: %s", err ? err : "unknown error");
        return 0;
    }

    // Bind to an auto-assigned UDP port
    int rc = sb_bind(engine, 0);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Failed to bind Rust engine: %s", err ? err : "unknown error");
        sb_engine_destroy(engine);
        return 0;
    }

    // Retrieve the assigned port for logging
    uint16_t port = 0;
    rc = sb_local_port(engine, &port);
    if (rc != 0) {
        LOGE("Failed to get local port from Rust engine");
        sb_engine_destroy(engine);
        return 0;
    }

    LOGI("Rust engine created, bound to UDP port %d (requested %dHz, %dch, %d samples)",
         port, sampleRate, channels, bufferSize);
    return fromRustHandle(engine);
#else
    // Legacy C++ path
    auto* engine = new soundbridge::AudioEngine();

    soundbridge::AudioConfig config;
    config.sample_rate = sampleRate;
    config.channels = channels;
    config.buffer_size = bufferSize;
    config.bit_depth = 16;

    if (!engine->initialize(config)) {
        LOGE("Failed to initialize audio engine");
        delete engine;
        return 0;
    }

    LOGI("Audio engine created: %dHz, %dch, %d samples", sampleRate, channels, bufferSize);
    return reinterpret_cast<jlong>(engine);
#endif
}

JNIEXPORT jboolean JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeStart(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return JNI_FALSE;
    return engine->start() ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeStop(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->stop();
    }
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeRelease(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (engine) {
        sb_engine_destroy(engine);
        LOGI("Rust engine destroyed");
    }
#else
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->release();
        delete engine;
    }
#endif
}

JNIEXPORT jfloat JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetAudioLevel(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return 0.0f;
    return engine->getAudioLevel();
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEchoCancellationEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled) {
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->setEchoCancellationEnabled(enabled);
    }
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetNoiseSuppressionEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled) {
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->setNoiseSuppressionEnabled(enabled);
    }
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetGainControlEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled) {
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->setGainControlEnabled(enabled);
    }
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetAudioMode(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint mode) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;
    engine->setAudioMode(mode);
    return 0;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetAudioMode(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return 0; // 默认 Balanced
    return engine->getAudioMode();
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetMixRatio(
        JNIEnv* env, jobject thiz, jlong engineHandle, jfloat pcVolume, jfloat phoneVolume) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;
    engine->setMixRatio(pcVolume, phoneVolume);
    return 0;
}

JNIEXPORT jfloatArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetMixRatio(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return nullptr;

    float pcVolume = 0.5f;
    float phoneVolume = 0.5f;
    engine->getMixRatio(pcVolume, phoneVolume);

    jfloatArray result = env->NewFloatArray(2);
    if (result) {
        jfloat buf[2] = {pcVolume, phoneVolume};
        env->SetFloatArrayRegion(result, 0, 2, buf);
    }
    return result;
}

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeCreateEncoder(
        JNIEnv* env, jobject thiz, jint bitrate, jint complexity) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* encoder = new soundbridge::OpusEncoderWrapper(48000, 1, bitrate, complexity);

    if (!encoder->initialize()) {
        LOGE("Failed to initialize Opus encoder");
        delete encoder;
        return 0;
    }

    LOGI("Opus encoder created: %dbps, complexity=%d", bitrate, complexity);
    return reinterpret_cast<jlong>(encoder);
#else
    LOGE("Opus not available");
    return 0;
#endif
}

JNIEXPORT jbyteArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeEncodeFrame(
        JNIEnv* env, jobject thiz, jlong encoderHandle, jbyteArray pcmData, jint frameSize) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* encoder = getEncoder(encoderHandle);
    if (!encoder) return nullptr;

    jbyte* pcm = env->GetByteArrayElements(pcmData, nullptr);
    if (!pcm) return nullptr;

    auto encoded = encoder->encode(reinterpret_cast<const int16_t*>(pcm), frameSize);

    env->ReleaseByteArrayElements(pcmData, pcm, JNI_ABORT);

    if (encoded.empty()) return nullptr;

    jbyteArray result = env->NewByteArray(encoded.size());
    env->SetByteArrayRegion(result, 0, encoded.size(),
                             reinterpret_cast<const jbyte*>(encoded.data()));

    return result;
#else
    return nullptr;
#endif
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeReleaseEncoder(
        JNIEnv* env, jobject thiz, jlong encoderHandle) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* encoder = getEncoder(encoderHandle);
    if (encoder) {
        encoder->release();
        delete encoder;
    }
#endif
}

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeCreateDecoder(
        JNIEnv* env, jobject thiz) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* decoder = new soundbridge::OpusDecoderWrapper(48000, 1);

    if (!decoder->initialize()) {
        LOGE("Failed to initialize Opus decoder");
        delete decoder;
        return 0;
    }

    LOGI("Opus decoder created");
    return reinterpret_cast<jlong>(decoder);
#else
    LOGE("Opus not available");
    return 0;
#endif
}

JNIEXPORT jbyteArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDecodeFrame(
        JNIEnv* env, jobject thiz, jlong decoderHandle, jbyteArray opusData) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* decoder = getDecoder(decoderHandle);
    if (!decoder) return nullptr;

    jbyte* opus = env->GetByteArrayElements(opusData, nullptr);
    jint size = env->GetArrayLength(opusData);
    if (!opus) return nullptr;

    auto decoded = decoder->decode(reinterpret_cast<const uint8_t*>(opus), size, 960);

    env->ReleaseByteArrayElements(opusData, opus, JNI_ABORT);

    if (decoded.empty()) return nullptr;

    jbyteArray result = env->NewByteArray(decoded.size() * sizeof(int16_t));
    env->SetByteArrayRegion(result, 0, decoded.size() * sizeof(int16_t),
                             reinterpret_cast<const jbyte*>(decoded.data()));

    return result;
#else
    return nullptr;
#endif
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeReleaseDecoder(
        JNIEnv* env, jobject thiz, jlong decoderHandle) {
#ifdef SOUNDBRIDGE_HAS_OPUS
    auto* decoder = getDecoder(decoderHandle);
    if (decoder) {
        decoder->release();
        delete decoder;
    }
#endif
}

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeCreateUdpSocket(
        JNIEnv* env, jobject thiz, jint port) {
    auto* socket = new soundbridge::UdpSocket();

    if (!socket->bind(port)) {
        LOGE("Failed to bind UDP socket to port %d", port);
        delete socket;
        return 0;
    }

    LOGI("UDP socket created on port %d", port);
    return reinterpret_cast<jlong>(socket);
}

JNIEXPORT jboolean JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSendTo(
        JNIEnv* env, jobject thiz, jlong socketHandle, jbyteArray data,
        jstring address, jint port) {
    auto* socket = getSocket(socketHandle);
    if (!socket) return JNI_FALSE;

    jbyte* bytes = env->GetByteArrayElements(data, nullptr);
    jint size = env->GetArrayLength(data);
    const char* addr = env->GetStringUTFChars(address, nullptr);

    if (!bytes || !addr) {
        if (bytes) env->ReleaseByteArrayElements(data, bytes, JNI_ABORT);
        if (addr) env->ReleaseStringUTFChars(address, addr);
        return JNI_FALSE;
    }

    bool result = socket->sendTo(reinterpret_cast<const uint8_t*>(bytes), size,
                                  std::string(addr), port);

    env->ReleaseByteArrayElements(data, bytes, JNI_ABORT);
    env->ReleaseStringUTFChars(address, addr);

    return result ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jbyteArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeReceiveFrom(
        JNIEnv* env, jobject thiz, jlong socketHandle, jint bufferSize) {
    auto* socket = getSocket(socketHandle);
    if (!socket) return nullptr;

    std::string senderAddress;
    uint16_t senderPort;

    auto data = socket->receiveFrom(bufferSize, senderAddress, senderPort);

    if (data.empty()) return nullptr;

    jbyteArray result = env->NewByteArray(data.size());
    env->SetByteArrayRegion(result, 0, data.size(),
                             reinterpret_cast<const jbyte*>(data.data()));

    return result;
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeCloseSocket(
        JNIEnv* env, jobject thiz, jlong socketHandle) {
    auto* socket = getSocket(socketHandle);
    if (socket) {
        socket->close();
        delete socket;
    }
}

// ============================================================
// 管线控制（Pipeline Control）- 对应 Rust FFI sb_bind/sb_connect/sb_pipeline_*
// ============================================================

// 管线网络状态（静态变量，单引擎场景）
static std::unique_ptr<soundbridge::UdpSocket> g_pipeline_socket;
static std::atomic<int> g_local_port{0};
static std::atomic<int> g_target_port{0};
static std::string g_target_address;

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeBind(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint port) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    int rc = sb_bind(engine, static_cast<uint16_t>(port));
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Rust sb_bind failed: %s", err ? err : "unknown error");
        return -1;
    }
    LOGI("Rust engine bound to UDP port %d", port);
    return 0;
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    // 创建 UDP socket 并绑定
    g_pipeline_socket = std::make_unique<soundbridge::UdpSocket>();
    if (!g_pipeline_socket->bind(static_cast<uint16_t>(port))) {
        LOGE("Failed to bind UDP socket to port %d", port);
        g_pipeline_socket.reset();
        return -1;
    }
    g_local_port = static_cast<uint16_t>(port);
    LOGI("Pipeline bound to UDP port %d", port);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeConnect(
        JNIEnv* env, jobject thiz, jlong engineHandle, jstring address) {
    const char* addr = env->GetStringUTFChars(address, nullptr);
    if (!addr) return -1;

#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) {
        env->ReleaseStringUTFChars(address, addr);
        return -1;
    }

    int rc = sb_connect(engine, addr);
    env->ReleaseStringUTFChars(address, addr);

    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Rust sb_connect failed: %s", err ? err : "unknown error");
        return -1;
    }

    LOGI("Rust engine connected to %s", addr);
    return 0;
#else
    // Legacy C++ path
    auto* engine = getEngine(engineHandle);
    if (!engine) {
        env->ReleaseStringUTFChars(address, addr);
        return -1;
    }

    // 解析 "ip:port" 格式
    std::string addrStr(addr);
    env->ReleaseStringUTFChars(address, addr);

    size_t colonPos = addrStr.find_last_of(':');
    if (colonPos == std::string::npos) {
        LOGE("Invalid address format (expected ip:port): %s", addrStr.c_str());
        return -1;
    }

    g_target_address = addrStr.substr(0, colonPos);
    std::string portStr = addrStr.substr(colonPos + 1);

    // 使用 strtol 替代 stoi 避免异常崩溃
    char* endPtr = nullptr;
    long port = strtol(portStr.c_str(), &endPtr, 10);
    if (endPtr == portStr.c_str() || *endPtr != '\0' || port <= 0 || port > 65535) {
        LOGE("Invalid port number: %s", portStr.c_str());
        return -1;
    }
    g_target_port = static_cast<uint16_t>(port);

    LOGI("Pipeline target set to %s:%d", g_target_address.c_str(), g_target_port);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetLocalPort(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return 0;
    uint16_t port = 0;
    if (sb_local_port(engine, &port) != 0) return 0;
    return static_cast<jint>(port);
#else
    return static_cast<jint>(g_local_port);
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineStart(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    int rc = sb_pipeline_start(engine);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Rust sb_pipeline_start failed: %s", err ? err : "unknown error");
        return -1;
    }
    LOGI("Rust pipeline started");
    return 0;
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    if (engine->start()) {
        LOGI("Pipeline started");
        return 0;
    }
    LOGE("Failed to start pipeline");
    return -1;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineStop(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    int rc = sb_pipeline_stop(engine);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Rust sb_pipeline_stop failed: %s", err ? err : "unknown error");
        return -1;
    }
    LOGI("Rust pipeline stopped");
    return 0;
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    engine->stop();
    LOGI("Pipeline stopped");
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineState(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    int state = -1;
    int rc = sb_pipeline_state(engine, &state);
    if (rc != 0) return -1;
    return static_cast<jint>(state);
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1; // Error

    auto state = engine->getState();
    // 映射 AudioEngineState 到 int: 0=Stopped, 1=Running, 2=Error
    switch (state) {
        case soundbridge::AudioEngineState::IDLE:
        case soundbridge::AudioEngineState::INITIALIZED:
        case soundbridge::AudioEngineState::STOPPED:
            return 0; // Stopped
        case soundbridge::AudioEngineState::RUNNING:
            return 1; // Running
        case soundbridge::AudioEngineState::ERROR:
        default:
            return 2; // Error
    }
#endif
}

JNIEXPORT jstring JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetVersion(
        JNIEnv* env, jobject thiz) {
    return env->NewStringUTF("1.0.0");
}

// ============================================================
// 设备发现（存根实现 - Android 应使用 NsdManager）
// ============================================================

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDiscoveryCreate(
        JNIEnv* env, jobject thiz) {
    // 存根：返回 1 作为有效句柄
    // 真正的发现功能应在 Kotlin 层使用 Android NsdManager 实现
    LOGI("Discovery create (stub)");
    return 1;
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDiscoveryClose(
        JNIEnv* env, jobject thiz, jlong discoveryHandle) {
    LOGI("Discovery close (stub)");
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDiscoveryInit(
        JNIEnv* env, jobject thiz, jlong discoveryHandle) {
    LOGI("Discovery init (stub)");
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDiscoveryRegister(
        JNIEnv* env, jobject thiz, jlong discoveryHandle, jstring name, jint port) {
    LOGI("Discovery register (stub)");
    return 0; // OK
}

JNIEXPORT jobjectArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDiscoveryFindDevices(
        JNIEnv* env, jobject thiz, jlong discoveryHandle) {
    // 返回空数组 - 真正的发现应在 Kotlin NsdManager 层实现
    LOGI("Discovery find devices (stub - use NsdManager)");
    jclass stringClass = env->FindClass("java/lang/String");
    return env->NewObjectArray(0, stringClass, nullptr);
}

// ============================================================
// 连接方式管理（薄触发器 - 真实逻辑在 Kotlin 管理器中）
// ============================================================
// 架构说明：
//   JNI 函数仅作为薄触发器（thin triggers），更新静态状态变量。
//   真正的平台连接逻辑由 Kotlin 管理器实现：
//   - HotspotManager: WiFi P2P (WifiP2pManager)
//   - AdbManager: ADB reverse port forwarding (Runtime.exec)
//   - BluetoothManager: RFCOMM server socket
//   AudioService 同时调用 JNI stub + Kotlin 管理器，保持状态同步。
// ============================================================

// 热点/ADB/蓝牙状态（原子变量，薄触发器模式）
// 真实状态由 Kotlin HotspotManager/AdbManager/BluetoothManager 的 StateFlow 管理
static std::atomic<int> g_hotspot_state{0}; // 0=Idle, 1=Creating, 2=Active, 3=Error
static std::atomic<int> g_adb_state{0};     // 0=Idle, 1=Connecting, 2=Connected, 3=Error
static std::atomic<int> g_bt_state{0};      // 0=Idle, 1=Listening, 2=Connected, 3=Error

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeHotspotCreate(
        JNIEnv* env, jobject thiz, jlong engineHandle, jstring ssid, jstring password, jint channel) {
    LOGI("Hotspot create (thin trigger): channel=%d - real logic in HotspotManager", channel);
    g_hotspot_state = 2; // Active (thin trigger always succeeds, real state from Kotlin)
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeHotspotDestroy(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    LOGI("Hotspot destroy (stub)");
    g_hotspot_state = 0; // Idle
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeHotspotState(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    return g_hotspot_state;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeHotspotSetState(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint state) {
    LOGI("Hotspot set state: %d", state);
    g_hotspot_state = state;
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeAdbSetupPortForward(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint localPort, jint remotePort) {
    LOGI("ADB setup port forward (thin trigger): %d -> %d - real logic in AdbManager", localPort, remotePort);
    g_adb_state = 2; // Connected (thin trigger, real state from Kotlin)
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeAdbState(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    return g_adb_state;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeAdbSetState(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint state) {
    LOGI("ADB set state: %d", state);
    g_adb_state = state;
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeBtInit(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    LOGI("Bluetooth init (thin trigger) - real logic in BluetoothManager");
    g_bt_state = 2; // Ready (thin trigger, real state from Kotlin)
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeBtState(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    return g_bt_state;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeBtSetState(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint state) {
    LOGI("Bluetooth set state: %d", state);
    g_bt_state = state;
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetExclusiveMode(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean exclusive) {
    LOGI("Set exclusive mode: %s", exclusive ? "true" : "false");
    // 存根：Android 不使用 WASAPI 独占模式
    return 0; // OK
}

// ============================================================
// 安全/加密（DTLS/SRTP 存根实现）
// ============================================================

static std::atomic<bool> g_encryption_enabled{false};
static std::mutex g_srtp_mutex;
static uint8_t g_srtp_master_key[16] = {0};
static uint8_t g_srtp_master_salt[14] = {0};

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEncryptionEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled,
        jbyteArray masterKey, jbyteArray masterSalt) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    if (enabled) {
        if (!masterKey || !masterSalt) {
            LOGE("Encryption enabled but masterKey or masterSalt is null");
            return -1;
        }
        jint keyLen = env->GetArrayLength(masterKey);
        jint saltLen = env->GetArrayLength(masterSalt);
        if (keyLen != 16 || saltLen != 14) {
            LOGE("Invalid key length: %d (expected 16), salt length: %d (expected 14)",
                 keyLen, saltLen);
            return -1;
        }

        jbyte* key = env->GetByteArrayElements(masterKey, nullptr);
        jbyte* salt = env->GetByteArrayElements(masterSalt, nullptr);
        if (!key || !salt) {
            if (key) env->ReleaseByteArrayElements(masterKey, key, JNI_ABORT);
            if (salt) env->ReleaseByteArrayElements(masterSalt, salt, JNI_ABORT);
            return -1;
        }

        int rc = sb_enable_encryption(engine,
            reinterpret_cast<const uint8_t*>(key),
            reinterpret_cast<const uint8_t*>(salt));
        env->ReleaseByteArrayElements(masterKey, key, JNI_ABORT);
        env->ReleaseByteArrayElements(masterSalt, salt, JNI_ABORT);

        if (rc != 0) {
            const char* err = sb_last_error();
            LOGE("Rust sb_enable_encryption failed: %s", err ? err : "unknown error");
            return -1;
        }
        LOGI("Encryption enabled via Rust FFI");
    } else {
        int rc = sb_disable_encryption(engine);
        if (rc != 0) {
            const char* err = sb_last_error();
            LOGE("Rust sb_disable_encryption failed: %s", err ? err : "unknown error");
            return -1;
        }
        LOGI("Encryption disabled via Rust FFI");
    }
    return 0;
#else
    // Legacy C++ stub path
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    if (enabled) {
        if (!masterKey || !masterSalt) {
            LOGE("Encryption enabled but masterKey or masterSalt is null");
            return -1;
        }
        jint keyLen = env->GetArrayLength(masterKey);
        jint saltLen = env->GetArrayLength(masterSalt);
        if (keyLen != 16 || saltLen != 14) {
            LOGE("Invalid key length: %d (expected 16), salt length: %d (expected 14)",
                 keyLen, saltLen);
            return -1;
        }

        jbyte* key = env->GetByteArrayElements(masterKey, nullptr);
        jbyte* salt = env->GetByteArrayElements(masterSalt, nullptr);
        if (!key || !salt) {
            if (key) env->ReleaseByteArrayElements(masterKey, key, JNI_ABORT);
            if (salt) env->ReleaseByteArrayElements(masterSalt, salt, JNI_ABORT);
            return -1;
        }

        {
            std::lock_guard<std::mutex> lock(g_srtp_mutex);
            memcpy(g_srtp_master_key, key, 16);
            memcpy(g_srtp_master_salt, salt, 14);
        }
        env->ReleaseByteArrayElements(masterKey, key, JNI_ABORT);
        env->ReleaseByteArrayElements(masterSalt, salt, JNI_ABORT);

        g_encryption_enabled.store(true);
        LOGI("Encryption enabled with SRTP keys (stub)");
    } else {
        g_encryption_enabled.store(false);
        {
            std::lock_guard<std::mutex> lock(g_srtp_mutex);
            memset(g_srtp_master_key, 0, sizeof(g_srtp_master_key));
            memset(g_srtp_master_salt, 0, sizeof(g_srtp_master_salt));
        }
        LOGI("Encryption disabled (stub)");
    }
    return 0; // OK
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeIsEncryptionEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;
    return sb_is_encrypted(engine);
#else
    if (!getEngine(engineHandle)) return -1; // error
    return g_encryption_enabled.load() ? 1 : 0;
#endif
}

// ============================================================
// 静音控制（Mute Control）
// ============================================================

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetMute(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint muted) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;

    int rc = sb_set_mute(engine, muted);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("Rust sb_set_mute failed: %s", err ? err : "unknown error");
        return -1;
    }
    LOGI("Mute state set: %d", muted);
    return 0;
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;
    LOGI("Mute state set (stub): %d", muted);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetMute(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    void* engine = getRustEngine(engineHandle);
    if (!engine) return -1;
    return sb_get_mute(engine);
#else
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;
    return 0;
#endif
}

// ============================================================
// 音质档位 / 均衡器 / 声道 / 自动挡
// ============================================================

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetAudioProfile(
        JNIEnv* env, jobject thiz, jint profile) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_audio_profile(static_cast<SbAudioProfile>(profile));
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_audio_profile failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("Audio profile set: %d", profile);
    return 0;
#else
    LOGI("Audio profile set (stub): %d", profile);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetAudioProfile(
        JNIEnv* env, jobject thiz) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    return static_cast<jint>(sb_get_audio_profile());
#else
    return 1; // Standard
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetChannels(
        JNIEnv* env, jobject thiz, jint channels) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_channels(static_cast<uint32_t>(channels));
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_channels failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("Channels set: %d", channels);
    return 0;
#else
    LOGI("Channels set (stub): %d", channels);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetChannels(
        JNIEnv* env, jobject thiz) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    return static_cast<jint>(sb_get_channels());
#else
    return 1; // Mono
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEqBand(
        JNIEnv* env, jobject thiz, jint band, jfloat gainDb, jfloat q) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_eq_band(
        static_cast<uint32_t>(band),
        static_cast<float>(gainDb),
        static_cast<float>(q));
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_eq_band failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("EQ band %d set: gain=%.1fdB, q=%.2f", band, gainDb, q);
    return 0;
#else
    LOGI("EQ band set (stub): %d", band);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEqPreset(
        JNIEnv* env, jobject thiz, jint preset) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_eq_preset(static_cast<SbEqPreset>(preset));
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_eq_preset failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("EQ preset set: %d", preset);
    return 0;
#else
    LOGI("EQ preset set (stub): %d", preset);
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEqEnabled(
        JNIEnv* env, jobject thiz, jboolean enabled) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_eq_enabled(enabled ? 1 : 0);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_eq_enabled failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("EQ enabled: %s", enabled ? "true" : "false");
    return 0;
#else
    LOGI("EQ enabled (stub): %s", enabled ? "true" : "false");
    return 0;
#endif
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetAutoProfileEnabled(
        JNIEnv* env, jobject thiz, jboolean enabled) {
#ifdef SOUNDBRIDGE_USE_RUST_FFI
    int rc = sb_set_auto_profile_enabled(enabled ? 1 : 0);
    if (rc != 0) {
        const char* err = sb_last_error();
        LOGE("sb_set_auto_profile_enabled failed: %s", err ? err : "unknown");
        return -1;
    }
    LOGI("Auto profile enabled: %s", enabled ? "true" : "false");
    return 0;
#else
    LOGI("Auto profile enabled (stub): %s", enabled ? "true" : "false");
    return 0;
#endif
}

} // extern "C"
