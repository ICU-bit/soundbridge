//! SoundBridge 设备发现模块
//!
//! 提供 mDNS 设备发现和设备记忆功能。

pub mod device_store;

pub use device_store::{DeviceStore, StoredDevice};

use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::net::IpAddr;

/// 发现配置
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// 服务名称
    pub service_name: String,

    /// 服务类型
    pub service_type: String,

    /// 端口
    pub port: u16,

    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            service_name: "SoundBridge".to_string(),
            service_type: "_soundbridge._udp.local.".to_string(),
            port: 0,
            timeout_ms: 3000,
        }
    }
}

/// 发现错误类型
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("发现失败: {0}")]
    DiscoveryFailed(String),

    #[error("注册失败: {0}")]
    RegistrationFailed(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// 发现结果类型
pub type Result<T> = std::result::Result<T, DiscoveryError>;

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备名称
    pub name: String,

    /// 设备地址
    pub address: IpAddr,

    /// 端口
    pub port: u16,

    /// 主机名
    pub hostname: String,
}

/// 设备发现
pub struct DeviceDiscovery {
    config: DiscoveryConfig,
    mdns: Option<ServiceDaemon>,
}

impl DeviceDiscovery {
    /// 创建新的发现服务
    pub fn new(config: DiscoveryConfig) -> Self {
        Self { config, mdns: None }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(DiscoveryConfig::default())
    }

    /// 初始化 mDNS 守护进程
    pub fn init(&mut self) -> Result<()> {
        if self.mdns.is_none() {
            let mdns = ServiceDaemon::new().map_err(|e| {
                DiscoveryError::DiscoveryFailed(format!("Failed to create mDNS daemon: {}", e))
            })?;
            self.mdns = Some(mdns);
        }
        Ok(())
    }

    /// 注册服务
    pub fn register_service(&self, name: &str, port: u16) -> Result<()> {
        let mdns = self.mdns.as_ref().ok_or_else(|| {
            DiscoveryError::RegistrationFailed("mDNS not initialized".to_string())
        })?;

        let builder = mdns_sd::ServiceInfo::new(
            &self.config.service_type,
            name,
            &format!("{}.local.", name),
            "",
            port,
            None,
        )
        .map_err(|e| {
            DiscoveryError::RegistrationFailed(format!("Failed to create service info: {}", e))
        })?;

        mdns.register(builder).map_err(|e| {
            DiscoveryError::RegistrationFailed(format!("Failed to register service: {}", e))
        })?;

        Ok(())
    }

    /// 发现设备
    pub fn discover(&self) -> Result<Vec<DeviceInfo>> {
        let mdns = self
            .mdns
            .as_ref()
            .ok_or_else(|| DiscoveryError::DiscoveryFailed("mDNS not initialized".to_string()))?;

        let receiver = mdns
            .browse(&self.config.service_type)
            .map_err(|e| DiscoveryError::DiscoveryFailed(format!("Failed to browse: {}", e)))?;

        let mut devices = Vec::new();
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        while let Ok(event) = receiver.recv_timeout(timeout) {
            if let ServiceEvent::ServiceResolved(info) = event {
                let addresses = info.get_addresses();
                let hostname = info.get_hostname().to_string();
                let port = info.get_port();
                for addr in addresses {
                    devices.push(DeviceInfo {
                        name: hostname.clone(),
                        address: IpAddr::V4(*addr),
                        port,
                        hostname: hostname.clone(),
                    });
                }
            }
        }

        Ok(devices)
    }

    /// 停止发现
    pub fn stop(&self) -> Result<()> {
        if let Some(ref mdns) = self.mdns {
            mdns.shutdown().map_err(|e| {
                DiscoveryError::DiscoveryFailed(format!("Failed to shutdown: {}", e))
            })?;
        }
        Ok(())
    }

    /// 获取配置
    pub fn config(&self) -> &DiscoveryConfig {
        &self.config
    }
}

impl Drop for DeviceDiscovery {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let config = DiscoveryConfig::default();
        let _discovery = DeviceDiscovery::new(config);
    }

    #[test]
    fn test_device_info() {
        let info = DeviceInfo {
            name: "Test Device".to_string(),
            address: "192.168.1.100".parse().unwrap(),
            port: 12345,
            hostname: "test.local.".to_string(),
        };

        assert_eq!(info.name, "Test Device");
        assert_eq!(info.address, "192.168.1.100".parse::<IpAddr>().unwrap());
        assert_eq!(info.port, 12345);
    }

    #[test]
    fn test_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.service_name, "SoundBridge");
        assert_eq!(config.service_type, "_soundbridge._udp.local.");
        assert_eq!(config.timeout_ms, 3000);
    }

    #[test]
    fn test_device_store() {
        let mut store = DeviceStore::new();
        let addr: IpAddr = "192.168.1.100".parse().unwrap();

        store.add_device("Test Device", addr, 12345);
        assert_eq!(store.len(), 1);
        assert!(store.has_device("Test Device"));

        store.set_auto_connect("Test Device", true);
        let auto_devices = store.get_auto_connect_devices();
        assert_eq!(auto_devices.len(), 1);
    }
}
