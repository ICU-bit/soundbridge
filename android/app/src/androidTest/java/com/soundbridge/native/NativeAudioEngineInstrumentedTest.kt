package com.soundbridge.native

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.FixMethodOrder
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters

/**
 * Instrumented tests for [NativeAudioEngine] JNI methods.
 *
 * These tests run on a real device or emulator via `./gradlew connectedAndroidTest`.
 * The native `soundbridge_native` library is guaranteed to be loaded on-device.
 */
@RunWith(AndroidJUnit4::class)
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
class NativeAudioEngineInstrumentedTest {

    // ═══════════════════════════════════════════════════
    // Version
    // ═══════════════════════════════════════════════════

    @Test
    fun version_returnsNonEmptyString() {
        val version = NativeAudioEngine.nativeGetVersion()
        assertNotNull(version)
        assertTrue(version.isNotBlank())
        assertTrue(version.matches(Regex("""\d+\.\d+\.\d+""")))
    }

    // ═══════════════════════════════════════════════════
    // Engine lifecycle
    // ═══════════════════════════════════════════════════

    @Test
    fun engine_initAndRelease() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue("Engine handle must be non-zero", handle != 0L)
        NativeAudioEngine.nativeRelease(handle)
    }

    @Test
    fun engine_invalidHandle_doesNotCrash() {
        NativeAudioEngine.nativeRelease(0L)
        NativeAudioEngine.nativeStart(0L)
        NativeAudioEngine.nativeStop(0L)
    }

    // ═══════════════════════════════════════════════════
    // Mix ratio
    // ═══════════════════════════════════════════════════

    @Test
    fun mixRatio_setAndGet_roundTrip() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            NativeAudioEngine.nativeSetMixRatio(handle, 0.25f, 0.75f)
            val ratio = NativeAudioEngine.nativeGetMixRatio(handle)
            assertNotNull(ratio)
            assertEquals(0.25f, ratio!![0], 0.001f)
            assertEquals(0.75f, ratio[1], 0.001f)
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun mixRatio_boundaryValues() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            // 0.0 / 0.0
            NativeAudioEngine.nativeSetMixRatio(handle, 0.0f, 0.0f)
            val min = NativeAudioEngine.nativeGetMixRatio(handle)!!
            assertEquals(0.0f, min[0], 0.001f)
            assertEquals(0.0f, min[1], 0.001f)

            // 1.0 / 1.0
            NativeAudioEngine.nativeSetMixRatio(handle, 1.0f, 1.0f)
            val max = NativeAudioEngine.nativeGetMixRatio(handle)!!
            assertEquals(1.0f, max[0], 0.001f)
            assertEquals(1.0f, max[1], 0.001f)
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun mixRatio_invalidHandle_returnsNull() {
        assertNull(NativeAudioEngine.nativeGetMixRatio(0L))
        assertEquals(-1, NativeAudioEngine.nativeSetMixRatio(0L, 0.5f, 0.5f))
    }

    // ═══════════════════════════════════════════════════
    // Audio mode
    // ═══════════════════════════════════════════════════

    @Test
    fun audioMode_setAndGet_roundTrip() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            for (mode in 0..2) {
                val rc = NativeAudioEngine.nativeSetAudioMode(handle, mode)
                assertEquals(0, rc)
                assertEquals(mode, NativeAudioEngine.nativeGetAudioMode(handle))
            }
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun audioMode_invalidHandle_returnsDefault() {
        assertEquals(0, NativeAudioEngine.nativeGetAudioMode(0L))
        assertEquals(-1, NativeAudioEngine.nativeSetAudioMode(0L, 1))
    }

    // ═══════════════════════════════════════════════════
    // Connection state stubs (hotspot / ADB / BT)
    // ═══════════════════════════════════════════════════

    @Test
    fun hotspot_setAndGetState() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            NativeAudioEngine.nativeHotspotSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeHotspotState(handle))
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun adb_setAndGetState() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            NativeAudioEngine.nativeAdbSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeAdbState(handle))
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun bt_setAndGetState() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            NativeAudioEngine.nativeBtSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeBtState(handle))
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }

    // ═══════════════════════════════════════════════════
    // Pipeline control
    // ═══════════════════════════════════════════════════

    @Test
    fun pipeline_invalidHandle_returnsError() {
        assertEquals(-1, NativeAudioEngine.nativePipelineState(0L))
        assertEquals(-1, NativeAudioEngine.nativePipelineStart(0L))
        assertEquals(-1, NativeAudioEngine.nativePipelineStop(0L))
        assertEquals(-1, NativeAudioEngine.nativeBind(0L, 12345))
    }

    // ═══════════════════════════════════════════════════
    // Discovery stubs
    // ═══════════════════════════════════════════════════

    @Test
    fun discovery_createAndFindDevices() {
        val handle = NativeAudioEngine.nativeDiscoveryCreate()
        assertTrue(handle != 0L)
        val devices = NativeAudioEngine.nativeDiscoveryFindDevices(handle)
        assertNotNull(devices)
        assertEquals(0, devices?.size)
    }

    // ═══════════════════════════════════════════════════
    // Encryption stubs
    // ═══════════════════════════════════════════════════

    @Test
    fun encryption_invalidHandle_returnsError() {
        assertEquals(-1, NativeAudioEngine.nativeIsEncryptionEnabled(0L))
    }

    @Test
    fun encryption_setEnabledOnValidHandle() {
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        assertTrue(handle != 0L)
        try {
            val key = ByteArray(16) { it.toByte() }
            val salt = ByteArray(14) { (it + 0x10).toByte() }
            val rc = NativeAudioEngine.nativeSetEncryptionEnabled(handle, true, key, salt)
            assertEquals(0, rc)
            assertEquals(1, NativeAudioEngine.nativeIsEncryptionEnabled(handle))
        } finally {
            NativeAudioEngine.nativeRelease(handle)
        }
    }
}
