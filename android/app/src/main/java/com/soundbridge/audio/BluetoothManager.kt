package com.soundbridge.audio

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothServerSocket
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.io.IOException
import java.util.UUID

/**
 * Bluetooth classic (RFCOMM) server socket manager.
 *
 * Opens a [BluetoothServerSocket] to accept incoming connections from the PC
 * client. Uses SPP (Serial Port Profile) UUID for classic Bluetooth.
 *
 * Note: This is for classic Bluetooth (RFCOMM), not BLE.
 * For BLE, a separate manager would be needed.
 */
class BluetoothManager(private val context: Context) {

    companion object {
        private const val TAG = "BluetoothManager"
        // Standard SPP (Serial Port Profile) UUID
        private val SPP_UUID: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")
        private const val SERVICE_NAME = "SoundBridge"
    }

    /** Bluetooth connection state exposed via StateFlow. */
    sealed class BluetoothState {
        data object Idle : BluetoothState()
        data object Listening : BluetoothState()
        data class Connected(val deviceName: String, val deviceAddress: String) : BluetoothState()
        data class Failed(val reason: String) : BluetoothState()
    }

    private val _state = MutableStateFlow<BluetoothState>(BluetoothState.Idle)
    val state: StateFlow<BluetoothState> = _state

    /** Callback invoked when a Bluetooth connection is accepted. Receives the remote device address. */
    var listener: ((String) -> Unit)? = null

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private var serverSocket: BluetoothServerSocket? = null
    private var connectedSocket: BluetoothSocket? = null
    private var acceptThread: Thread? = null

    private val bluetoothAdapter: BluetoothAdapter? by lazy {
        val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? android.bluetooth.BluetoothManager
        manager?.adapter
    }

    /** Whether Bluetooth is available on this device. */
    val isAvailable: Boolean get() = bluetoothAdapter != null

    /** Whether Bluetooth is currently enabled. */
    val isEnabled: Boolean get() = bluetoothAdapter?.isEnabled == true

    /**
     * Start listening for incoming Bluetooth connections.
     *
     * Opens a RFCOMM server socket with the standard SPP UUID.
     * The PC client can then connect to this device using the SPP profile.
     *
     * @param deviceName the name to advertise (appears in Bluetooth scan)
     */
    @SuppressLint("MissingPermission")
    fun startListening(deviceName: String = SERVICE_NAME) {
        val adapter = bluetoothAdapter
        if (adapter == null) {
            _state.value = BluetoothState.Failed("Bluetooth not available")
            return
        }

        if (!adapter.isEnabled) {
            _state.value = BluetoothState.Failed("Bluetooth is not enabled")
            return
        }

        // Close existing socket if any
        stopListening()

        scope.launch {
            try {
                // Cancel discovery to improve connection speed
                adapter.cancelDiscovery()

                Log.i(TAG, "Starting Bluetooth server socket: $deviceName")
                serverSocket = adapter.listenUsingRfcommWithServiceRecord(
                    deviceName, SPP_UUID
                )

                _state.value = BluetoothState.Listening
                Log.i(TAG, "Bluetooth listening on SPP UUID: $SPP_UUID")

                // Accept connections in a background thread
                acceptThread = Thread {
                    acceptLoop()
                }.apply {
                    name = "BT-Accept"
                    isDaemon = true
                    start()
                }
            } catch (e: IOException) {
                Log.e(TAG, "Failed to create server socket: ${e.message}")
                _state.value = BluetoothState.Failed("Failed to open Bluetooth socket: ${e.message}")
            }
        }
    }

    /**
     * Accept loop - runs in a background thread, waiting for connections.
     */
    private fun acceptLoop() {
        while (!Thread.currentThread().isInterrupted) {
            try {
                val socket = serverSocket?.accept() ?: break
                Log.i(TAG, "Bluetooth connection from: ${socket.remoteDevice.name} (${socket.remoteDevice.address})")

                // Close previous connection if any
                connectedSocket?.close()

                connectedSocket = socket
                val address = socket.remoteDevice.address ?: "Unknown"
                _state.value = BluetoothState.Connected(
                    deviceName = socket.remoteDevice.name ?: "Unknown",
                    deviceAddress = address
                )

                listener?.invoke(address)

                // Keep the socket open for data transfer
                // Actual data I/O would be handled by the caller
            } catch (e: IOException) {
                if (!Thread.currentThread().isInterrupted) {
                    Log.e(TAG, "Accept failed: ${e.message}")
                }
                break
            }
        }
    }

    /**
     * Stop listening and close all Bluetooth sockets.
     */
    fun stopListening() {
        acceptThread?.interrupt()
        acceptThread = null

        try {
            connectedSocket?.close()
        } catch (_: IOException) {}
        connectedSocket = null

        try {
            serverSocket?.close()
        } catch (_: IOException) {}
        serverSocket = null

        _state.value = BluetoothState.Idle
        Log.i(TAG, "Bluetooth listening stopped")
    }

    /**
     * Get the connected socket for data I/O, or null if not connected.
     */
    fun getConnectedSocket(): BluetoothSocket? = connectedSocket

    /**
     * Get the numeric state code matching JNI bridge convention.
     * 0=Idle, 1=Listening, 2=Connected, 3=Failed
     */
    fun getStateCode(): Int = when (_state.value) {
        is BluetoothState.Idle -> 0
        is BluetoothState.Listening -> 1
        is BluetoothState.Connected -> 2
        is BluetoothState.Failed -> 3
    }

    fun release() {
        listener = null
        stopListening()
    }
}
