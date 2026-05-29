//! 音频管线模块
//!
//! 定义音频处理管道的接口和类型。
//! 实际实现依赖 audio-capture、audio-playback、audio-codec、audio-mixer 等 crate。

/// 管线配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 采样率
    pub sample_rate: u32,

    /// 通道数
    pub channels: u16,

    /// 帧大小（样本数）
    pub frame_size: usize,

    /// 比特率（bps）
    pub bitrate: u32,

    /// 混音模式
    pub mix_mode: MixMode,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            frame_size: 960,
            bitrate: 128000,
            mix_mode: MixMode::Mix,
        }
    }
}

/// 混音模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixMode {
    /// 混音（本地 + 远端）
    Mix,
    /// 只听本地
    LocalOnly,
    /// 只听远端
    RemoteOnly,
}

/// 管线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// 停止
    Stopped,
    /// 运行中
    Running,
    /// 错误
    Error,
}

/// 管线统计
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// 采集帧数
    pub frames_captured: u64,

    /// 播放帧数
    pub frames_played: u64,

    /// 编码帧数
    pub frames_encoded: u64,

    /// 解码帧数
    pub frames_decoded: u64,

    /// 丢帧数
    pub frames_dropped: u64,

    /// 当前延迟（毫秒）
    pub latency_ms: f32,
}

/// 管线错误
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("采集错误: {0}")]
    CaptureError(String),

    #[error("播放错误: {0}")]
    PlaybackError(String),

    #[error("编码错误: {0}")]
    EncodingError(String),

    #[error("解码错误: {0}")]
    DecodingError(String),

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),
}

/// 音频管线 trait
///
/// 定义音频管线的核心接口。
/// 实现此 trait 的类型需要协调采集、编码、网络、解码、播放等组件。
pub trait AudioPipeline {
    /// 启动管线
    fn start(&mut self) -> std::result::Result<(), PipelineError>;

    /// 停止管线
    fn stop(&mut self);

    /// 获取状态
    fn state(&self) -> PipelineState;

    /// 获取统计
    fn stats(&self) -> PipelineStats;

    /// 获取配置
    fn config(&self) -> &PipelineConfig;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.frame_size, 960);
        assert_eq!(config.bitrate, 128000);
        assert_eq!(config.mix_mode, MixMode::Mix);
    }

    #[test]
    fn test_pipeline_state() {
        assert_eq!(PipelineState::Stopped, PipelineState::Stopped);
        assert_ne!(PipelineState::Stopped, PipelineState::Running);
    }

    #[test]
    fn test_mix_mode() {
        assert_eq!(MixMode::Mix, MixMode::Mix);
        assert_ne!(MixMode::Mix, MixMode::LocalOnly);
    }

    #[test]
    fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.frames_captured, 0);
        assert_eq!(stats.latency_ms, 0.0);
    }
}
