//! 设备记忆存储模块
//!
//! 提供设备记忆功能，记住已连接过的设备，支持 JSON 文件持久化。

use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// 已存储的设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDevice {
    /// 设备名称
    pub name: String,

    /// 设备地址（序列化为字符串）
    pub address: String,

    /// 端口
    pub port: u16,

    /// 最后连接时间（序列化为时间戳）
    pub last_connected_secs: u64,

    /// 连接次数
    pub connection_count: u32,

    /// 是否自动连接
    pub auto_connect: bool,
}

/// 持久化存储格式
#[derive(Serialize, Deserialize)]
struct DeviceStoreData {
    devices: Vec<StoredDevice>,
}

/// 设备存储
pub struct DeviceStore {
    /// 已存储的设备列表
    devices: HashMap<String, StoredDevice>,

    /// 持久化文件路径
    file_path: Option<PathBuf>,
}

impl DeviceStore {
    /// 创建新的设备存储
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            file_path: None,
        }
    }

    /// 创建带持久化的设备存储
    pub fn with_file(path: &Path) -> Self {
        let mut store = Self {
            devices: HashMap::new(),
            file_path: Some(path.to_path_buf()),
        };
        let _ = store.load();
        store
    }

    /// 从文件加载
    fn load(&mut self) -> Result<(), String> {
        let path = match &self.file_path {
            Some(p) => p,
            None => return Ok(()),
        };

        if !path.exists() {
            return Ok(());
        }

        let data = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read device store: {}", e))?;

        let store_data: DeviceStoreData = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse device store: {}", e))?;

        self.devices.clear();
        for device in store_data.devices {
            self.devices.insert(device.name.clone(), device);
        }

        Ok(())
    }

    /// 保存到文件
    pub fn save(&self) -> Result<(), String> {
        let path = match &self.file_path {
            Some(p) => p,
            None => return Ok(()),
        };

        let store_data = DeviceStoreData {
            devices: self.devices.values().cloned().collect(),
        };

        let data = serde_json::to_string_pretty(&store_data)
            .map_err(|e| format!("Failed to serialize device store: {}", e))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        fs::write(path, data)
            .map_err(|e| format!("Failed to write device store: {}", e))?;

        Ok(())
    }

    /// 添加或更新设备
    pub fn add_device(&mut self, name: &str, address: IpAddr, port: u16) {
        let entry = self.devices.entry(name.to_string()).or_insert_with(|| StoredDevice {
            name: name.to_string(),
            address: address.to_string(),
            port,
            last_connected_secs: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            connection_count: 0,
            auto_connect: false,
        });

        entry.address = address.to_string();
        entry.port = port;
        entry.last_connected_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        entry.connection_count += 1;

        let _ = self.save();
    }

    /// 获取设备信息
    pub fn get_device(&self, name: &str) -> Option<&StoredDevice> {
        self.devices.get(name)
    }

    /// 获取所有设备
    pub fn get_all_devices(&self) -> Vec<&StoredDevice> {
        self.devices.values().collect()
    }

    /// 获取自动连接的设备
    pub fn get_auto_connect_devices(&self) -> Vec<&StoredDevice> {
        self.devices.values()
            .filter(|d| d.auto_connect)
            .collect()
    }

    /// 设置设备自动连接
    pub fn set_auto_connect(&mut self, name: &str, auto_connect: bool) {
        if let Some(device) = self.devices.get_mut(name) {
            device.auto_connect = auto_connect;
            let _ = self.save();
        }
    }

    /// 删除设备
    pub fn remove_device(&mut self, name: &str) -> bool {
        let removed = self.devices.remove(name).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// 清除所有设备
    pub fn clear(&mut self) {
        self.devices.clear();
        let _ = self.save();
    }

    /// 获取设备数量
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// 检查设备是否存在
    pub fn has_device(&self, name: &str) -> bool {
        self.devices.contains_key(name)
    }
}

impl Default for DeviceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_device_store_creation() {
        let store = DeviceStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_add_device() {
        let mut store = DeviceStore::new();
        let addr: IpAddr = "192.168.1.100".parse().unwrap();

        store.add_device("Test Device", addr, 12345);

        assert_eq!(store.len(), 1);
        assert!(store.has_device("Test Device"));

        let device = store.get_device("Test Device").unwrap();
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.address, "192.168.1.100");
        assert_eq!(device.port, 12345);
        assert_eq!(device.connection_count, 1);
    }

    #[test]
    fn test_update_device() {
        let mut store = DeviceStore::new();
        let addr1: IpAddr = "192.168.1.100".parse().unwrap();
        let addr2: IpAddr = "192.168.1.200".parse().unwrap();

        store.add_device("Test Device", addr1, 12345);
        store.add_device("Test Device", addr2, 54321);

        assert_eq!(store.len(), 1);

        let device = store.get_device("Test Device").unwrap();
        assert_eq!(device.address, "192.168.1.200");
        assert_eq!(device.port, 54321);
        assert_eq!(device.connection_count, 2);
    }

    #[test]
    fn test_auto_connect() {
        let mut store = DeviceStore::new();
        let addr: IpAddr = "192.168.1.100".parse().unwrap();

        store.add_device("Test Device", addr, 12345);
        store.set_auto_connect("Test Device", true);

        let auto_devices = store.get_auto_connect_devices();
        assert_eq!(auto_devices.len(), 1);
        assert_eq!(auto_devices[0].name, "Test Device");
    }

    #[test]
    fn test_remove_device() {
        let mut store = DeviceStore::new();
        let addr: IpAddr = "192.168.1.100".parse().unwrap();

        store.add_device("Test Device", addr, 12345);
        assert!(store.has_device("Test Device"));

        assert!(store.remove_device("Test Device"));
        assert!(!store.has_device("Test Device"));
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut store = DeviceStore::new();
        let addr: IpAddr = "192.168.1.100".parse().unwrap();

        store.add_device("Device 1", addr, 12345);
        store.add_device("Device 2", addr, 54321);
        assert_eq!(store.len(), 2);

        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn test_persistence() {
        let temp_dir = std::env::temp_dir().join("soundbridge_test");
        let file_path = temp_dir.join("devices.json");

        // 清理
        let _ = fs::remove_file(&file_path);

        // 创建并保存
        {
            let mut store = DeviceStore::with_file(&file_path);
            let addr: IpAddr = "192.168.1.100".parse().unwrap();
            store.add_device("Test Device", addr, 12345);
            store.set_auto_connect("Test Device", true);
        }

        // 重新加载验证
        {
            let store = DeviceStore::with_file(&file_path);
            assert_eq!(store.len(), 1);
            let device = store.get_device("Test Device").unwrap();
            assert_eq!(device.address, "192.168.1.100");
            assert_eq!(device.port, 12345);
            assert!(device.auto_connect);
        }

        // 清理
        let _ = fs::remove_file(&file_path);
        let _ = fs::remove_dir(&temp_dir);
    }
}
