package com.soundbridge.audio

import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.InputStreamReader

/**
 * ADB reverse port forwarding manager.
 *
 * Uses `adb reverse tcp:LOCAL tcp:REMOTE` to set up a reverse port forward
 * so the PC can connect to the Android device's UDP port via localhost.
 *
 * Requires: device connected via USB with ADB debugging enabled.
 * No special Android permissions needed (runs via adb shell).
 */
class AdbManager {

    companion object {
        private const val TAG = "AdbManager"
    }

    /** ADB connection state exposed via StateFlow. */
    sealed class AdbState {
        data object Disconnected : AdbState()
        data object Connecting : AdbState()
        data class Connected(val localPort: Int, val remotePort: Int) : AdbState()
        data class Failed(val reason: String) : AdbState()
    }

    private val _state = MutableStateFlow<AdbState>(AdbState.Disconnected)
    val state: StateFlow<AdbState> = _state

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /** Current port forward info (null if not active). */
    private var activeForward: Pair<Int, Int>? = null

    /**
     * Set up ADB reverse port forwarding.
     *
     * Executes `adb reverse tcp:<remotePort> tcp:<localPort>` so that
     * connections to `tcp:<remotePort>` on the PC are forwarded to
     * `tcp:<localPort>` on this Android device.
     *
     * @param localPort  the port on this device (e.g., the UDP listen port)
     * @param remotePort the port on the PC side (localhost)
     */
    fun setupPortForward(localPort: Int, remotePort: Int) {
        if (localPort <= 0 || localPort > 65535 || remotePort <= 0 || remotePort > 65535) {
            _state.value = AdbState.Failed("Invalid port: local=$localPort, remote=$remotePort")
            return
        }

        _state.value = AdbState.Connecting
        Log.i(TAG, "Setting up ADB reverse: tcp:$remotePort -> tcp:$localPort")

        scope.launch {
            try {
                // First, remove any existing reverse forward for this remote port
                execAdb("reverse", "--remove", "tcp:$remotePort")

                // Set up the reverse forward
                val result = execAdb("reverse", "tcp:$remotePort", "tcp:$localPort")
                if (result.exitCode == 0) {
                    activeForward = Pair(localPort, remotePort)
                    _state.value = AdbState.Connected(localPort, remotePort)
                    Log.i(TAG, "ADB reverse established: tcp:$remotePort -> tcp:$localPort")
                } else {
                    val error = result.stderr.ifEmpty { result.stdout }
                    Log.e(TAG, "ADB reverse failed: $error")
                    _state.value = AdbState.Failed("ADB reverse failed: $error")
                }
            } catch (e: Exception) {
                Log.e(TAG, "ADB reverse exception: ${e.message}")
                _state.value = AdbState.Failed("ADB not available: ${e.message}")
            }
        }
    }

    /**
     * Check if the current port forward is still active.
     */
    fun checkConnection() {
        val forward = activeForward
        if (forward == null) {
            _state.value = AdbState.Disconnected
            return
        }

        scope.launch {
            try {
                val result = execAdb("reverse", "--list")
                if (result.exitCode == 0) {
                    val expected = "tcp:${forward.second} tcp:${forward.first}"
                    val isActive = result.stdout.contains(expected)
                    if (isActive) {
                        _state.value = AdbState.Connected(forward.first, forward.second)
                    } else {
                        Log.w(TAG, "Port forward no longer active")
                        activeForward = null
                        _state.value = AdbState.Disconnected
                    }
                } else {
                    _state.value = AdbState.Failed("Failed to check ADB status")
                }
            } catch (e: Exception) {
                _state.value = AdbState.Failed("ADB check failed: ${e.message}")
            }
        }
    }

    /**
     * Remove the active port forward and disconnect.
     */
    fun disconnect() {
        val forward = activeForward
        if (forward != null) {
            scope.launch {
                try {
                    execAdb("reverse", "--remove", "tcp:${forward.second}")
                } catch (e: Exception) {
                    Log.w(TAG, "Failed to remove ADB reverse: ${e.message}")
                }
            }
        }
        activeForward = null
        _state.value = AdbState.Disconnected
    }

    /**
     * Get the numeric state code matching JNI bridge convention.
     * 0=Idle/Disconnected, 1=Connecting, 2=Connected, 3=Failed
     */
    fun getStateCode(): Int = when (_state.value) {
        is AdbState.Disconnected -> 0
        is AdbState.Connecting -> 1
        is AdbState.Connected -> 2
        is AdbState.Failed -> 3
    }

    /**
     * Execute an adb command.
     */
    private suspend fun execAdb(vararg args: String): ProcessResult = withContext(Dispatchers.IO) {
        val cmd = listOf("adb") + args.toList()
        Log.d(TAG, "Executing: ${cmd.joinToString(" ")}")

        val process = Runtime.getRuntime().exec(cmd.toTypedArray())

        val stdout = BufferedReader(InputStreamReader(process.inputStream)).use { it.readText() }
        val stderr = BufferedReader(InputStreamReader(process.errorStream)).use { it.readText() }
        val exitCode = process.waitFor()

        ProcessResult(exitCode, stdout.trim(), stderr.trim())
    }

    private data class ProcessResult(val exitCode: Int, val stdout: String, val stderr: String)

    fun release() {
        disconnect()
    }
}
