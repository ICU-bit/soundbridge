//! 连接管理模块
//!
//! 提供连接状态管理、自动重连、心跳检测功能。
//! 支持多种连接方式：WiFi 局域网、WiFi 直连、USB/ADB、蓝牙。

use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// 连接类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// WiFi 局域网（默认，自动发现）
    WiFiLan,

    /// WiFi 直连（热点模式）
    WiFiDirect,

    /// USB 有线连接（ADB 端口转发）
    UsbAdb,

    /// 蓝牙连接
    Bluetooth,
}

impl Default for ConnectionType {
    fn default() -> Self {
        Self::WiFiLan
    }
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WiFiLan => write!(f, "WiFi LAN"),
            Self::WiFiDirect => write!(f, "WiFi Direct"),
            Self::UsbAdb => write!(f, "USB/ADB"),
            Self::Bluetooth => write!(f, "Bluetooth"),
        }
    }
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 未连接
    Disconnected,

    /// 连接中
    Connecting,

    /// 已连接
    Connected,

    /// 重连中
    Reconnecting,

    /// 连接错误
    Error,
}

/// 连接配置
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,

    /// 心跳超时（毫秒）
    pub heartbeat_timeout_ms: u64,

    /// 最大重连次数
    pub max_reconnect_attempts: u32,

    /// 重连间隔（毫秒）
    pub reconnect_interval_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 5000,
            heartbeat_timeout_ms: 10000,
            max_reconnect_attempts: 5,
            reconnect_interval_ms: 1000,
        }
    }
}

/// 连接管理器
pub struct ConnectionManager {
    /// 当前状态
    state: ConnectionState,

    /// 连接类型
    connection_type: ConnectionType,

    /// 配置
    config: ConnectionConfig,

    /// 远程地址
    remote_addr: Option<SocketAddr>,

    /// 最后心跳时间
    last_heartbeat: Option<Instant>,

    /// 重连尝试次数
    reconnect_attempts: u32,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::default(),
            config,
            remote_addr: None,
            last_heartbeat: None,
            reconnect_attempts: 0,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(ConnectionConfig::default())
    }

    /// 开始连接
    pub fn connect(&mut self, addr: SocketAddr) {
        self.state = ConnectionState::Connecting;
        self.connection_type = ConnectionType::WiFiLan;
        self.remote_addr = Some(addr);
        self.reconnect_attempts = 0;
    }

    /// 开始连接（指定连接类型）
    pub fn connect_with_type(&mut self, addr: SocketAddr, conn_type: ConnectionType) {
        self.state = ConnectionState::Connecting;
        self.connection_type = conn_type;
        self.remote_addr = Some(addr);
        self.reconnect_attempts = 0;
    }

    /// 连接成功
    pub fn connected(&mut self) {
        self.state = ConnectionState::Connected;
        self.last_heartbeat = Some(Instant::now());
        self.reconnect_attempts = 0;
    }

    /// 断开连接
    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.connection_type = ConnectionType::default();
        self.remote_addr = None;
        self.last_heartbeat = None;
        self.reconnect_attempts = 0;
    }

    /// 获取连接类型
    pub fn connection_type(&self) -> ConnectionType {
        self.connection_type
    }

    /// 更新心跳
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Some(Instant::now());
    }

    /// 检查心跳超时
    pub fn check_heartbeat_timeout(&self) -> bool {
        if let Some(last) = self.last_heartbeat {
            last.elapsed() > Duration::from_millis(self.config.heartbeat_timeout_ms)
        } else {
            false
        }
    }

    /// 尝试重连
    pub fn try_reconnect(&mut self) -> bool {
        if self.reconnect_attempts < self.config.max_reconnect_attempts {
            self.state = ConnectionState::Reconnecting;
            self.reconnect_attempts += 1;
            true
        } else {
            self.state = ConnectionState::Error;
            false
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// 获取远程地址
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// 获取配置
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// 获取重连尝试次数
    pub fn reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::with_default_config();
        assert_eq!(manager.state(), ConnectionState::Disconnected);
        assert!(!manager.is_connected());
        assert!(manager.remote_addr().is_none());
    }

    #[test]
    fn test_connection_lifecycle() {
        let mut manager = ConnectionManager::with_default_config();
        let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();

        manager.connect(addr);
        assert_eq!(manager.state(), ConnectionState::Connecting);
        assert_eq!(manager.remote_addr(), Some(addr));

        manager.connected();
        assert_eq!(manager.state(), ConnectionState::Connected);
        assert!(manager.is_connected());

        manager.disconnect();
        assert_eq!(manager.state(), ConnectionState::Disconnected);
        assert!(!manager.is_connected());
    }

    #[test]
    fn test_heartbeat() {
        let mut manager = ConnectionManager::with_default_config();
        let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();

        manager.connect(addr);
        manager.connected();

        // 刚连接时不应超时
        assert!(!manager.check_heartbeat_timeout());

        // 更新心跳
        manager.update_heartbeat();
        assert!(!manager.check_heartbeat_timeout());
    }

    #[test]
    fn test_reconnect() {
        let config = ConnectionConfig {
            max_reconnect_attempts: 3,
            ..Default::default()
        };
        let mut manager = ConnectionManager::new(config);

        // 前 3 次应该成功
        assert!(manager.try_reconnect());
        assert_eq!(manager.state(), ConnectionState::Reconnecting);
        assert_eq!(manager.reconnect_attempts(), 1);

        assert!(manager.try_reconnect());
        assert_eq!(manager.reconnect_attempts(), 2);

        assert!(manager.try_reconnect());
        assert_eq!(manager.reconnect_attempts(), 3);

        // 第 4 次应该失败
        assert!(!manager.try_reconnect());
        assert_eq!(manager.state(), ConnectionState::Error);
    }

    #[test]
    fn test_connection_config() {
        let config = ConnectionConfig::default();
        assert_eq!(config.heartbeat_interval_ms, 5000);
        assert_eq!(config.heartbeat_timeout_ms, 10000);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert_eq!(config.reconnect_interval_ms, 1000);
    }

    #[test]
    fn test_connection_type_default() {
        assert_eq!(ConnectionType::default(), ConnectionType::WiFiLan);
    }

    #[test]
    fn test_connection_type_display() {
        assert_eq!(format!("{}", ConnectionType::WiFiLan), "WiFi LAN");
        assert_eq!(format!("{}", ConnectionType::WiFiDirect), "WiFi Direct");
        assert_eq!(format!("{}", ConnectionType::UsbAdb), "USB/ADB");
        assert_eq!(format!("{}", ConnectionType::Bluetooth), "Bluetooth");
    }

    #[test]
    fn test_connect_with_type() {
        let mut manager = ConnectionManager::with_default_config();
        let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();

        manager.connect_with_type(addr, ConnectionType::UsbAdb);
        assert_eq!(manager.state(), ConnectionState::Connecting);
        assert_eq!(manager.connection_type(), ConnectionType::UsbAdb);
        assert_eq!(manager.remote_addr(), Some(addr));
    }

    #[test]
    fn test_connection_type_lifecycle() {
        let mut manager = ConnectionManager::with_default_config();
        let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();

        // 默认连接使用 WiFiLan
        manager.connect(addr);
        assert_eq!(manager.connection_type(), ConnectionType::WiFiLan);

        manager.connected();
        assert_eq!(manager.connection_type(), ConnectionType::WiFiLan);

        // 断开后重置为默认
        manager.disconnect();
        assert_eq!(manager.connection_type(), ConnectionType::WiFiLan);
    }

    #[test]
    fn test_connection_type_clone_copy() {
        let conn_type = ConnectionType::WiFiDirect;
        let cloned = conn_type;
        assert_eq!(conn_type, cloned);
    }

    #[test]
    fn test_connection_type_eq() {
        assert_eq!(ConnectionType::WiFiLan, ConnectionType::WiFiLan);
        assert_ne!(ConnectionType::WiFiLan, ConnectionType::UsbAdb);
    }
}
