package com.soundbridge.audio

import kotlinx.coroutines.*
import java.io.InputStream
import java.io.OutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress

class BluetoothUdpBridge(
    private val btInput: InputStream,
    private val btOutput: OutputStream,
    private val localUdpPort: Int
) {
    private val udpSocket = DatagramSocket(localUdpPort)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var running = false

    fun start(remoteAddress: InetAddress, remotePort: Int) {
        running = true
        // BT → UDP
        scope.launch {
            val buffer = ByteArray(1024)
            while (running && isActive) {
                try {
                    val bytesRead = btInput.read(buffer)
                    if (bytesRead > 0) {
                        val packet = DatagramPacket(buffer, bytesRead, remoteAddress, remotePort)
                        udpSocket.send(packet)
                    }
                } catch (e: Exception) {
                    if (running) e.printStackTrace()
                    break
                }
            }
        }
        // UDP → BT
        scope.launch {
            val buffer = ByteArray(1024)
            while (running && isActive) {
                try {
                    val packet = DatagramPacket(buffer, buffer.size)
                    udpSocket.receive(packet)
                    btOutput.write(packet.data, 0, packet.length)
                    btOutput.flush()
                } catch (e: Exception) {
                    if (running) e.printStackTrace()
                    break
                }
            }
        }
    }

    fun stop() {
        running = false
        scope.cancel()
        udpSocket.close()
    }
}
