//! SoundBridge 音频采集模块
//!
//! 提供跨平台音频采集功能，基于 cpal 库实现。

pub mod config;
pub mod device;

pub use config::CaptureConfig;
pub use device::{CaptureDevice, DeviceInfo};

/// 音频采集错误类型
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
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

/// 音频采集结果类型
pub type Result<T> = std::result::Result<T, CaptureError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let devices = CaptureDevice::list_devices().unwrap();
        // 至少应该有一个输入设备
        println!("输入设备数量: {}", devices.len());
        for device in &devices {
            println!("  - {}", device.name);
        }
    }

    #[test]
    fn test_default_config() {
        let config = CaptureConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.buffer_size, 960);
    }
}
