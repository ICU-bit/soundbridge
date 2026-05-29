//! SoundBridge 音频播放模块
//!
//! 提供跨平台音频播放功能，基于 cpal 库实现。

pub mod device;
pub mod config;

pub use device::{PlaybackDevice, DeviceInfo};
pub use config::PlaybackConfig;

/// 音频播放错误类型
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("设备未找到: {0}")]
    DeviceNotFound(String),

    #[error("配置不支持: {0}")]
    ConfigNotSupported(String),

    #[error("流错误: {0}")]
    StreamError(String),

    #[error("cpal 错误: {0}")]
    CpalError(#[from] cpal::DefaultStreamConfigError),

    #[error("构建流错误: {0}")]
    BuildStreamError(#[from] cpal::BuildStreamError),

    #[error("播放流错误: {0}")]
    PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("设备不可用")]
    DeviceUnavailable,
}

/// 音频播放结果类型
pub type Result<T> = std::result::Result<T, PlaybackError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let devices = PlaybackDevice::list_devices().unwrap();
        // 至少应该有一个输出设备
        println!("输出设备数量: {}", devices.len());
        for device in &devices {
            println!("  - {}", device.name);
        }
    }

    #[test]
    fn test_default_config() {
        let config = PlaybackConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.buffer_size, 960);
    }
}
