//! 全局快捷键配置模块
//!
//! 提供跨平台全局快捷键的配置接口。

/// 快捷键动作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyAction {
    /// 切换混音模式
    ToggleMixMode,

    /// 切换传输方向
    ToggleDirection,

    /// 暂停/恢复
    TogglePause,

    /// 打开设置
    OpenSettings,

    /// 音量增加
    VolumeUp,

    /// 音量减少
    VolumeDown,
}

/// 快捷键配置
#[derive(Debug, Clone)]
pub struct HotkeyConfig {
    /// 快捷键标识符
    pub id: String,

    /// 快捷键描述
    pub description: String,

    /// 触发动作
    pub action: HotkeyAction,

    /// 是否启用
    pub enabled: bool,
}

/// 快捷键管理器 trait
pub trait HotkeyManager {
    /// 注册快捷键
    fn register(&mut self, config: &HotkeyConfig) -> std::result::Result<(), String>;

    /// 注销快捷键
    fn unregister(&mut self, id: &str) -> std::result::Result<(), String>;

    /// 检查快捷键是否已注册
    fn is_registered(&self, id: &str) -> bool;

    /// 获取所有已注册的快捷键
    fn list_hotkeys(&self) -> Vec<&HotkeyConfig>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_action_variants() {
        assert_eq!(HotkeyAction::ToggleMixMode, HotkeyAction::ToggleMixMode);
        assert_ne!(HotkeyAction::ToggleMixMode, HotkeyAction::ToggleDirection);
    }

    #[test]
    fn test_hotkey_config() {
        let config = HotkeyConfig {
            id: "toggle_mix".to_string(),
            description: "Toggle mix mode".to_string(),
            action: HotkeyAction::ToggleMixMode,
            enabled: true,
        };
        assert_eq!(config.id, "toggle_mix");
        assert_eq!(config.description, "Toggle mix mode");
        assert_eq!(config.action, HotkeyAction::ToggleMixMode);
        assert!(config.enabled);
    }
}
