package com.soundbridge.audio

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * mDNS 设备发现管理器
 *
 * 使用 Android NsdManager 实现 _soundbridge._udp 服务发现。
 * 替代 JNI 存根实现，提供真正的 mDNS 设备扫描功能。
 */
class DeviceDiscoveryManager(private val context: Context) {

    companion object {
        private const val TAG = "DeviceDiscovery"
        private const val SERVICE_TYPE = "_soundbridge._udp.local."
    }

    /** 已发现的设备列表 */
    private val _discoveredDevices = MutableStateFlow<List<DiscoveredDevice>>(emptyList())
    val discoveredDevices: StateFlow<List<DiscoveredDevice>> = _discoveredDevices

    /** 是否正在扫描 */
    private val _isScanning = MutableStateFlow(false)
    val isScanning: StateFlow<Boolean> = _isScanning

    private var nsdManager: NsdManager? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null

    /**
     * 开始扫描设备
     */
    fun startDiscovery() {
        if (_isScanning.value) return

        nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
        _discoveredDevices.value = emptyList()
        _isScanning.value = true

        discoveryListener = createDiscoveryListener()
        nsdManager?.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discoveryListener)

        Log.i(TAG, "Started mDNS discovery for $SERVICE_TYPE")
    }

    /**
     * 停止扫描设备
     */
    fun stopDiscovery() {
        if (!_isScanning.value) return

        try {
            discoveryListener?.let { nsdManager?.stopServiceDiscovery(it) }
        } catch (e: Exception) {
            Log.w(TAG, "Error stopping discovery: ${e.message}")
        }
        discoveryListener = null
        _isScanning.value = false

        Log.i(TAG, "Stopped mDNS discovery")
    }

    /**
     * 创建发现监听器
     */
    private fun createDiscoveryListener(): NsdManager.DiscoveryListener {
        return object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String) {
                Log.d(TAG, "Discovery started: $serviceType")
            }

            override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                Log.d(TAG, "Service found: ${serviceInfo.serviceName}")
                // 解析服务获取详细信息
                nsdManager?.resolveService(serviceInfo, createResolveListener())
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                Log.d(TAG, "Service lost: ${serviceInfo.serviceName}")
                _discoveredDevices.value = _discoveredDevices.value.filter {
                    it.name != serviceInfo.serviceName
                }
            }

            override fun onDiscoveryStopped(serviceType: String) {
                Log.d(TAG, "Discovery stopped: $serviceType")
                _isScanning.value = false
            }

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Start discovery failed: $errorCode")
                _isScanning.value = false
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Stop discovery failed: $errorCode")
            }
        }
    }

    /**
     * 创建服务解析监听器
     */
    private fun createResolveListener(): NsdManager.ResolveListener {
        return object : NsdManager.ResolveListener {
            override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                Log.e(TAG, "Resolve failed: ${serviceInfo.serviceName}, error: $errorCode")
            }

            override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                val host = serviceInfo.host
                val port = serviceInfo.port
                val name = serviceInfo.serviceName

                if (host != null) {
                    val device = DiscoveredDevice(
                        name = name,
                        address = host.hostAddress ?: "unknown",
                        port = port,
                        hostname = host.hostName ?: "unknown"
                    )

                    Log.i(TAG, "Resolved: $name at ${device.address}:${device.port}")

                    // 避免重复添加
                    val currentList = _discoveredDevices.value.toMutableList()
                    val existingIndex = currentList.indexOfFirst { it.name == name }
                    if (existingIndex >= 0) {
                        currentList[existingIndex] = device
                    } else {
                        currentList.add(device)
                    }
                    _discoveredDevices.value = currentList
                }
            }
        }
    }

    /**
     * 释放资源
     */
    fun release() {
        stopDiscovery()
        nsdManager = null
    }
}

/**
 * 已发现的设备信息
 */
data class DiscoveredDevice(
    val name: String,
    val address: String,
    val port: Int,
    val hostname: String
)
