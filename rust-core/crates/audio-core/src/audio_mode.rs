//! 音频模式管理模块
//!
//! 提供音频模式切换功能：均衡模式、高音质模式、超低延迟模式。

/// 音频模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioMode {
    /// 均衡模式（50-100ms 延迟）
    Balanced,

    /// 高音质模式（48kHz/24bit）
    HighQuality,

    /// 超低延迟模式（<30ms）
    LowLatency,
}

/// 音频模式配置
#[derive(Debug, Clone)]
pub struct AudioModeConfig {
    /// 模式
    pub mode: AudioMode,

    /// 采样率
    pub sample_rate: u32,

    /// 比特率
    pub bitrate: u32,

    /// 帧大小（毫秒）
    pub frame_size_ms: u32,

    /// 编码复杂度
    pub complexity: u32,
}

impl AudioModeConfig {
    /// 创建均衡模式配置（对齐技术规格）
    pub fn balanced() -> Self {
        Self {
            mode: AudioMode::Balanced,
            sample_rate: 44100,
            bitrate: 128000,
            frame_size_ms: 20,
            complexity: 5,
        }
    }

    /// 创建高音质模式配置（对齐技术规格）
    pub fn high_quality() -> Self {
        Self {
            mode: AudioMode::HighQuality,
            sample_rate: 48000,
            bitrate: 256000,
            frame_size_ms: 20,
            complexity: 10,
        }
    }

    /// 创建超低延迟模式配置（对齐技术规格）
    pub fn low_latency() -> Self {
        Self {
            mode: AudioMode::LowLatency,
            sample_rate: 44100,
            bitrate: 64000,
            frame_size_ms: 10,
            complexity: 3,
        }
    }

    /// 获取帧大小（样本数）
    pub fn frame_size_samples(&self) -> usize {
        (self.sample_rate * self.frame_size_ms / 1000) as usize
    }
}

/// 音频模式管理器
pub struct AudioModeManager {
    current_mode: AudioMode,
    current_config: AudioModeConfig,
}

impl AudioModeManager {
    /// 创建新的音频模式管理器
    pub fn new() -> Self {
        Self {
            current_mode: AudioMode::Balanced,
            current_config: AudioModeConfig::balanced(),
        }
    }

    /// 切换模式
    pub fn switch_mode(&mut self, mode: AudioMode) {
        self.current_mode = mode;
        self.current_config = match mode {
            AudioMode::Balanced => AudioModeConfig::balanced(),
            AudioMode::HighQuality => AudioModeConfig::high_quality(),
            AudioMode::LowLatency => AudioModeConfig::low_latency(),
        };
    }

    /// 获取当前模式
    pub fn current_mode(&self) -> AudioMode {
        self.current_mode
    }

    /// 获取当前配置
    pub fn current_config(&self) -> &AudioModeConfig {
        &self.current_config
    }
}

impl Default for AudioModeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_mode_config() {
        let balanced = AudioModeConfig::balanced();
        assert_eq!(balanced.mode, AudioMode::Balanced);
        assert_eq!(balanced.sample_rate, 44100);
        assert_eq!(balanced.bitrate, 128000);
        assert_eq!(balanced.frame_size_ms, 20);

        let high_quality = AudioModeConfig::high_quality();
        assert_eq!(high_quality.mode, AudioMode::HighQuality);
        assert_eq!(high_quality.bitrate, 256000);

        let low_latency = AudioModeConfig::low_latency();
        assert_eq!(low_latency.mode, AudioMode::LowLatency);
        assert_eq!(low_latency.frame_size_ms, 10);
        assert_eq!(low_latency.complexity, 3);
    }

    #[test]
    fn test_frame_size_samples() {
        let balanced = AudioModeConfig::balanced();
        assert_eq!(balanced.frame_size_samples(), 882); // 44100 * 20 / 1000

        let low_latency = AudioModeConfig::low_latency();
        assert_eq!(low_latency.frame_size_samples(), 441); // 44100 * 10 / 1000
    }

    #[test]
    fn test_mode_manager() {
        let mut manager = AudioModeManager::new();

        assert_eq!(manager.current_mode(), AudioMode::Balanced);

        manager.switch_mode(AudioMode::HighQuality);
        assert_eq!(manager.current_mode(), AudioMode::HighQuality);
        assert_eq!(manager.current_config().bitrate, 256000);

        manager.switch_mode(AudioMode::LowLatency);
        assert_eq!(manager.current_mode(), AudioMode::LowLatency);
        assert_eq!(manager.current_config().frame_size_ms, 10);
    }
}
