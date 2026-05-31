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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalContext
import com.soundbridge.R
import com.soundbridge.audio.AudioService
import com.soundbridge.native.EqPreset
import com.soundbridge.native.NativeAudioEngine

@Composable
fun SettingsScreen(engineHandle: Long = 0L, audioService: AudioService? = null) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("soundbridge_prefs", android.content.Context.MODE_PRIVATE) }
    
    var echoCancellation by remember { mutableStateOf(prefs.getBoolean("echo_cancellation", true)) }
    var noiseSuppression by remember { mutableStateOf(prefs.getBoolean("noise_suppression", true)) }
    var gainControl by remember { mutableStateOf(prefs.getBoolean("gain_control", true)) }
    var selectedSampleRate by remember { mutableIntStateOf(prefs.getInt("sample_rate", 48000)) }
    var selectedBitrate by remember { mutableIntStateOf(prefs.getInt("bitrate", 128000)) }
    var isAutoMode by remember { mutableStateOf(prefs.getBoolean("auto_mode", false)) }
    val encryptionState by (audioService?.encryptionState?.collectAsState()
        ?: remember { mutableStateOf(AudioService.EncryptionState.DISABLED) })
    var encryptionEnabled by remember { mutableStateOf(encryptionState == AudioService.EncryptionState.ENABLED) }

    // 同步外部加密状态变化
    LaunchedEffect(encryptionState) {
        encryptionEnabled = encryptionState == AudioService.EncryptionState.ENABLED
    }

    // 初始化：将保存的设置同步到native层
    LaunchedEffect(Unit) {
        NativeAudioEngine.nativeSetSampleRate(selectedSampleRate)
        NativeAudioEngine.nativeSetBitrate(selectedBitrate)
        NativeAudioEngine.setAutoProfileEnabled(isAutoMode)
        // 音频处理开关同步到native层
        // 注意：这些需要engineHandle，但当前架构下handle在AudioService中
        // 先保存状态，连接后AudioService会应用
    }

    // 保存设置变化的辅助函数
    fun saveAutoMode(enabled: Boolean) {
        isAutoMode = enabled
        NativeAudioEngine.setAutoProfileEnabled(enabled)
        prefs.edit().putBoolean("auto_mode", enabled).apply()
    }

    fun saveSampleRate(rate: Int) {
        selectedSampleRate = rate
        NativeAudioEngine.nativeSetSampleRate(rate)
        prefs.edit().putInt("sample_rate", rate).apply()
    }

    fun saveBitrate(rate: Int) {
        selectedBitrate = rate
        NativeAudioEngine.nativeSetBitrate(rate)
        prefs.edit().putInt("bitrate", rate).apply()
    }

    fun saveEchoCancellation(enabled: Boolean) {
        echoCancellation = enabled
        if (audioService != null && audioService.handle != 0L) {
            NativeAudioEngine.nativeSetEchoCancellationEnabled(audioService.handle, enabled)
        }
        prefs.edit().putBoolean("echo_cancellation", enabled).apply()
    }

    fun saveNoiseSuppression(enabled: Boolean) {
        noiseSuppression = enabled
        if (audioService != null && audioService.handle != 0L) {
            NativeAudioEngine.nativeSetNoiseSuppressionEnabled(audioService.handle, enabled)
        }
        prefs.edit().putBoolean("noise_suppression", enabled).apply()
    }

    fun saveGainControl(enabled: Boolean) {
        gainControl = enabled
        if (audioService != null && audioService.handle != 0L) {
            NativeAudioEngine.nativeSetGainControlEnabled(audioService.handle, enabled)
        }
        prefs.edit().putBoolean("gain_control", enabled).apply()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp)
    ) {
        SettingsSection(title = stringResource(R.string.section_audio_processing)) {
            SettingsSwitch(
                title = stringResource(R.string.echo_cancellation),
                subtitle = stringResource(R.string.echo_cancellation_desc),
                icon = Icons.Default.Cancel,
                checked = echoCancellation,
                onCheckedChange = { saveEchoCancellation(it) }
            )

            SettingsSwitch(
                title = stringResource(R.string.noise_suppression),
                subtitle = stringResource(R.string.noise_suppression_desc),
                icon = Icons.Default.NoiseAware,
                checked = noiseSuppression,
                onCheckedChange = { saveNoiseSuppression(it) }
            )

            SettingsSwitch(
                title = stringResource(R.string.gain_control),
                subtitle = stringResource(R.string.gain_control_desc),
                icon = Icons.Default.VolumeUp,
                checked = gainControl,
                onCheckedChange = { saveGainControl(it) }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = stringResource(R.string.section_audio_quality)) {
            // 自动档位开关
            SettingsSwitch(
                title = stringResource(R.string.auto_mode),
                subtitle = if (isAutoMode) stringResource(R.string.auto_mode_enabled_desc) else stringResource(R.string.auto_mode_disabled_desc),
                icon = Icons.Default.AutoMode,
                checked = isAutoMode,
                onCheckedChange = { enabled -> saveAutoMode(enabled) }
            )

            // 采样率选择（自动档位禁用时才可选）
            SettingsDropdown(
                title = stringResource(R.string.label_sample_rate),
                subtitle = "${selectedSampleRate / 1000} kHz",
                icon = Icons.Default.Speed,
                options = listOf(44100, 48000, 96000, 192000),
                selectedOption = selectedSampleRate,
                onOptionSelected = { saveSampleRate(it) },
                optionLabel = { "${it / 1000} kHz" },
                enabled = !isAutoMode
            )

            // 码率选择（自动档位禁用时才可选）
            SettingsDropdown(
                title = stringResource(R.string.label_bitrate),
                subtitle = "${selectedBitrate / 1000} kbps",
                icon = Icons.Default.DataUsage,
                options = listOf(128000, 192000, 256000, 320000, 512000, 1024000),
                selectedOption = selectedBitrate,
                onOptionSelected = { saveBitrate(it) },
                optionLabel = { "${it / 1000} kbps" },
                enabled = !isAutoMode
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        EqualizerSection(prefs)

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = stringResource(R.string.section_network)) {
            SettingsItem(
                title = stringResource(R.string.label_protocol),
                subtitle = "UDP / QUIC",
                icon = Icons.Default.Wifi
            )

            SettingsItem(
                title = stringResource(R.string.label_buffer_size),
                subtitle = "20ms",
                icon = Icons.Default.Timer
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SettingsSection(title = stringResource(R.string.section_security)) {
            SettingsSwitch(
                title = stringResource(R.string.encryption_dtls_srtp),
                subtitle = if (encryptionEnabled) stringResource(R.string.encryption_enabled_desc) else stringResource(R.string.encryption_disabled_desc),
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

        SettingsSection(title = stringResource(R.string.section_about)) {
            SettingsItem(
                title = stringResource(R.string.label_version),
                subtitle = "1.0.0",
                icon = Icons.Default.Info
            )

            SettingsItem(
                title = stringResource(R.string.label_native_engine),
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
    optionLabel: @Composable (T) -> String,
    enabled: Boolean = true
) {
    var expanded by remember { mutableStateOf(false) }
    val alpha = if (enabled) 1f else 0.5f

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp)
            .let { if (!enabled) it.alpha(0.5f) else it },
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.primary.copy(alpha = alpha)
        )
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(horizontal = 16.dp)
        ) {
            Text(
                text = title,
                fontWeight = FontWeight.Medium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = alpha)
            )
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = if (enabled) 0.7f else 0.35f)
            )
        }
        Box {
            TextButton(
                onClick = { expanded = true },
                enabled = enabled
            ) {
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
// 均衡器
// ============================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EqualizerSection(prefs: android.content.SharedPreferences) {
    var selectedPreset by remember { mutableIntStateOf(prefs.getInt("eq_preset", 0)) }
    var isEnabled by remember { mutableStateOf(prefs.getBoolean("eq_enabled", true)) }

    fun saveEqEnabled(enabled: Boolean) {
        isEnabled = enabled
        NativeAudioEngine.setEqEnabled(enabled)
        prefs.edit().putBoolean("eq_enabled", enabled).apply()
    }

    fun saveEqPreset(index: Int) {
        selectedPreset = index
        NativeAudioEngine.setEqPreset(EqPreset.entries[index])
        prefs.edit().putInt("eq_preset", index).apply()
    }

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
                    text = if (isEnabled) stringResource(R.string.equalizer_enabled, EqPreset.entries[selectedPreset].label) else stringResource(R.string.equalizer_disabled),
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }
            Switch(
                checked = isEnabled,
                onCheckedChange = { saveEqEnabled(it) }
            )
        }

        // 预设选择
        LazyRow(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 40.dp, top = 4.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            items(EqPreset.entries.size) { index ->
                FilterChip(
                    selected = selectedPreset == index,
                    onClick = { saveEqPreset(index) },
                    label = { Text(EqPreset.entries[index].label) },
                    enabled = isEnabled
                )
            }
        }
    }
}
