package com.soundbridge.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.sp
import com.soundbridge.R
import com.soundbridge.audio.AudioService
import com.soundbridge.native.AudioProfile
import com.soundbridge.native.EqPreset
import com.soundbridge.native.NativeAudioEngine

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

        AudioProfileSection()

        Spacer(modifier = Modifier.height(16.dp))

        EqualizerSection()

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

// ============================================================
// 音质档位选择
// ============================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AudioProfileSection() {
    var selectedProfile by remember { mutableStateOf(AudioProfile.Standard) }
    var isAutoEnabled by remember { mutableStateOf(false) }
    var expanded by remember { mutableStateOf(false) }

        SettingsSection(title = stringResource(R.string.audio_quality_title)) {
        // 音质选择下拉菜单
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.HighQuality,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 16.dp)
            ) {
                Text(text = stringResource(R.string.audio_quality_tier), fontWeight = FontWeight.Medium)
                Text(
                    text = selectedProfile.label,
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }
            Box {
                TextButton(
                    onClick = { expanded = true },
                    enabled = !isAutoEnabled
                ) {
                    Text(selectedProfile.label)
                }
                DropdownMenu(
                    expanded = expanded,
                    onDismissRequest = { expanded = false }
                ) {
                    AudioProfile.entries
                        .filter { it != AudioProfile.Auto && it != AudioProfile.Custom }
                        .forEach { profile ->
                            DropdownMenuItem(
                                text = { Text(profile.label) },
                                onClick = {
                                    selectedProfile = profile
                                    expanded = false
                                    NativeAudioEngine.setAudioProfile(profile)
                                }
                            )
                        }
                }
            }
        }

        // 自动挡开关
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.AutoMode,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 16.dp)
            ) {
                Text(text = stringResource(R.string.audio_quality_auto), fontWeight = FontWeight.Medium)
                Text(
                    text = if (isAutoEnabled) stringResource(R.string.audio_quality_auto_desc) else stringResource(R.string.audio_quality_manual_desc),
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }
            Switch(
                checked = isAutoEnabled,
                onCheckedChange = {
                    isAutoEnabled = it
                    NativeAudioEngine.setAutoProfileEnabled(it)
                    if (it) {
                        NativeAudioEngine.setAudioProfile(AudioProfile.Auto)
                    }
                }
            )
        }
    }
}

// ============================================================
// 均衡器
// ============================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EqualizerSection() {
    var selectedPreset by remember { mutableStateOf(EqPreset.Flat) }
    var isEnabled by remember { mutableStateOf(true) }

        SettingsSection(title = stringResource(R.string.equalizer_title)) {
        // 开关
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.Equalizer,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 16.dp)
            ) {
                Text(text = stringResource(R.string.equalizer_title), fontWeight = FontWeight.Medium)
                Text(
                    text = if (isEnabled) stringResource(R.string.equalizer_enabled, selectedPreset.label) else stringResource(R.string.equalizer_disabled),
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }
            Switch(
                checked = isEnabled,
                onCheckedChange = {
                    isEnabled = it
                    NativeAudioEngine.setEqEnabled(it)
                }
            )
        }

        // 预设选择
        LazyRow(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 40.dp, top = 4.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            items(EqPreset.entries) { preset ->
                FilterChip(
                    selected = selectedPreset == preset,
                    onClick = {
                        selectedPreset = preset
                        NativeAudioEngine.setEqPreset(preset)
                    },
                    label = { Text(preset.label) },
                    enabled = isEnabled
                )
            }
        }
    }
}
