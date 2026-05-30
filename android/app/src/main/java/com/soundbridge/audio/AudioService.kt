package com.soundbridge.audio

import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.soundbridge.MainActivity
import com.soundbridge.R
import com.soundbridge.SoundBridgeApp
import com.soundbridge.native.NativeAudioEngine
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class AudioService : Service() {

    private val binder = AudioServiceBinder()
    private var engineHandle: Long = 0L
    private var discoveryManager: DeviceDiscoveryManager? = null

    /** Engine handle for JNI calls. Returns 0L if not initialized. */
    val handle: Long get() = engineHandle

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    val connectionState: StateFlow<ConnectionState> = _connectionState

    private val _audioLevel = MutableStateFlow(0f)
    val audioLevel: StateFlow<Float> = _audioLevel

    /** 已发现的设备列表 */
    val discoveredDevices: StateFlow<List<DiscoveredDevice>>
        get() = discoveryManager?.discoveredDevices ?: MutableStateFlow(emptyList())

    /** 是否正在扫描 */
    val isScanning: StateFlow<Boolean>
        get() = discoveryManager?.isScanning ?: MutableStateFlow(false)

    enum class ConnectionState {
        DISCONNECTED, CONNECTING, CONNECTED
    }

    inner class AudioServiceBinder : Binder() {
        fun getService(): AudioService = this@AudioService
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onCreate() {
        super.onCreate()
        initializeEngine()
        discoveryManager = DeviceDiscoveryManager(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> startForegroundService()
            ACTION_STOP -> stopForegroundService()
        }
        return START_STICKY
    }

    private fun initializeEngine() {
        engineHandle = NativeAudioEngine.nativeInit(
            AudioCaptureManager.SAMPLE_RATE,
            1,
            AudioCaptureManager.FRAME_SIZE
        )
    }

    private fun startForegroundService() {
        val notification = createNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        _connectionState.value = ConnectionState.CONNECTING
    }

    private fun stopForegroundService() {
        _connectionState.value = ConnectionState.DISCONNECTED
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun createNotification(): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, SoundBridgeApp.NOTIFICATION_CHANNEL_ID)
            .setContentTitle("SoundBridge")
            .setContentText("Audio processing active")
            .setSmallIcon(R.drawable.ic_audio)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    fun connectToServer(address: String, port: Int) {
        if (engineHandle == 0L) {
            _connectionState.value = ConnectionState.DISCONNECTED
            return
        }
        _connectionState.value = ConnectionState.CONNECTING

        // 绑定本地 UDP 端口（0 = 自动分配）
        val bindResult = NativeAudioEngine.nativeBind(engineHandle, 0)
        if (bindResult != 0) {
            _connectionState.value = ConnectionState.DISCONNECTED
            return
        }

        // 设置目标地址
        val connectResult = NativeAudioEngine.nativeConnect(engineHandle, "$address:$port")
        if (connectResult != 0) {
            _connectionState.value = ConnectionState.DISCONNECTED
            return
        }

        // 启动采集和播放
        NativeAudioEngine.nativeStart(engineHandle)

        // 启动管线（采集→编码→发送 / 接收→解码→播放）
        val pipelineResult = NativeAudioEngine.nativePipelineStart(engineHandle)
        if (pipelineResult == 0) {
            _connectionState.value = ConnectionState.CONNECTED
        } else {
            _connectionState.value = ConnectionState.DISCONNECTED
        }
    }

    fun disconnect() {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativePipelineStop(engineHandle)
            NativeAudioEngine.nativeStop(engineHandle)
        }
        _connectionState.value = ConnectionState.DISCONNECTED
    }

    /** 开始扫描设备 */
    fun startDeviceDiscovery() {
        discoveryManager?.startDiscovery()
    }

    /** 停止扫描设备 */
    fun stopDeviceDiscovery() {
        discoveryManager?.stopDiscovery()
    }

    /** 设置混音比例（0.0=全PC，1.0=全手机，0.5=均衡） */
    fun setMixRatio(pcVolume: Float, phoneVolume: Float) {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeSetMixRatio(engineHandle, pcVolume, phoneVolume)
        }
    }

    /** 获取混音比例，返回 [pcVolume, phoneVolume]，失败返回 null */
    fun getMixRatio(): Pair<Float, Float>? {
        if (engineHandle == 0L) return null
        val result = NativeAudioEngine.nativeGetMixRatio(engineHandle) ?: return null
        if (result.size >= 2) {
            return Pair(result[0], result[1])
        }
        return null
    }

    override fun onDestroy() {
        super.onDestroy()
        discoveryManager?.release()
        discoveryManager = null
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeRelease(engineHandle)
            engineHandle = 0L
        }
    }

    companion object {
        const val ACTION_START = "com.soundbridge.ACTION_START"
        const val ACTION_STOP = "com.soundbridge.ACTION_STOP"
        const val NOTIFICATION_ID = 1001
    }
}
