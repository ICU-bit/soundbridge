//! 音频采集配置

/// 音频采集配置
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// 采样率（Hz），默认 48000
    pub sample_rate: u32,

    /// 通道数，默认 2（立体声）
    pub channels: u16,

    /// 缓冲区大小（samples），默认 960（20ms @ 48kHz）
    pub buffer_size: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_size: 960,
        }
    }
}

impl CaptureConfig {
    /// 创建新的配置
    pub fn new(sample_rate: u32, channels: u16, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            channels,
            buffer_size,
        }
    }

    /// 获取帧时长（毫秒）
    pub fn frame_duration_ms(&self) -> f64 {
        self.buffer_size as f64 / self.sample_rate as f64 * 1000.0
    }

    /// 每帧字节数（f32 格式）
    pub fn frame_bytes(&self) -> usize {
        self.buffer_size as usize * self.channels as usize * std::mem::size_of::<f32>()
    }
}
