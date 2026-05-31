@file:OptIn(ExperimentalMaterial3Api::class)

package com.soundbridge.ui

import androidx.compose.animation.core.*
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.soundbridge.audio.AudioService
import com.soundbridge.ui.theme.*

@Composable
fun HomeScreen(audioService: AudioService? = null) {
    val isConnected by (audioService?.connectionState?.collectAsState() ?: remember { mutableStateOf(AudioService.ConnectionState.DISCONNECTED) })
    val audioLevel by (audioService?.audioLevel?.collectAsState() ?: remember { mutableFloatStateOf(0f) })
    var isMuted by remember { mutableStateOf(audioService?.isMuted() ?: false) }
    var serverAddress by remember { mutableStateOf("192.168.1.100") }
    var serverPort by remember { mutableStateOf("8080") }
    var mixRatio by remember { mutableFloatStateOf(50f) } // 0=全PC, 100=全手机
    var selectedConnectionType by remember { mutableIntStateOf(0) } // 0=WiFiLan, 1=WiFiDirect, 2=UsbAdb, 3=Bluetooth
    val connectionTypeNames = listOf("WiFi 局域网", "WiFi 直连", "USB/ADB", "蓝牙")

    val connected = isConnected == AudioService.ConnectionState.CONNECTED

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        MaterialTheme.colorScheme.background,
                        MaterialTheme.colorScheme.surface
                    )
                )
            )
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        ConnectionStatusCard(connected)

        Spacer(modifier = Modifier.height(32.dp))

        AudioLevelVisualizer(audioLevel)

        Spacer(modifier = Modifier.height(32.dp))

        ControlButtons(
            isConnected = connected,
            isMuted = isMuted,
            onConnectClick = {
                if (connected) {
                    audioService?.disconnect()
                } else {
                    val port = serverPort.toIntOrNull() ?: 8080
                    audioService?.connectToServer(serverAddress, port)
                }
            },
            onMuteClick = {
                val newMuted = !isMuted
                isMuted = newMuted
                audioService?.setMute(newMuted)
            }
        )

        Spacer(modifier = Modifier.height(24.dp))

        ServerConfigSection(
            serverAddress = serverAddress,
            serverPort = serverPort,
            onAddressChange = { serverAddress = it },
            onPortChange = { serverPort = it }
        )

        Spacer(modifier = Modifier.height(16.dp))

        ConnectionTypeSection(
            selectedType = selectedConnectionType,
            typeNames = connectionTypeNames,
            onTypeSelected = { selectedConnectionType = it }
        )

        Spacer(modifier = Modifier.height(16.dp))

        DeviceDiscoverySection(
            audioService = audioService,
            onDeviceSelected = { address, port ->
                serverAddress = address
                serverPort = port.toString()
            }
        )

        Spacer(modifier = Modifier.height(16.dp))

        MixRatioSection(
            mixRatio = mixRatio,
            onMixRatioChange = { newRatio ->
                mixRatio = newRatio
                val pcVol = (100f - newRatio) / 100f
                val phoneVol = newRatio / 100f
                audioService?.setMixRatio(pcVol, phoneVol)
            }
        )
    }
}

@Composable
fun ConnectionStatusCard(isConnected: Boolean) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (isConnected) ConnectionConnected.copy(alpha = 0.2f)
            else ConnectionDisconnected.copy(alpha = 0.2f)
        ),
        shape = RoundedCornerShape(16.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center
        ) {
            Box(
                modifier = Modifier
                    .size(12.dp)
                    .clip(CircleShape)
                    .background(if (isConnected) ConnectionConnected else ConnectionDisconnected)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = if (isConnected) "Connected" else "Disconnected",
                color = if (isConnected) ConnectionConnected else ConnectionDisconnected,
                fontWeight = FontWeight.Bold,
                fontSize = 16.sp
            )
        }
    }
}

@Composable
fun AudioLevelVisualizer(level: Float) {
    val animatedLevel by animateFloatAsState(
        targetValue = level,
        animationSpec = tween(durationMillis = 100),
        label = "audio_level"
    )

    Box(
        modifier = Modifier.size(200.dp),
        contentAlignment = Alignment.Center
    ) {
        Canvas(modifier = Modifier.size(180.dp)) {
            val strokeWidth = 12.dp.toPx()
            val radius = (size.minDimension - strokeWidth) / 2
            val center = Offset(size.width / 2, size.height / 2)

            drawCircle(
                color = Color.Gray.copy(alpha = 0.3f),
                radius = radius,
                center = center,
                style = Stroke(strokeWidth)
            )

            val sweepAngle = animatedLevel * 360f
            val color = when {
                animatedLevel < 0.3f -> AudioLevelLow
                animatedLevel < 0.7f -> AudioLevelMedium
                else -> AudioLevelHigh
            }

            drawArc(
                color = color,
                startAngle = -90f,
                sweepAngle = sweepAngle,
                useCenter = false,
                style = Stroke(strokeWidth, cap = StrokeCap.Round)
            )
        }

        Column(
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "${(animatedLevel * 100).toInt()}%",
                fontSize = 36.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Text(
                text = "Audio Level",
                fontSize = 14.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
            )
        }
    }
}

@Composable
fun ControlButtons(
    isConnected: Boolean,
    isMuted: Boolean,
    onConnectClick: () -> Unit,
    onMuteClick: () -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceEvenly
    ) {
        Button(
            onClick = onConnectClick,
            colors = ButtonDefaults.buttonColors(
                containerColor = if (isConnected) ConnectionDisconnected else ConnectionConnected
            ),
            modifier = Modifier
                .weight(1f)
                .height(56.dp)
                .padding(horizontal = 8.dp)
        ) {
            Icon(
                imageVector = if (isConnected) Icons.Default.Close else Icons.Default.PlayArrow,
                contentDescription = null,
                modifier = Modifier.size(24.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(if (isConnected) "Disconnect" else "Connect")
        }

        Button(
            onClick = onMuteClick,
            colors = ButtonDefaults.buttonColors(
                containerColor = if (isMuted) MaterialTheme.colorScheme.error
                else MaterialTheme.colorScheme.secondary
            ),
            modifier = Modifier
                .weight(1f)
                .height(56.dp)
                .padding(horizontal = 8.dp)
        ) {
            Icon(
                imageVector = if (isMuted) Icons.Default.MicOff else Icons.Default.Mic,
                contentDescription = null,
                modifier = Modifier.size(24.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(if (isMuted) "Unmute" else "Mute")
        }
    }
}

@Composable
fun ServerConfigSection(
    serverAddress: String,
    serverPort: String,
    onAddressChange: (String) -> Unit,
    onPortChange: (String) -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Text(
                text = "Server Configuration",
                fontWeight = FontWeight.Bold,
                fontSize = 16.sp,
                modifier = Modifier.padding(bottom = 12.dp)
            )

            OutlinedTextField(
                value = serverAddress,
                onValueChange = onAddressChange,
                label = { Text("Server Address") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            Spacer(modifier = Modifier.height(8.dp))

            OutlinedTextField(
                value = serverPort,
                onValueChange = onPortChange,
                label = { Text("Port") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
        }
    }
}

@Composable
fun ConnectionTypeSection(
    selectedType: Int,
    typeNames: List<String>,
    onTypeSelected: (Int) -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Connection Type",
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
                Icon(
                    imageVector = Icons.Default.Wifi,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary
                )
            }

            Spacer(modifier = Modifier.height(4.dp))

            Text(
                text = "WiFi LAN: auto-discover. WiFi Direct: hotspot. USB/ADB: wired. Bluetooth: BLE.",
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
            )

            Spacer(modifier = Modifier.height(12.dp))

            // 连接方式选择芯片
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                typeNames.forEachIndexed { index, name ->
                    FilterChip(
                        selected = selectedType == index,
                        onClick = { onTypeSelected(index) },
                        label = { Text(name, fontSize = 12.sp) },
                        modifier = Modifier.weight(1f)
                    )
                }
            }
        }
    }
}

@Composable
fun DeviceDiscoverySection(
    audioService: AudioService?,
    onDeviceSelected: (String, Int) -> Unit
) {
    val discoveredDevices by (audioService?.discoveredDevices?.collectAsState()
        ?: remember { mutableStateOf(emptyList()) })
    val isScanning by (audioService?.isScanning?.collectAsState()
        ?: remember { mutableStateOf(false) })

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Discovered Devices",
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )

                Button(
                    onClick = {
                        if (isScanning) {
                            audioService?.stopDeviceDiscovery()
                        } else {
                            audioService?.startDeviceDiscovery()
                        }
                    },
                    enabled = audioService != null,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (isScanning) MaterialTheme.colorScheme.error
                        else MaterialTheme.colorScheme.primary
                    )
                ) {
                    Icon(
                        imageVector = if (isScanning) Icons.Default.Close else Icons.Default.Search,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(if (isScanning) "Stop" else "Scan")
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            if (discoveredDevices.isEmpty()) {
                Text(
                    text = if (isScanning) "Scanning..." else "No devices found. Tap Scan to search.",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    fontSize = 14.sp,
                    modifier = Modifier.padding(vertical = 8.dp)
                )
            } else {
                discoveredDevices.forEach { device ->
                    DeviceItem(
                        name = device.name,
                        address = device.address,
                        port = device.port,
                        onClick = { onDeviceSelected(device.address, device.port) }
                    )
                }
            }
        }
    }
}

@Composable
fun DeviceItem(
    name: String,
    address: String,
    port: Int,
    onClick: () -> Unit
) {
    Card(
        onClick = onClick,
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        ),
        shape = RoundedCornerShape(8.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.PhoneAndroid,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = name,
                    fontWeight = FontWeight.Medium,
                    fontSize = 14.sp
                )
                Text(
                    text = "$address:$port",
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                )
            }
            Icon(
                imageVector = Icons.Default.ArrowForward,
                contentDescription = "Connect",
                tint = MaterialTheme.colorScheme.primary
            )
        }
    }
}

@Composable
fun MixRatioSection(
    mixRatio: Float,
    onMixRatioChange: (Float) -> Unit
) {
    val pcPercent = (100f - mixRatio).toInt()
    val phonePercent = mixRatio.toInt()

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Mix Ratio",
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
                Text(
                    text = "PC $pcPercent% / Phone $phonePercent%",
                    fontSize = 14.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "PC",
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    modifier = Modifier.width(28.dp)
                )
                Slider(
                    value = mixRatio,
                    onValueChange = onMixRatioChange,
                    valueRange = 0f..100f,
                    steps = 0,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    text = "Phone",
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    modifier = Modifier.width(50.dp)
                )
            }
        }
    }
}
