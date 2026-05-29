package com.soundbridge.audio

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class AudioCaptureManager {

    companion object {
        const val SAMPLE_RATE = 48000
        const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT
        const val FRAME_SIZE = 960
    }

    private var audioRecord: AudioRecord? = null
    private var captureJob: Job? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val _isCapturing = MutableStateFlow(false)
    val isCapturing: StateFlow<Boolean> = _isCapturing

    private val _audioLevel = MutableStateFlow(0f)
    val audioLevel: StateFlow<Float> = _audioLevel

    var onAudioDataCaptured: ((ByteArray, Int) -> Unit)? = null

    private val bufferSize = AudioRecord.getMinBufferSize(SAMPLE_RATE, CHANNEL_CONFIG, AUDIO_FORMAT)

    @SuppressLint("MissingPermission")
    fun startCapture() {
        if (_isCapturing.value) return

        try {
            audioRecord = AudioRecord(
                MediaRecorder.AudioSource.VOICE_COMMUNICATION,
                SAMPLE_RATE,
                CHANNEL_CONFIG,
                AUDIO_FORMAT,
                bufferSize * 2
            )

            if (audioRecord?.state != AudioRecord.STATE_INITIALIZED) {
                return
            }

            audioRecord?.startRecording()
            _isCapturing.value = true

            captureJob = scope.launch {
                val buffer = ByteArray(FRAME_SIZE * 2)
                while (isActive && _isCapturing.value) {
                    val readSize = audioRecord?.read(buffer, 0, buffer.size) ?: 0
                    if (readSize > 0) {
                        val level = calculateAudioLevel(buffer, readSize)
                        _audioLevel.value = level
                        onAudioDataCaptured?.invoke(buffer.copyOf(readSize), readSize)
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun stopCapture() {
        _isCapturing.value = false
        captureJob?.cancel()
        captureJob = null

        try {
            audioRecord?.stop()
            audioRecord?.release()
        } catch (e: Exception) {
            e.printStackTrace()
        }
        audioRecord = null
        _audioLevel.value = 0f
    }

    private fun calculateAudioLevel(buffer: ByteArray, size: Int): Float {
        var sum = 0L
        for (i in 0 until size step 2) {
            val sample = (buffer[i].toInt() and 0xFF) or (buffer[i + 1].toInt() shl 8)
            sum += sample.toLong() * sample.toLong()
        }
        val rms = Math.sqrt(sum.toDouble() / (size / 2))
        return (rms / Short.MAX_VALUE).toFloat().coerceIn(0f, 1f)
    }

    fun release() {
        stopCapture()
        scope.cancel()
    }
}
