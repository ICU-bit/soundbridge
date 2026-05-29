package com.soundbridge.native

object NativeAudioEngine {

    init {
        System.loadLibrary("soundbridge_native")
    }

    external fun nativeInit(sampleRate: Int, channels: Int, bufferSize: Int): Long

    external fun nativeStart(engineHandle: Long): Boolean

    external fun nativeStop(engineHandle: Long)

    external fun nativeRelease(engineHandle: Long)

    external fun nativeGetAudioLevel(engineHandle: Long): Float

    external fun nativeSetEchoCancellationEnabled(engineHandle: Long, enabled: Boolean)

    external fun nativeSetNoiseSuppressionEnabled(engineHandle: Long, enabled: Boolean)

    external fun nativeSetGainControlEnabled(engineHandle: Long, enabled: Boolean)

    external fun nativeCreateEncoder(bitrate: Int, complexity: Int): Long

    external fun nativeEncodeFrame(encoderHandle: Long, pcmData: ByteArray, frameSize: Int): ByteArray?

    external fun nativeReleaseEncoder(encoderHandle: Long)

    external fun nativeCreateDecoder(): Long

    external fun nativeDecodeFrame(decoderHandle: Long, opusData: ByteArray): ByteArray?

    external fun nativeReleaseDecoder(decoderHandle: Long)

    external fun nativeCreateUdpSocket(port: Int): Long

    external fun nativeSendTo(socketHandle: Long, data: ByteArray, address: String, port: Int): Boolean

    external fun nativeReceiveFrom(socketHandle: Long, bufferSize: Int): ByteArray?

    external fun nativeCloseSocket(socketHandle: Long)

    external fun nativeGetVersion(): String
}
