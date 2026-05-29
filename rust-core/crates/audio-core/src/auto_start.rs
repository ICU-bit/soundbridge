//! 启动自启配置模块
//!
//! 提供跨平台启动自启功能的配置接口。

/// 启动自启配置
#[derive(Debug, Clone)]
pub struct AutoStartConfig {
    /// 是否启用启动自启
    pub enabled: bool,

    /// 启动延迟（秒）
    pub startup_delay_secs: u32,

    /// 是否最小化启动
    pub start_minimized: bool,
}

impl Default for AutoStartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            startup_delay_secs: 0,
            start_minimized: true,
        }
    }
}

/// 启动自启管理器 trait
pub trait AutoStartManager {
    /// 启用启动自启
    fn enable(&mut self) -> std::result::Result<(), String>;

    /// 禁用启动自启
    fn disable(&mut self) -> std::result::Result<(), String>;

    /// 检查是否已启用
    fn is_enabled(&self) -> bool;

    /// 获取配置
    fn config(&self) -> &AutoStartConfig;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_start_config_default() {
        let config = AutoStartConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.startup_delay_secs, 0);
        assert!(config.start_minimized);
    }

    #[test]
    fn test_auto_start_config_custom() {
        let config = AutoStartConfig {
            enabled: true,
            startup_delay_secs: 5,
            start_minimized: false,
        };
        assert!(config.enabled);
        assert_eq!(config.startup_delay_secs, 5);
        assert!(!config.start_minimized);
    }
}
