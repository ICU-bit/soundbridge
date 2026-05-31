package com.soundbridge.audio

import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import com.soundbridge.MainActivity
import com.soundbridge.R
import com.soundbridge.SoundBridgeApp
import com.soundbridge.native.NativeAudioEngine
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.security.SecureRandom

class AudioService : Service() {

    private val binder = AudioServiceBinder()
    private var engineHandle: Long = 0L
    private var discoveryManager: DeviceDiscoveryManager? = null
    private var feedbackManager: FeedbackManager? = null

    private val prefs: SharedPreferences by lazy {
        getSharedPreferences("soundbridge_prefs", MODE_PRIVATE)
    }

    // === 平台连接管理器（Real implementations）===
    private var hotspotManager: HotspotManager? = null
    private var adbManager: AdbManager? = null
    private var bluetoothManager: BluetoothManager? = null

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

    // === Hotspot (WiFi Direct) state ===
    val hotspotState: StateFlow<HotspotManager.HotspotState>
        get() = hotspotManager?.state ?: MutableStateFlow(HotspotManager.HotspotState.Idle)

    // === ADB state ===
    val adbState: StateFlow<AdbManager.AdbState>
        get() = adbManager?.state ?: MutableStateFlow(AdbManager.AdbState.Disconnected)

    // === Bluetooth state ===
    val bluetoothState: StateFlow<BluetoothManager.BluetoothState>
        get() = bluetoothManager?.state ?: MutableStateFlow(BluetoothManager.BluetoothState.Idle)

    private var btUdpBridge: BluetoothUdpBridge? = null

    /** 用户反馈状态 */
    val feedbackState: StateFlow<FeedbackManager.FeedbackState>
        get() = feedbackManager?.feedbackState ?: MutableStateFlow(FeedbackManager.FeedbackState.Idle)

    /** 错误消息 */
    val errorMessage: StateFlow<String?>
        get() = feedbackManager?.errorMessage ?: MutableStateFlow(null)

    /** 加密状态 */
    enum class EncryptionState {
        DISABLED, ENABLED
    }

    private val _encryptionState = MutableStateFlow(EncryptionState.DISABLED)
    val encryptionState: StateFlow<EncryptionState> = _encryptionState

    /** SRTP 主密钥（16 字节），启用加密时生成 */
    private var srtpMasterKey: ByteArray? = null
    /** SRTP 主盐值（14 字节），启用加密时生成 */
    private var srtpMasterSalt: ByteArray? = null

    enum class ConnectionState {
        DISCONNECTED, CONNECTING, CONNECTED
    }

    enum class ReconnectState {
        IDLE, RECONNECTING, FAILED
    }

    // === Auto-reconnect ===
    private val _reconnectState = MutableStateFlow(ReconnectState.IDLE)
    val reconnectState: StateFlow<ReconnectState> = _reconnectState

    private val _reconnectAttempt = MutableStateFlow(0)
    val reconnectAttempt: StateFlow<Int> = _reconnectAttempt

    private var autoReconnectEnabled = true
    private var reconnectJob: Job? = null
    private val reconnectScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var lastAddress: String = ""
    private var lastPort: Int = 0
    private var userDisconnected = false

    inner class AudioServiceBinder : Binder() {
        fun getService(): AudioService = this@AudioService
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onCreate() {
        super.onCreate()
        initializeEngine()
        feedbackManager = FeedbackManager(this)
        discoveryManager = DeviceDiscoveryManager(this)
        hotspotManager = HotspotManager(this)
        adbManager = AdbManager()
        bluetoothManager = BluetoothManager(this).apply {
            listener = { deviceAddress -> connectViaBluetooth(deviceAddress) }
        }
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

        val savedMode = prefs.getInt("audio_mode", 0)
        NativeAudioEngine.nativeSetAudioMode(engineHandle, savedMode)

        val savedPcVolume = prefs.getFloat("mix_pc_volume", 0.5f)
        val savedPhoneVolume = prefs.getFloat("mix_phone_volume", 0.5f)
        NativeAudioEngine.nativeSetMixRatio(engineHandle, savedPcVolume, savedPhoneVolume)
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
        userDisconnected = true
        cancelReconnect()
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
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.ENGINE_NOT_INITIALIZED)
            return
        }
        userDisconnected = false
        lastAddress = address
        lastPort = port
        _connectionState.value = ConnectionState.CONNECTING
        feedbackManager?.onConnectionStarted()

        // 绑定本地 UDP 端口（0 = 自动分配）
        val bindResult = NativeAudioEngine.nativeBind(engineHandle, 0)
        if (bindResult != 0) {
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.BIND_FAILED)
            scheduleReconnect()
            return
        }

        // 设置目标地址
        val connectResult = NativeAudioEngine.nativeConnect(engineHandle, "$address:$port")
        if (connectResult != 0) {
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.CONNECT_FAILED)
            scheduleReconnect()
            return
        }

        // 启动采集和播放
        NativeAudioEngine.nativeStart(engineHandle)

        // 启动管线（采集→编码→发送 / 接收→解码→播放）
        val pipelineResult = NativeAudioEngine.nativePipelineStart(engineHandle)
        if (pipelineResult == 0) {
            _connectionState.value = ConnectionState.CONNECTED
            feedbackManager?.onConnectionSuccess()
            cancelReconnect()
        } else {
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.PIPELINE_FAILED)
            scheduleReconnect()
        }
    }

    fun disconnect() {
        userDisconnected = true
        cancelReconnect()
        btUdpBridge?.stop()
        btUdpBridge = null
        if (engineHandle != 0L) {
            NativeAudioEngine.nativePipelineStop(engineHandle)
            NativeAudioEngine.nativeStop(engineHandle)
        }
        _connectionState.value = ConnectionState.DISCONNECTED
        feedbackManager?.onDisconnected()
    }

    /** 开始扫描设备 */
    fun startDeviceDiscovery() {
        discoveryManager?.startDiscovery()
    }

    /** 停止扫描设备 */
    fun stopDeviceDiscovery() {
        discoveryManager?.stopDiscovery()
    }

    /** 设置音频模式并持久化 */
    fun setAudioMode(mode: Int) {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeSetAudioMode(engineHandle, mode)
        }
        prefs.edit().putInt("audio_mode", mode).apply()
    }

    /** 设置混音比例（0.0=全PC，1.0=全手机，0.5=均衡）并持久化 */
    fun setMixRatio(pcVolume: Float, phoneVolume: Float) {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeSetMixRatio(engineHandle, pcVolume, phoneVolume)
        }
        prefs.edit()
            .putFloat("mix_pc_volume", pcVolume)
            .putFloat("mix_phone_volume", phoneVolume)
            .apply()
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

    /** 设置静音状态 */
    fun setMute(muted: Boolean) {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeSetMute(engineHandle, if (muted) 1 else 0)
        }
    }

    /** 获取静音状态。引擎未初始化时返回 false。 */
    fun isMuted(): Boolean {
        if (engineHandle == 0L) return false
        return NativeAudioEngine.nativeGetMute(engineHandle) == 1
    }

    /** 启用 DTLS/SRTP 加密传输。管线运行中不允许修改。 */
    fun enableEncryption() {
        if (engineHandle == 0L) return

        // 生成随机 SRTP 主密钥（16 字节）和主盐值（14 字节）
        val random = SecureRandom()
        val key = ByteArray(16)
        val salt = ByteArray(14)
        random.nextBytes(key)
        random.nextBytes(salt)

        val result = NativeAudioEngine.nativeSetEncryptionEnabled(engineHandle, true, key, salt)
        if (result == 0) {
            srtpMasterKey = key
            srtpMasterSalt = salt
            _encryptionState.value = EncryptionState.ENABLED
        }
    }

    /** 禁用加密传输。管线运行中不允许修改。 */
    fun disableEncryption() {
        if (engineHandle == 0L) return

        val result = NativeAudioEngine.nativeSetEncryptionEnabled(engineHandle, false, null, null)
        if (result == 0) {
            srtpMasterKey = null
            srtpMasterSalt = null
            _encryptionState.value = EncryptionState.DISABLED
        }
    }

    /** 切换加密状态 */
    fun toggleEncryption() {
        if (_encryptionState.value == EncryptionState.ENABLED) {
            disableEncryption()
        } else {
            enableEncryption()
        }
    }

    // ============================================================
    // WiFi Direct (Hotspot) 管理
    // ============================================================

    /**
     * 创建 WiFi Direct 热点组。
     * 会同时调用 JNI stub（状态同步）和真正的 Kotlin HotspotManager。
     */
    fun createHotspot(ssid: String = "", password: String = "", channel: Int = 0) {
        // JNI stub call (for state sync with native layer)
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeHotspotCreate(engineHandle, ssid, password, channel)
        }
        // Real implementation
        hotspotManager?.createGroup(ssid, password, channel)
    }

    /**
     * 销毁 WiFi Direct 热点组。
     */
    fun destroyHotspot() {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeHotspotDestroy(engineHandle)
        }
        hotspotManager?.destroyGroup()
    }

    /**
     * 获取热点状态码（JNI 兼容）。
     */
    fun getHotspotStateCode(): Int = hotspotManager?.getStateCode() ?: 0

    // ============================================================
    // ADB 端口转发管理
    // ============================================================

    /**
     * 设置 ADB 反向端口转发。
     * 会同时调用 JNI stub 和真正的 Kotlin AdbManager。
     */
    fun setupAdbPortForward(localPort: Int, remotePort: Int) {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeAdbSetupPortForward(engineHandle, localPort, remotePort)
        }
        adbManager?.setupPortForward(localPort, remotePort)
    }

    /**
     * 检查 ADB 连接状态。
     */
    fun checkAdbConnection() {
        adbManager?.checkConnection()
    }

    /**
     * 断开 ADB 端口转发。
     */
    fun disconnectAdb() {
        adbManager?.disconnect()
    }

    /**
     * 获取 ADB 状态码（JNI 兼容）。
     */
    fun getAdbStateCode(): Int = adbManager?.getStateCode() ?: 0

    // ============================================================
    // 蓝牙管理
    // ============================================================

    /**
     * Called when a Bluetooth RFCOMM connection is accepted.
     * Starts the audio pipeline bound to a local port.
     *
     * @param deviceAddress the MAC address of the connected Bluetooth device
     */
    fun connectViaBluetooth(deviceAddress: String) {
        if (engineHandle == 0L) {
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.ENGINE_NOT_INITIALIZED)
            return
        }
        _connectionState.value = ConnectionState.CONNECTING
        feedbackManager?.onConnectionStarted()

        val bindResult = NativeAudioEngine.nativeBind(engineHandle, 0)
        if (bindResult != 0) {
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.BIND_FAILED)
            return
        }

        val socket = bluetoothManager?.getConnectedSocket()
        if (socket == null) {
            Log.e(TAG, "No Bluetooth socket available for bridge")
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.CONNECT_FAILED, "蓝牙连接不可用")
            return
        }

        val localPort = NativeAudioEngine.nativeGetLocalPort(engineHandle)
        val bridge = BluetoothUdpBridge(
            btInput = socket.inputStream,
            btOutput = socket.outputStream,
            localUdpPort = localPort
        )
        btUdpBridge = bridge
        bridge.start(
            remoteAddress = java.net.InetAddress.getByName("127.0.0.1"),
            remotePort = localPort
        )

        NativeAudioEngine.nativeStart(engineHandle)
        val pipelineResult = NativeAudioEngine.nativePipelineStart(engineHandle)
        if (pipelineResult == 0) {
            _connectionState.value = ConnectionState.CONNECTED
            feedbackManager?.onConnectionSuccess()
            Log.i(TAG, "Audio pipeline started via Bluetooth from $deviceAddress")
        } else {
            bridge.stop()
            btUdpBridge = null
            _connectionState.value = ConnectionState.DISCONNECTED
            feedbackManager?.onConnectionFailed(FeedbackManager.ErrorCode.PIPELINE_FAILED)
        }
    }

    /**
     * 启动蓝牙 RFCOMM 服务端监听。
     * 会同时调用 JNI stub 和真正的 Kotlin BluetoothManager。
     */
    fun startBluetoothListening(deviceName: String = "SoundBridge") {
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeBtInit(engineHandle)
        }
        bluetoothManager?.startListening(deviceName)
    }

    /**
     * 停止蓝牙监听。
     */
    fun stopBluetoothListening() {
        bluetoothManager?.stopListening()
    }

    /**
     * 获取蓝牙状态码（JNI 兼容）。
     */
    fun getBluetoothStateCode(): Int = bluetoothManager?.getStateCode() ?: 0

    /**
     * 清除错误消息
     */
    fun clearError() {
        feedbackManager?.clearError()
    }

    /**
     * 清除超时状态
     */
    fun clearTimeout() {
        feedbackManager?.clearTimeout()
    }

    // ============================================================
    // Auto-reconnect with exponential backoff
    // ============================================================

    private fun scheduleReconnect() {
        if (!autoReconnectEnabled || userDisconnected || lastAddress.isEmpty()) return
        if (reconnectJob?.isActive == true) return

        reconnectJob = reconnectScope.launch {
            _reconnectState.value = ReconnectState.RECONNECTING
            var attempt = 0
            while (attempt < MAX_RECONNECT_ATTEMPTS && autoReconnectEnabled && !userDisconnected) {
                attempt++
                _reconnectAttempt.value = attempt
                val delayMs = (INITIAL_DELAY_MS * (1L shl (attempt - 1))).coerceAtMost(MAX_DELAY_MS)
                Log.i(TAG, "Reconnect attempt $attempt/$MAX_RECONNECT_ATTEMPTS in ${delayMs}ms")
                delay(delayMs)

                if (!autoReconnectEnabled || userDisconnected) break

                withContext(Dispatchers.Main) {
                    connectToServer(lastAddress, lastPort)
                }

                // 等待连接结果
                delay(2000)
                if (_connectionState.value == ConnectionState.CONNECTED) {
                    Log.i(TAG, "Reconnect succeeded on attempt $attempt")
                    _reconnectState.value = ReconnectState.IDLE
                    _reconnectAttempt.value = 0
                    return@launch
                }
            }
            if (_connectionState.value != ConnectionState.CONNECTED) {
                Log.w(TAG, "Reconnect failed after $MAX_RECONNECT_ATTEMPTS attempts")
                _reconnectState.value = ReconnectState.FAILED
            }
        }
    }

    private fun cancelReconnect() {
        reconnectJob?.cancel()
        reconnectJob = null
        _reconnectState.value = ReconnectState.IDLE
        _reconnectAttempt.value = 0
    }

    /** 启用/禁用自动重连 */
    fun setAutoReconnectEnabled(enabled: Boolean) {
        autoReconnectEnabled = enabled
        if (!enabled) cancelReconnect()
    }

    /** 获取自动重连是否启用 */
    fun isAutoReconnectEnabled(): Boolean = autoReconnectEnabled

    /** 手动触发重连（重置失败状态后重试） */
    fun manualReconnect() {
        cancelReconnect()
        userDisconnected = false
        if (lastAddress.isNotEmpty()) {
            connectToServer(lastAddress, lastPort)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        userDisconnected = true
        cancelReconnect()
        reconnectScope.cancel()
        btUdpBridge?.stop()
        btUdpBridge = null
        // Release platform managers
        bluetoothManager?.release()
        bluetoothManager = null
        adbManager?.release()
        adbManager = null
        hotspotManager?.release()
        hotspotManager = null
        discoveryManager?.release()
        discoveryManager = null
        feedbackManager?.release()
        feedbackManager = null
        if (engineHandle != 0L) {
            NativeAudioEngine.nativeRelease(engineHandle)
            engineHandle = 0L
        }
    }

    companion object {
        private const val TAG = "AudioService"
        const val ACTION_START = "com.soundbridge.ACTION_START"
        const val ACTION_STOP = "com.soundbridge.ACTION_STOP"
        const val NOTIFICATION_ID = 1001
        const val MAX_RECONNECT_ATTEMPTS = 5
        const val INITIAL_DELAY_MS = 1000L
        const val MAX_DELAY_MS = 16000L
    }
}
