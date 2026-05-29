package com.soundbridge

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build

class SoundBridgeApp : Application() {

    companion object {
        const val NOTIFICATION_CHANNEL_ID = "soundbridge_audio"
        const val NOTIFICATION_CHANNEL_NAME = "Audio Service"
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                NOTIFICATION_CHANNEL_ID,
                NOTIFICATION_CHANNEL_NAME,
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "SoundBridge audio processing service"
                setShowBadge(false)
            }

            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(channel)
        }
    }
}
