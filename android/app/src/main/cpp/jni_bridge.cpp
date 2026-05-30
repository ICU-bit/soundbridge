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

JNIEXPORT jstring JNICALL
Java_com_soundbridge_native_NativeAudioEngine_nativeGetVersion(
        JNIEnv* env, jobject thiz) {
    return env->NewStringUTF("1.0.0");
}

} // extern "C"
