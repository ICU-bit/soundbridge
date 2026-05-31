package com.soundbridge.audio

import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.wifi.p2p.WifiP2pGroup
import android.net.wifi.p2p.WifiP2pManager
import android.os.Build
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.util.UUID

/**
 * WiFi Direct (WiFi P2P) hotspot manager.
 *
 * Creates a WiFi Direct group so the PC can connect to this device.
 * Uses [WifiP2pManager] to create/destroy the group and expose
 * connection info (group owner address, passphrase, network name).
 */
class HotspotManager(private val context: Context) {

    companion object {
        private const val TAG = "HotspotManager"
    }

    /** Hotspot state exposed via StateFlow. */
    sealed class HotspotState {
        data object Idle : HotspotState()
        data object Creating : HotspotState()
        data class Active(
            val ssid: String,
            val password: String,
            val groupOwnerAddress: String
        ) : HotspotState()
        data class Failed(val reason: String) : HotspotState()
        data object Destroyed : HotspotState()
    }

    private val _state = MutableStateFlow<HotspotState>(HotspotState.Idle)
    val state: StateFlow<HotspotState> = _state

    private val wifiP2pManager: WifiP2pManager? =
        context.getSystemService(Context.WIFI_P2P_SERVICE) as? WifiP2pManager
    private var channel: WifiP2pManager.Channel? = null
    private var receiver: BroadcastReceiver? = null
    private var isReceiverRegistered = false

    /** Whether WiFi P2P is available on this device. */
    val isAvailable: Boolean get() = wifiP2pManager != null

    /**
     * Create a WiFi Direct group.
     * Requires [android.Manifest.permission.ACCESS_FINE_LOCATION] and
     * [android.Manifest.permission.NEARBY_WIFI_DEVICES] (API 33+).
     *
     * @param ssid  desired SSID (used as device name hint, actual group name is system-determined)
     * @param password desired passphrase (8-63 chars, or auto-generated if blank)
     * @param channel WiFi channel hint (currently unused, reserved for future)
     */
    @SuppressLint("MissingPermission")
    fun createGroup(ssid: String = "", password: String = "", channel: Int = 0) {
        val mgr = wifiP2pManager
        if (mgr == null) {
            _state.value = HotspotState.Failed("WiFi P2P not available on this device")
            return
        }

        if (this.channel == null) {
            this.channel = mgr.initialize(context, Looper.getMainLooper(), null)
        }
        val ch = this.channel
        if (ch == null) {
            _state.value = HotspotState.Failed("Failed to initialize WiFi P2P channel")
            return
        }

        _state.value = HotspotState.Creating
        Log.i(TAG, "Creating WiFi Direct group...")

        registerReceiver(mgr, ch)

        // Generate a meaningful passphrase if none provided
        val actualPassword = if (password.length >= 8) password
        else UUID.randomUUID().toString().replace("-", "").substring(0, 12)

        mgr.createGroup(ch, object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                Log.i(TAG, "WiFi Direct group created successfully")
                // Group info will arrive via WIFI_P2P_THIS_DEVICE_CHANGED_ACTION broadcast
                // We query it after a short delay
                queryGroupInfo(mgr, ch, actualPassword)
            }

            override fun onFailure(reason: Int) {
                val reasonStr = mapReason(reason)
                Log.e(TAG, "Failed to create group: $reasonStr")
                _state.value = HotspotState.Failed(reasonStr)
            }
        })
    }

    /**
     * Query the group info after creation to get SSID/password/address.
     */
    @SuppressLint("MissingPermission")
    private fun queryGroupInfo(mgr: WifiP2pManager, ch: WifiP2pManager.Channel, fallbackPassword: String) {
        // Use a delayed query to allow group info to populate
        android.os.Handler(Looper.getMainLooper()).postDelayed({
            mgr.requestGroupInfo(ch) { group: WifiP2pGroup? ->
                if (group != null) {
                    val networkName = group.networkName
                    val passphrase = group.passphrase ?: fallbackPassword
                    val goAddress = group.owner?.deviceAddress ?: "192.168.49.1"

                    Log.i(TAG, "Group info: SSID=$networkName, pass=$passphrase, GO=$goAddress")
                    _state.value = HotspotState.Active(
                        ssid = networkName,
                        password = passphrase,
                        groupOwnerAddress = goAddress
                    )
                } else {
                    // Group info not yet available; use fallback
                    Log.w(TAG, "Group info not available yet, using fallback values")
                    _state.value = HotspotState.Active(
                        ssid = "SoundBridge_${Build.MODEL}",
                        password = fallbackPassword,
                        groupOwnerAddress = "192.168.49.1"
                    )
                }
            }
        }, 2000) // 2 second delay for group to stabilize
    }

    /**
     * Destroy the WiFi Direct group and release resources.
     */
    fun destroyGroup() {
        val mgr = wifiP2pManager
        val ch = channel
        if (mgr != null && ch != null) {
            mgr.removeGroup(ch, object : WifiP2pManager.ActionListener {
                override fun onSuccess() {
                    Log.i(TAG, "WiFi Direct group destroyed")
                    _state.value = HotspotState.Destroyed
                }

                override fun onFailure(reason: Int) {
                    Log.w(TAG, "Failed to destroy group: ${mapReason(reason)}")
                    // Still mark as destroyed since we're cleaning up
                    _state.value = HotspotState.Destroyed
                }
            })
        } else {
            _state.value = HotspotState.Destroyed
        }
        unregisterReceiver()
    }

    /**
     * Get the numeric state code matching JNI bridge convention.
     * 0=Idle, 1=Creating, 2=Active, 3=Failed/Destroyed
     */
    fun getStateCode(): Int = when (_state.value) {
        is HotspotState.Idle -> 0
        is HotspotState.Creating -> 1
        is HotspotState.Active -> 2
        is HotspotState.Failed -> 3
        is HotspotState.Destroyed -> 0
    }

    private fun registerReceiver(mgr: WifiP2pManager, ch: WifiP2pManager.Channel) {
        if (isReceiverRegistered) return

        receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                when (intent.action) {
                    WifiP2pManager.WIFI_P2P_THIS_DEVICE_CHANGED_ACTION -> {
                        @Suppress("DEPRECATION")
                        val device = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                            intent.getParcelableExtra(
                                WifiP2pManager.EXTRA_WIFI_P2P_DEVICE,
                                android.net.wifi.p2p.WifiP2pDevice::class.java
                            )
                        } else {
                            intent.getParcelableExtra(WifiP2pManager.EXTRA_WIFI_P2P_DEVICE)
                        }
                        device?.let {
                            Log.d(TAG, "This device changed: ${it.deviceName} (${it.deviceAddress})")
                        }
                    }
                }
            }
        }

        val filter = IntentFilter().apply {
            addAction(WifiP2pManager.WIFI_P2P_THIS_DEVICE_CHANGED_ACTION)
        }
        context.registerReceiver(receiver, filter)
        isReceiverRegistered = true
    }

    private fun unregisterReceiver() {
        if (isReceiverRegistered && receiver != null) {
            try {
                context.unregisterReceiver(receiver)
            } catch (_: Exception) {
                // Already unregistered
            }
            receiver = null
            isReceiverRegistered = false
        }
    }

    private fun mapReason(reason: Int): String = when (reason) {
        WifiP2pManager.BUSY -> "System busy"
        WifiP2pManager.ERROR -> "Internal error"
        WifiP2pManager.P2P_UNSUPPORTED -> "WiFi P2P not supported"
        else -> "Unknown error ($reason)"
    }

    fun release() {
        destroyGroup()
        channel = null
    }
}
