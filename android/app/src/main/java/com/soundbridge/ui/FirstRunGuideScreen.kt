@file:OptIn(ExperimentalMaterial3Api::class)

package com.soundbridge.ui

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import com.soundbridge.R
import com.soundbridge.ui.theme.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val PREF_NAME = "soundbridge_prefs"
private const val KEY_FIRST_RUN_COMPLETE = "first_run_complete"

fun isFirstRunComplete(context: Context): Boolean {
    val prefs = context.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
    return prefs.getBoolean(KEY_FIRST_RUN_COMPLETE, false)
}

fun markFirstRunComplete(context: Context) {
    context.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        .edit()
        .putBoolean(KEY_FIRST_RUN_COMPLETE, true)
        .apply()
}

private data class GuidePage(
    val titleResId: Int,
    val descriptionResId: Int,
    val icon: ImageVector
)

private val guidePageConfigs = listOf(
    GuidePage(
        titleResId = R.string.guide_welcome_title,
        descriptionResId = R.string.guide_welcome_desc,
        icon = Icons.Default.Headphones
    ),
    GuidePage(
        titleResId = R.string.guide_permissions_title,
        descriptionResId = R.string.guide_permissions_title, // 描述在 strings.xml 中
        icon = Icons.Default.Security
    ),
    GuidePage(
        titleResId = R.string.guide_audio_test_title,
        descriptionResId = R.string.guide_audio_test_desc,
        icon = Icons.Default.VolumeUp
    ),
    GuidePage(
        titleResId = R.string.guide_ready_title,
        descriptionResId = R.string.guide_ready_desc,
        icon = Icons.Default.CheckCircle
    )
)

@Composable
fun FirstRunGuideScreen(onGuideComplete: () -> Unit) {
    val context = LocalContext.current
    var currentPage by remember { mutableIntStateOf(0) }
    var permissionsGranted by remember { mutableStateOf(false) }
    var audioTestDone by remember { mutableStateOf(false) }
    var isPlayingTest by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val requiredPermissions = remember {
        val perms = mutableListOf(
            Manifest.permission.RECORD_AUDIO,
            Manifest.permission.INTERNET,
            Manifest.permission.ACCESS_NETWORK_STATE,
            Manifest.permission.ACCESS_FINE_LOCATION
        )
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            perms.add(Manifest.permission.NEARBY_WIFI_DEVICES)
            perms.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            perms.add(Manifest.permission.BLUETOOTH_CONNECT)
            perms.add(Manifest.permission.BLUETOOTH_ADVERTISE)
        }
        perms.toTypedArray()
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { results ->
        permissionsGranted = results.values.all { it }
    }

    LaunchedEffect(Unit) {
        permissionsGranted = requiredPermissions.all {
            ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED
        }
    }

    fun playTestTone() {
        isPlayingTest = true
        scope.launch {
            try {
                val toneGen = ToneGenerator(
                    AudioManager.STREAM_MUSIC,
                    ToneGenerator.MAX_VOLUME
                )
                toneGen.startTone(ToneGenerator.TONE_PROP_BEEP2, 500)
                delay(600)
                toneGen.release()
                audioTestDone = true
            } catch (_: Exception) {
                audioTestDone = true
            } finally {
                isPlayingTest = false
            }
        }
    }

    fun finishGuide() {
        markFirstRunComplete(context)
        onGuideComplete()
    }

    val isLastPage = currentPage == guidePageConfigs.size - 1
    val canProceed = when (currentPage) {
        1 -> permissionsGranted
        2 -> audioTestDone
        else -> true
    }

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
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // Page indicator dots
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 32.dp),
            horizontalArrangement = Arrangement.Center
        ) {
            repeat(guidePageConfigs.size) { index ->
                Box(
                    modifier = Modifier
                        .padding(horizontal = 4.dp)
                        .size(if (index == currentPage) 10.dp else 8.dp)
                        .clip(CircleShape)
                        .background(
                            if (index == currentPage) SoundBridgePrimary
                            else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                        )
                )
            }
        }

        // Animated page content
        AnimatedContent(
            targetState = currentPage,
            transitionSpec = {
                fadeIn(animationSpec = tween(300)) togetherWith fadeOut(animationSpec = tween(300))
            },
            label = "page_transition"
        ) { targetPage ->
            val targetPageData = guidePageConfigs[targetPage]
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                // Icon
                Box(
                    modifier = Modifier
                        .size(100.dp)
                        .clip(CircleShape)
                        .background(SoundBridgePrimary.copy(alpha = 0.15f)),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = targetPageData.icon,
                        contentDescription = null,
                        modifier = Modifier.size(56.dp),
                        tint = SoundBridgePrimary
                    )
                }

                Spacer(modifier = Modifier.height(32.dp))

                Text(
                    text = targetPageData.title,
                    fontSize = 24.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onBackground,
                    textAlign = TextAlign.Center
                )

                Spacer(modifier = Modifier.height(16.dp))

                Text(
                    text = targetPageData.description,
                    fontSize = 15.sp,
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.7f),
                    textAlign = TextAlign.Center,
                    lineHeight = 22.sp
                )

                Spacer(modifier = Modifier.height(32.dp))

                // Page-specific action buttons
                when (targetPage) {
                    1 -> {
                        val allGranted = requiredPermissions.all {
                            ContextCompat.checkSelfPermission(context, it) ==
                                    PackageManager.PERMISSION_GRANTED
                        }
                        if (allGranted) {
                            StatusChip(
                                text = stringResource(R.string.guide_permissions_granted),
                                icon = Icons.Default.CheckCircle,
                                color = ConnectionConnected
                            )
                        } else {
                            Button(
                                onClick = { permissionLauncher.launch(requiredPermissions) },
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = SoundBridgePrimary
                                ),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(52.dp)
                            ) {
                                Icon(Icons.Default.Security, contentDescription = null)
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(stringResource(R.string.guide_permissions_button), fontSize = 16.sp)
                            }
                        }
                    }

                    2 -> {
                        if (audioTestDone) {
                            StatusChip(
                                text = stringResource(R.string.guide_audio_test_complete),
                                icon = Icons.Default.CheckCircle,
                                color = ConnectionConnected
                            )
                        } else {
                            Button(
                                onClick = { playTestTone() },
                                enabled = !isPlayingTest,
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = SoundBridgeSecondary
                                ),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(52.dp)
                            ) {
                                if (isPlayingTest) {
                                    CircularProgressIndicator(
                                        modifier = Modifier.size(20.dp),
                                        color = MaterialTheme.colorScheme.onPrimary,
                                        strokeWidth = 2.dp
                                    )
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text(stringResource(R.string.guide_audio_test_playing), fontSize = 16.sp)
                                } else {
                                    Icon(Icons.Default.VolumeUp, contentDescription = null)
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text(stringResource(R.string.guide_audio_test_button), fontSize = 16.sp)
                                }
                            }
                        }
                    }
                }
            }
        }

        // Bottom navigation buttons
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            if (currentPage > 0) {
                OutlinedButton(
                    onClick = { currentPage-- },
                    modifier = Modifier
                        .weight(1f)
                        .height(52.dp),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Text(stringResource(R.string.guide_button_previous), fontSize = 15.sp)
                }
            }

            Button(
                onClick = {
                    if (isLastPage) {
                        finishGuide()
                    } else {
                        currentPage++
                    }
                },
                enabled = canProceed,
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (isLastPage) ConnectionConnected else SoundBridgePrimary
                ),
                modifier = Modifier
                    .weight(1f)
                    .height(52.dp),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text(
                    text = if (isLastPage) stringResource(R.string.guide_button_start) else stringResource(R.string.guide_button_next),
                    fontSize = 15.sp,
                    fontWeight = FontWeight.Medium
                )
            }
        }

        // Skip button (only on non-last pages)
        if (!isLastPage) {
            TextButton(
                onClick = { finishGuide() },
                modifier = Modifier.padding(top = 8.dp)
            ) {
                Text(
                    text = stringResource(R.string.guide_button_skip),
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.5f),
                    fontSize = 13.sp
                )
            }
        }
    }
}

@Composable
private fun StatusChip(
    text: String,
    icon: ImageVector,
    color: androidx.compose.ui.graphics.Color
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = color.copy(alpha = 0.15f)
        ),
        shape = RoundedCornerShape(24.dp)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = color,
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = text,
                color = color,
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp
            )
        }
    }
}
