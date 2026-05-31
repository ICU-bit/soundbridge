package com.soundbridge.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.soundbridge.audio.AudioService

enum class AudioMode(val label: String, val subtitle: String) {
    BALANCED("Balanced", "50-100ms latency"),
    HIGH_QUALITY("High Quality", "48kHz/24bit"),
    LOW_LATENCY("Low Latency", "<30ms latency")
}

@Composable
fun SettingsScreen(engineHandle: Long = 0L, audioService: AudioService? = null) {
    var echoCancellation by remember { mutableStateOf(true) }
    var noiseSuppression by remember { mutableStateOf(true) }
    var gainControl by remember { mutableStateOf(true) }
    var selectedSampleRate by remember { mutableIntStateOf(48000) }
    var selectedBitrate by remember { mutableIntStateOf(64000) }
    var selectedAudioMode by remember { mutableStateOf(AudioMode.BALANCED) }
    val encryptionState by (audioService?.encryptionState?.collectAsState()
        ?: remember { mutableStateOf(AudioService.EncryptionState.DISABLED) })
    var encryptionEnabled by remember { mutableStateOf(encryptionState == AudioService.EncryptionState.ENABLED) }

    // 同步外部加密状态变化
    LaunchedEffect(encryptionState) {
        encryptionEnabled = encryptionState == AudioService.EncryptionState.ENABLED
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp)
    ) {
        SettingsSection(title = "Audio Processing") {
            SettingsSwitch(
                title = "Echo Cancellation",
                subtitle = "Remove echo from audio",
                icon = Icons.Default.Cancel,
                checked = echoCancellation,
                onCheckedChange = { echoCancellation = it }
            )

            SettingsSwitch(
                title = "Noise Suppression",
                subtitle = "Reduce background noise",
                icon = Icons.Default.NoiseAware,
                checked = noiseSuppression,
                onCheckedChange = { noiseSuppression = it }
            )

            SettingsSwitch(
                title = "Gain Control",
                subtitle = "Automatic volume adjustment",
                icon = Icons.Default.VolumeUp,
                checked = gainControl,
                onCheckedChange = { gainControl = it }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = "Audio Mode") {
            SettingsDropdown(
                title = "Mode",
                subtitle = selectedAudioMode.subtitle,
                icon = Icons.Default.Tune,
                options = AudioMode.entries,
                selectedOption = selectedAudioMode,
                onOptionSelected = { mode ->
                    selectedAudioMode = mode
                    audioService?.setAudioMode(mode.ordinal)
                },
                optionLabel = { it.label }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = "Audio Quality") {
            SettingsDropdown(
                title = "Sample Rate",
                subtitle = "${selectedSampleRate / 1000} kHz",
                icon = Icons.Default.Speed,
                options = listOf(16000, 24000, 44100, 48000),
                selectedOption = selectedSampleRate,
                onOptionSelected = { selectedSampleRate = it },
                optionLabel = { "${it / 1000} kHz" }
            )

            SettingsDropdown(
                title = "Bitrate",
                subtitle = "${selectedBitrate / 1000} kbps",
                icon = Icons.Default.DataUsage,
                options = listOf(16000, 24000, 32000, 48000, 64000, 128000),
                selectedOption = selectedBitrate,
                onOptionSelected = { selectedBitrate = it },
                optionLabel = { "${it / 1000} kbps" }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = "Network") {
            SettingsItem(
                title = "Protocol",
                subtitle = "UDP / QUIC",
                icon = Icons.Default.Wifi
            )

            SettingsItem(
                title = "Buffer Size",
                subtitle = "20ms",
                icon = Icons.Default.Timer
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = "Security") {
            SettingsSwitch(
                title = "Encryption (DTLS/SRTP)",
                subtitle = if (encryptionEnabled) "AES-128-CM + HMAC-SHA1-80 enabled" else "End-to-end audio encryption disabled",
                icon = Icons.Default.Lock,
                checked = encryptionEnabled,
                onCheckedChange = { enabled ->
                    encryptionEnabled = enabled
                    if (enabled) {
                        audioService?.enableEncryption()
                    } else {
                        audioService?.disableEncryption()
                    }
                }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = "About") {
            SettingsItem(
                title = "Version",
                subtitle = "1.0.0",
                icon = Icons.Default.Info
            )

            SettingsItem(
                title = "Native Engine",
                subtitle = "SoundBridge Native",
                icon = Icons.Default.Memory
            )
        }
    }
}

@Composable
fun SettingsSection(
    title: String,
    content: @Composable ColumnScope.() -> Unit
) {
    Column {
        Text(
            text = title,
            fontWeight = FontWeight.Bold,
            fontSize = 14.sp,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(bottom = 8.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surface
            )
        ) {
            Column(modifier = Modifier.padding(8.dp)) {
                content()
            }
        }
    }
}

@Composable
fun SettingsSwitch(
    title: String,
    subtitle: String,
    icon: ImageVector,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.primary
        )
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(horizontal = 16.dp)
        ) {
            Text(text = title, fontWeight = FontWeight.Medium)
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange
        )
    }
}

@Composable
fun SettingsItem(
    title: String,
    subtitle: String,
    icon: ImageVector
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.primary
        )
        Column(
            modifier = Modifier.padding(horizontal = 16.dp)
        ) {
            Text(text = title, fontWeight = FontWeight.Medium)
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun <T> SettingsDropdown(
    title: String,
    subtitle: String,
    icon: ImageVector,
    options: List<T>,
    selectedOption: T,
    onOptionSelected: (T) -> Unit,
    optionLabel: (T) -> String
) {
    var expanded by remember { mutableStateOf(false) }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.primary
        )
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(horizontal = 16.dp)
        ) {
            Text(text = title, fontWeight = FontWeight.Medium)
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
            )
        }
        Box {
            TextButton(onClick = { expanded = true }) {
                Text(optionLabel(selectedOption))
            }
            DropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false }
            ) {
                options.forEach { option ->
                    DropdownMenuItem(
                        text = { Text(optionLabel(option)) },
                        onClick = {
                            onOptionSelected(option)
                            expanded = false
                        }
                    )
                }
            }
        }
    }
}
