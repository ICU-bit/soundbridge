package com.soundbridge.audio

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.Dispatchers

class AudioPlaybackManager {

    companion object {
        const val SAMPLE_RATE = 48000
        const val CHANNEL_CONFIG = AudioFormat.CHANNEL_OUT_MONO
        const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT
    }

    private var audioTrack: AudioTrack? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val audioDispatcher = Dispatchers.IO.limitedParallelism(1)

    private val _isPlaying = MutableStateFlow(false)
    val isPlaying: StateFlow<Boolean> = _isPlaying

    private val bufferSize = AudioTrack.getMinBufferSize(SAMPLE_RATE, CHANNEL_CONFIG, AUDIO_FORMAT)

    fun initialize() {
        val audioAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
            .build()

        val audioFormat = AudioFormat.Builder()
            .setSampleRate(SAMPLE_RATE)
            .setChannelMask(CHANNEL_CONFIG)
            .setEncoding(AUDIO_FORMAT)
            .build()

        audioTrack = AudioTrack.Builder()
            .setAudioAttributes(audioAttributes)
            .setAudioFormat(audioFormat)
            .setBufferSizeInBytes(bufferSize * 2)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
            .build()
    }

    fun startPlayback() {
        if (_isPlaying.value) return
        audioTrack?.play()
        _isPlaying.value = true
    }

    fun writeAudioData(data: ByteArray) {
        if (_isPlaying.value) {
            scope.launch(audioDispatcher) {
                audioTrack?.write(data, 0, data.size)
            }
        }
    }

    fun stopPlayback() {
        _isPlaying.value = false
        try {
            audioTrack?.pause()
            audioTrack?.flush()
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun release() {
        stopPlayback()
        scope.cancel()
        try {
            audioTrack?.release()
        } catch (e: Exception) {
            e.printStackTrace()
        }
        audioTrack = null
    }
}
