#include <jni.h>
#include <android/log.h>
#include <string>

#include "include/audio_engine.h"
#include "include/opus_codec.h"
#include "include/udp_socket.h"

#define LOG_TAG "SoundBridge_JNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

static soundbridge::AudioEngine* getEngine(jlong handle) {
    return reinterpret_cast<soundbridge::AudioEngine*>(handle);
}

static soundbridge::OpusEncoderWrapper* getEncoder(jlong handle) {
    return reinterpret_cast<soundbridge::OpusEncoderWrapper*>(handle);
}

static soundbridge::OpusDecoderWrapper* getDecoder(jlong handle) {
    return reinterpret_cast<soundbridge::OpusDecoderWrapper*>(handle);
}

static soundbridge::UdpSocket* getSocket(jlong handle) {
    return reinterpret_cast<soundbridge::UdpSocket*>(handle);
}

extern "C" {

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeInit(
        JNIEnv* env, jobject thiz, jint sampleRate, jint channels, jint bufferSize) {
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
    auto* engine = getEngine(engineHandle);
    if (engine) {
        engine->release();
        delete engine;
    }
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
    auto* encoder = new soundbridge::OpusEncoderWrapper(48000, 1, bitrate, complexity);

    if (!encoder->initialize()) {
        LOGE("Failed to initialize Opus encoder");
        delete encoder;
        return 0;
    }

    LOGI("Opus encoder created: %dbps, complexity=%d", bitrate, complexity);
    return reinterpret_cast<jlong>(encoder);
}

JNIEXPORT jbyteArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeEncodeFrame(
        JNIEnv* env, jobject thiz, jlong encoderHandle, jbyteArray pcmData, jint frameSize) {
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
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeReleaseEncoder(
        JNIEnv* env, jobject thiz, jlong encoderHandle) {
    auto* encoder = getEncoder(encoderHandle);
    if (encoder) {
        encoder->release();
        delete encoder;
    }
}

JNIEXPORT jlong JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeCreateDecoder(
        JNIEnv* env, jobject thiz) {
    auto* decoder = new soundbridge::OpusDecoderWrapper(48000, 1);

    if (!decoder->initialize()) {
        LOGE("Failed to initialize Opus decoder");
        delete decoder;
        return 0;
    }

    LOGI("Opus decoder created");
    return reinterpret_cast<jlong>(decoder);
}

JNIEXPORT jbyteArray JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeDecodeFrame(
        JNIEnv* env, jobject thiz, jlong decoderHandle, jbyteArray opusData) {
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
}

JNIEXPORT void JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeReleaseDecoder(
        JNIEnv* env, jobject thiz, jlong decoderHandle) {
    auto* decoder = getDecoder(decoderHandle);
    if (decoder) {
        decoder->release();
        delete decoder;
    }
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
static uint16_t g_local_port = 0;
static std::string g_target_address;
static uint16_t g_target_port = 0;

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeBind(
        JNIEnv* env, jobject thiz, jlong engineHandle, jint port) {
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
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeConnect(
        JNIEnv* env, jobject thiz, jlong engineHandle, jstring address) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    const char* addr = env->GetStringUTFChars(address, nullptr);
    if (!addr) return -1;

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
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetLocalPort(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    return static_cast<jint>(g_local_port);
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineStart(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    if (engine->start()) {
        LOGI("Pipeline started");
        return 0;
    }
    LOGE("Failed to start pipeline");
    return -1;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineStop(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    auto* engine = getEngine(engineHandle);
    if (!engine) return -1;

    engine->stop();
    LOGI("Pipeline stopped");
    return 0;
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativePipelineState(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
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
// 连接方式管理（存根 - 平台层实现）
// ============================================================

// 热点状态（静态变量）
static int g_hotspot_state = 0; // 0=Idle, 1=Creating, 2=Active, 3=Error
static int g_adb_state = 0;     // 0=Idle, 1=Connecting, 2=Connected, 3=Error
static int g_bt_state = 0;      // 0=Idle, 1=Initializing, 2=Ready, 3=Error

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeHotspotCreate(
        JNIEnv* env, jobject thiz, jlong engineHandle, jstring ssid, jstring password, jint channel) {
    LOGI("Hotspot create (stub): channel=%d", channel);
    g_hotspot_state = 2; // Active
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
    LOGI("ADB setup port forward (stub): %d -> %d", localPort, remotePort);
    g_adb_state = 2; // Connected
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
    LOGI("Bluetooth init (stub)");
    g_bt_state = 2; // Ready
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

static bool g_encryption_enabled = false;
static uint8_t g_srtp_master_key[16] = {0};
static uint8_t g_srtp_master_salt[14] = {0};

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeSetEncryptionEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle, jboolean enabled,
        jbyteArray masterKey, jbyteArray masterSalt) {
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

        memcpy(g_srtp_master_key, key, 16);
        memcpy(g_srtp_master_salt, salt, 14);
        env->ReleaseByteArrayElements(masterKey, key, JNI_ABORT);
        env->ReleaseByteArrayElements(masterSalt, salt, JNI_ABORT);

        g_encryption_enabled = true;
        LOGI("Encryption enabled with SRTP keys (stub)");
    } else {
        g_encryption_enabled = false;
        memset(g_srtp_master_key, 0, sizeof(g_srtp_master_key));
        memset(g_srtp_master_salt, 0, sizeof(g_srtp_master_salt));
        LOGI("Encryption disabled (stub)");
    }
    return 0; // OK
}

JNIEXPORT jint JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeIsEncryptionEnabled(
        JNIEnv* env, jobject thiz, jlong engineHandle) {
    if (!getEngine(engineHandle)) return -1; // error
    return g_encryption_enabled ? 1 : 0;
}

} // extern "C"
