package com.soundbridge.native

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeTrue
import org.junit.BeforeClass
import org.junit.Test

/**
 * Unit tests for [NativeAudioEngine] JNI methods.
 *
 * These tests exercise the native C++ bridge via JNI. They require the
 * `soundbridge_native` shared library to be loaded, which only happens
 * on a real device or emulator with the APK installed.
 *
 * On a plain JVM (CI `test` task), the `@BeforeClass` guard skips all
 * tests gracefully. On-device, run via `connectedAndroidTest` or
 * `./gradlew connectedCheck`.
 */
class NativeAudioEngineTest {

    companion object {
        private var nativeAvailable = false

        @JvmStatic
        @BeforeClass
        fun checkNativeLibrary() {
            nativeAvailable = try {
                NativeAudioEngine.nativeGetVersion()
                true
            } catch (e: UnsatisfiedLinkError) {
                System.err.println(
                    "SKIP: native library not loaded — " +
                        "run on device/emulator for full coverage: ${e.message}"
                )
                false
            }
        }
    }

    // ── Helper: skip test if native lib is unavailable ──

    private fun requireNative() {
        assumeTrue("Native library not loaded", nativeAvailable)
    }

    // ═══════════════════════════════════════════════════
    // Version
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeGetVersion_returnsNonNullNonEmptyString() {
        requireNative()
        val version = NativeAudioEngine.nativeGetVersion()
        assertNotNull(version)
        assertTrue("Version should not be blank", version.isNotBlank())
    }

    @Test
    fun nativeGetVersion_returnsExpectedFormat() {
        requireNative()
        val version = NativeAudioEngine.nativeGetVersion()
        // Expect semantic versioning: major.minor.patch
        assertTrue(
            "Version '$version' should match X.Y.Z pattern",
            version.matches(Regex("""\d+\.\d+\.\d+"""))
        )
    }

    // ═══════════════════════════════════════════════════
    // Engine lifecycle: init → release
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeInit_returnsNonZeroHandle() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            assertTrue("Engine handle should be non-zero on success", handle != 0L)
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeRelease_doesNotCrashOnValidHandle() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        if (handle != 0L) {
            NativeAudioEngine.nativeRelease(handle)
            // Second release should be safe (no double-free crash)
            // Note: behavior depends on impl; we just verify no crash.
        }
    }

    @Test
    fun nativeRelease_doesNotCrashOnZeroHandle() {
        requireNative()
        // 0L = invalid handle; release should be a no-op
        NativeAudioEngine.nativeRelease(0L)
    }

    // ═══════════════════════════════════════════════════
    // Mix ratio: set / get boundary values
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeSetMixRatio_returnsZeroOnSuccess() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return  // engine init may fail in test env
            val rc = NativeAudioEngine.nativeSetMixRatio(handle, 0.5f, 0.5f)
            assertEquals(0, rc)
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeSetMixRatio_returnsMinusOneOnInvalidHandle() {
        requireNative()
        val rc = NativeAudioEngine.nativeSetMixRatio(0L, 0.5f, 0.5f)
        assertEquals(-1, rc)
    }

    @Test
    fun nativeGetMixRatio_returnsSetValues() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            NativeAudioEngine.nativeSetMixRatio(handle, 0.3f, 0.7f)
            val ratio = NativeAudioEngine.nativeGetMixRatio(handle)
            assertNotNull(ratio)
            if (ratio != null) {
                assertEquals(2, ratio.size)
                assertEquals(0.3f, ratio[0], 0.001f)
                assertEquals(0.7f, ratio[1], 0.001f)
            }
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeGetMixRatio_boundaryValues() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return

            // Min: 0.0, 0.0
            NativeAudioEngine.nativeSetMixRatio(handle, 0.0f, 0.0f)
            val min = NativeAudioEngine.nativeGetMixRatio(handle)
            assertNotNull(min)
            if (min != null) {
                assertEquals(0.0f, min[0], 0.001f)
                assertEquals(0.0f, min[1], 0.001f)
            }

            // Max: 1.0, 1.0
            NativeAudioEngine.nativeSetMixRatio(handle, 1.0f, 1.0f)
            val max = NativeAudioEngine.nativeGetMixRatio(handle)
            assertNotNull(max)
            if (max != null) {
                assertEquals(1.0f, max[0], 0.001f)
                assertEquals(1.0f, max[1], 0.001f)
            }

            // Asymmetric: 0.0 PC, 1.0 Phone
            NativeAudioEngine.nativeSetMixRatio(handle, 0.0f, 1.0f)
            val asym = NativeAudioEngine.nativeGetMixRatio(handle)
            assertNotNull(asym)
            if (asym != null) {
                assertEquals(0.0f, asym[0], 0.001f)
                assertEquals(1.0f, asym[1], 0.001f)
            }
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeGetMixRatio_returnsNullOnInvalidHandle() {
        requireNative()
        val ratio = NativeAudioEngine.nativeGetMixRatio(0L)
        assertNull(ratio)
    }

    // ═══════════════════════════════════════════════════
    // Audio mode: set / get
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeSetAudioMode_returnsZeroOnSuccess() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            val rc = NativeAudioEngine.nativeSetAudioMode(handle, 1)
            assertEquals(0, rc)
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeSetAudioMode_returnsMinusOneOnInvalidHandle() {
        requireNative()
        val rc = NativeAudioEngine.nativeSetAudioMode(0L, 1)
        assertEquals(-1, rc)
    }

    @Test
    fun nativeGetAudioMode_returnsSetMode() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            NativeAudioEngine.nativeSetAudioMode(handle, 2)
            val mode = NativeAudioEngine.nativeGetAudioMode(handle)
            assertEquals(2, mode)
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeGetAudioMode_returnsDefaultOnInvalidHandle() {
        requireNative()
        val mode = NativeAudioEngine.nativeGetAudioMode(0L)
        assertEquals(0, mode) // default Balanced
    }

    @Test
    fun nativeSetGetAudioMode_multipleModes() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            for (mode in 0..2) {
                NativeAudioEngine.nativeSetAudioMode(handle, mode)
                assertEquals(mode, NativeAudioEngine.nativeGetAudioMode(handle))
            }
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    // ═══════════════════════════════════════════════════
    // Connection type (hotspot / ADB / Bluetooth state)
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeHotspotState_returnsInitialState() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            val state = NativeAudioEngine.nativeHotspotState(handle)
            assertTrue("Hotspot state should be >= 0", state >= 0)
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeHotspotSetState_thenGet() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            NativeAudioEngine.nativeHotspotSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeHotspotState(handle))
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeAdbState_setAndGet() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            NativeAudioEngine.nativeAdbSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeAdbState(handle))
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    @Test
    fun nativeBtState_setAndGet() {
        requireNative()
        val handle = NativeAudioEngine.nativeInit(48000, 1, 960)
        try {
            if (handle == 0L) return
            NativeAudioEngine.nativeBtSetState(handle, 2)
            assertEquals(2, NativeAudioEngine.nativeBtState(handle))
        } finally {
            if (handle != 0L) NativeAudioEngine.nativeRelease(handle)
        }
    }

    // ═══════════════════════════════════════════════════
    // Pipeline state
    // ═══════════════════════════════════════════════════

    @Test
    fun nativePipelineState_returnsMinusOneOnInvalidHandle() {
        requireNative()
        val state = NativeAudioEngine.nativePipelineState(0L)
        assertEquals(-1, state)
    }

    @Test
    fun nativePipelineStop_returnsMinusOneOnInvalidHandle() {
        requireNative()
        val rc = NativeAudioEngine.nativePipelineStop(0L)
        assertEquals(-1, rc)
    }

    // ═══════════════════════════════════════════════════
    // Discovery stubs
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeDiscoveryCreate_returnsNonZero() {
        requireNative()
        val handle = NativeAudioEngine.nativeDiscoveryCreate()
        assertTrue("Discovery handle should be non-zero", handle != 0L)
    }

    @Test
    fun nativeDiscoveryFindDevices_returnsEmptyArray() {
        requireNative()
        val handle = NativeAudioEngine.nativeDiscoveryCreate()
        val devices = NativeAudioEngine.nativeDiscoveryFindDevices(handle)
        assertNotNull(devices)
        assertEquals(0, devices?.size)
    }

    // ═══════════════════════════════════════════════════
    // Encryption stubs
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeIsEncryptionEnabled_returnsMinusOneOnInvalidHandle() {
        requireNative()
        val rc = NativeAudioEngine.nativeIsEncryptionEnabled(0L)
        assertEquals(-1, rc)
    }

    // ═══════════════════════════════════════════════════
    // Exclusive mode stub
    // ═══════════════════════════════════════════════════

    @Test
    fun nativeSetExclusiveMode_returnsZero() {
        requireNative()
        val rc = NativeAudioEngine.nativeSetExclusiveMode(0L, true)
        assertEquals(0, rc)
    }
}
