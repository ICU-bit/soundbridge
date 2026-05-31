package com.soundbridge.audio

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * 用户反馈管理器：处理连接错误、超时提示、状态消息
 */
class FeedbackManager(private val context: Context) {

    private val handler = Handler(Looper.getMainLooper())

    private val _feedbackState = MutableStateFlow<FeedbackState>(FeedbackState.Idle)
    val feedbackState: StateFlow<FeedbackState> = _feedbackState

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private var timeoutRunnable: Runnable? = null
    private var connectionStartTime: Long = 0L

    companion object {
        private const val TAG = "FeedbackManager"
        private const val CONNECTION_TIMEOUT_MS = 15_000L // 15秒超时
    }

    sealed class FeedbackState {
        data object Idle : FeedbackState()
        data object Connecting : FeedbackState()
        data object Connected : FeedbackState()
        data class Error(val code: ErrorCode, val message: String) : FeedbackState()
        data class Timeout(val elapsedMs: Long) : FeedbackState()
    }

    enum class ErrorCode {
        ENGINE_NOT_INITIALIZED,
        BIND_FAILED,
        CONNECT_FAILED,
        PIPELINE_FAILED,
        CONNECTION_TIMEOUT,
        UNKNOWN
    }

    /**
     * 连接开始时调用，启动超时计时器
     */
    fun onConnectionStarted() {
        connectionStartTime = System.currentTimeMillis()
        _feedbackState.value = FeedbackState.Connecting
        _errorMessage.value = null

        // 启动超时检测
        cancelTimeout()
        timeoutRunnable = Runnable {
            val elapsed = System.currentTimeMillis() - connectionStartTime
            Log.w(TAG, "Connection timeout after ${elapsed}ms")
            _feedbackState.value = FeedbackState.Timeout(elapsed)
            _errorMessage.value = "连接超时，请检查网络和服务器地址"
        }
        handler.postDelayed(timeoutRunnable!!, CONNECTION_TIMEOUT_MS)
    }

    /**
     * 连接成功时调用
     */
    fun onConnectionSuccess() {
        cancelTimeout()
        _feedbackState.value = FeedbackState.Connected
        _errorMessage.value = null
        Log.i(TAG, "Connection established successfully")
    }

    /**
     * 连接失败时调用
     */
    fun onConnectionFailed(errorCode: ErrorCode, detail: String = "") {
        cancelTimeout()
        val message = getErrorMessage(errorCode, detail)
        _feedbackState.value = FeedbackState.Error(errorCode, message)
        _errorMessage.value = message
        Log.e(TAG, "Connection failed: $errorCode - $message")
    }

    /**
     * 断开连接时调用
     */
    fun onDisconnected() {
        cancelTimeout()
        _feedbackState.value = FeedbackState.Idle
        _errorMessage.value = null
    }

    /**
     * 清除错误消息
     */
    fun clearError() {
        _errorMessage.value = null
        if (_feedbackState.value is FeedbackState.Error) {
            _feedbackState.value = FeedbackState.Idle
        }
    }

    /**
     * 清除超时状态
     */
    fun clearTimeout() {
        if (_feedbackState.value is FeedbackState.Timeout) {
            _feedbackState.value = FeedbackState.Idle
        }
        _errorMessage.value = null
    }

    private fun cancelTimeout() {
        timeoutRunnable?.let { handler.removeCallbacks(it) }
        timeoutRunnable = null
    }

    private fun getErrorMessage(errorCode: ErrorCode, detail: String): String {
        return when (errorCode) {
            ErrorCode.ENGINE_NOT_INITIALIZED -> "音频引擎未初始化，请重启应用"
            ErrorCode.BIND_FAILED -> "无法绑定网络端口，请检查权限"
            ErrorCode.CONNECT_FAILED -> "连接服务器失败，请检查地址和端口"
            ErrorCode.PIPELINE_FAILED -> "音频管线启动失败，请重试"
            ErrorCode.CONNECTION_TIMEOUT -> "连接超时，请检查网络连接"
            ErrorCode.UNKNOWN -> if (detail.isNotEmpty()) detail else "发生未知错误"
        }
    }

    /**
     * 释放资源
     */
    fun release() {
        cancelTimeout()
    }
}
