package com.soundbridge.ui

import androidx.compose.animation.core.*
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
import com.soundbridge.ui.theme.*

@Composable
fun HomeScreen() {
    var isConnected by remember { mutableStateOf(false) }
    var isMuted by remember { mutableStateOf(false) }
    var audioLevel by remember { mutableFloatStateOf(0f) }
    var serverAddress by remember { mutableStateOf("192.168.1.100") }
    var serverPort by remember { mutableStateOf("8080") }

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
        ConnectionStatusCard(isConnected)

        Spacer(modifier = Modifier.height(32.dp))

        AudioLevelVisualizer(audioLevel)

        Spacer(modifier = Modifier.height(32.dp))

        ControlButtons(
            isConnected = isConnected,
            isMuted = isMuted,
            onConnectClick = { isConnected = !isConnected },
            onMuteClick = { isMuted = !isMuted }
        )

        Spacer(modifier = Modifier.height(24.dp))

        ServerConfigSection(
            serverAddress = serverAddress,
            serverPort = serverPort,
            onAddressChange = { serverAddress = it },
            onPortChange = { serverPort = it }
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
