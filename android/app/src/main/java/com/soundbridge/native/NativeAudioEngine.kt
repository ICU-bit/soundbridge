package com.soundbridge.native

object NativeAudioEngine {

    init {
        System.loadLibrary("soundbridge_native")
    }

    // === 引擎生命周期 ===
    external fun nativeInit(sampleRate: Int, channels: Int, bufferSize: Int): Long
    external fun nativeStart(engineHandle: Long): Boolean
    external fun nativeStop(engineHandle: Long)
    external fun nativeRelease(engineHandle: Long)
    external fun nativeGetAudioLevel(engineHandle: Long): Float

    // === 音频处理 ===
    external fun nativeSetEchoCancellationEnabled(engineHandle: Long, enabled: Boolean)
    external fun nativeSetNoiseSuppressionEnabled(engineHandle: Long, enabled: Boolean)
    external fun nativeSetGainControlEnabled(engineHandle: Long, enabled: Boolean)
    external fun nativeSetAudioMode(engineHandle: Long, mode: Int): Int
    external fun nativeGetAudioMode(engineHandle: Long): Int
    external fun nativeSetMixRatio(engineHandle: Long, pcVolume: Float, phoneVolume: Float): Int
    external fun nativeGetMixRatio(engineHandle: Long): FloatArray?

    // === 编解码 ===
    external fun nativeCreateEncoder(bitrate: Int, complexity: Int): Long
    external fun nativeEncodeFrame(encoderHandle: Long, pcmData: ByteArray, frameSize: Int): ByteArray?
    external fun nativeReleaseEncoder(encoderHandle: Long)
    external fun nativeCreateDecoder(): Long
    external fun nativeDecodeFrame(decoderHandle: Long, opusData: ByteArray): ByteArray?
    external fun nativeReleaseDecoder(decoderHandle: Long)

    // === 网络 ===
    external fun nativeCreateUdpSocket(port: Int): Long
    external fun nativeSendTo(socketHandle: Long, data: ByteArray, address: String, port: Int): Boolean
    external fun nativeReceiveFrom(socketHandle: Long, bufferSize: Int): ByteArray?
    external fun nativeCloseSocket(socketHandle: Long)

    // === 管线控制（对应 Rust FFI sb_bind/sb_connect/sb_pipeline_*）===
    external fun nativeBind(engineHandle: Long, port: Int): Int
    external fun nativeConnect(engineHandle: Long, address: String): Int
    external fun nativeGetLocalPort(engineHandle: Long): Int
    external fun nativePipelineStart(engineHandle: Long): Int
    external fun nativePipelineStop(engineHandle: Long): Int
    external fun nativePipelineState(engineHandle: Long): Int
    external fun nativeGetVersion(): String

    // === 设备发现（对应 Rust FFI sb_discovery_*）===
    external fun nativeDiscoveryCreate(): Long
    external fun nativeDiscoveryClose(discoveryHandle: Long)
    external fun nativeDiscoveryInit(discoveryHandle: Long): Int
    external fun nativeDiscoveryRegister(discoveryHandle: Long, name: String, port: Int): Int
    external fun nativeDiscoveryFindDevices(discoveryHandle: Long): Array<String>?

    // === 连接方式管理（对应 Rust FFI sb_hotspot_*/sb_adb_*/sb_bt_*）===
    external fun nativeHotspotCreate(engineHandle: Long, ssid: String, password: String, channel: Int): Int
    external fun nativeHotspotDestroy(engineHandle: Long): Int
    external fun nativeHotspotState(engineHandle: Long): Int
    external fun nativeHotspotSetState(engineHandle: Long, state: Int): Int
    external fun nativeAdbSetupPortForward(engineHandle: Long, localPort: Int, remotePort: Int): Int
    external fun nativeAdbState(engineHandle: Long): Int
    external fun nativeAdbSetState(engineHandle: Long, state: Int): Int
    external fun nativeBtInit(engineHandle: Long): Int
    external fun nativeBtState(engineHandle: Long): Int
    external fun nativeBtSetState(engineHandle: Long, state: Int): Int
    external fun nativeSetExclusiveMode(engineHandle: Long, exclusive: Boolean): Int

    // === 安全/加密（DTLS/SRTP）===
    external fun nativeSetEncryptionEnabled(engineHandle: Long, enabled: Boolean, masterKey: ByteArray?, masterSalt: ByteArray?): Int
    external fun nativeIsEncryptionEnabled(engineHandle: Long): Int

    // === 静音控制 ===
    external fun nativeSetMute(engineHandle: Long, muted: Int): Int
    external fun nativeGetMute(engineHandle: Long): Int

    // === 音质档位 ===
    external fun nativeSetAudioProfile(profile: Int): Int
    external fun nativeGetAudioProfile(): Int

    // === 声道控制 ===
    external fun nativeSetChannels(channels: Int): Int
    external fun nativeGetChannels(): Int

    // === 采样率控制 ===
    external fun nativeSetSampleRate(sampleRate: Int): Int
    external fun nativeGetSampleRate(): Int

    // === 码率控制 ===
    external fun nativeSetBitrate(bitrate: Int): Int
    external fun nativeGetBitrate(): Int

    // === 均衡器 ===
    external fun nativeSetEqBand(band: Int, gainDb: Float, q: Float): Int
    external fun nativeSetEqPreset(preset: Int): Int
    external fun nativeSetEqEnabled(enabled: Boolean): Int

    // === 自动挡 ===
    external fun nativeSetAutoProfileEnabled(enabled: Boolean): Int

    // ── Kotlin 包装方法 ──

    fun setAudioProfile(profile: AudioProfile) {
        nativeSetAudioProfile(profile.ordinal)
    }

    fun getAudioProfile(): AudioProfile {
        val ordinal = nativeGetAudioProfile()
        return AudioProfile.entries.getOrElse(ordinal) { AudioProfile.Standard }
    }

    fun setEqPreset(preset: EqPreset) {
        nativeSetEqPreset(preset.ordinal)
    }

    fun setEqEnabled(enabled: Boolean) {
        nativeSetEqEnabled(enabled)
    }

    fun setAutoProfileEnabled(enabled: Boolean) {
        nativeSetAutoProfileEnabled(enabled)
    }
}

/** 音质档位（与 Rust SbAudioProfile 枚举顺序一致） */
enum class AudioProfile(val label: String) {
    BandwidthSaving("节省带宽"),
    Standard("标准"),
    HighQuality("高质量"),
    Lossless("无损"),
    HighResolution("高解析度"),
    StudioMaster("录音室母带"),
    Auto("自动挡"),
    Custom("自定义")
}

/** 均衡器预设（与 Rust SbEqPreset 枚举顺序一致） */
enum class EqPreset(val label: String) {
    Flat("Flat"),
    Gaming("Gaming"),
    Music("Music"),
    Voice("Voice"),
    Bass("Bass"),
    Treble("Treble")
}
