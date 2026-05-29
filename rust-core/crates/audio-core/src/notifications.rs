//! 系统通知配置模块
//!
//! 提供跨平台系统通知的配置接口。

/// 通知类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    /// 连接状态变化
    ConnectionChanged,

    /// 音频状态变化
    AudioChanged,

    /// 设备变化
    DeviceChanged,

    /// 错误通知
    Error,
}

/// 通知配置
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// 是否启用通知
    pub enabled: bool,

    /// 通知标题
    pub title: String,

    /// 通知内容
    pub body: String,

    /// 通知类型
    pub notification_type: NotificationType,
}

/// 通知管理器 trait
pub trait NotificationManager {
    /// 发送通知
    fn send(&self, config: &NotificationConfig) -> std::result::Result<(), String>;

    /// 检查是否支持通知
    fn is_supported(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type() {
        assert_eq!(NotificationType::ConnectionChanged, NotificationType::ConnectionChanged);
        assert_ne!(NotificationType::ConnectionChanged, NotificationType::AudioChanged);
    }

    #[test]
    fn test_notification_config() {
        let config = NotificationConfig {
            enabled: true,
            title: "连接成功".to_string(),
            body: "已连接到设备".to_string(),
            notification_type: NotificationType::ConnectionChanged,
        };
        assert!(config.enabled);
        assert_eq!(config.title, "连接成功");
        assert_eq!(config.body, "已连接到设备");
    }
}
