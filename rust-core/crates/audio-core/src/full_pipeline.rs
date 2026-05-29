//! SoundBridge 完整音频管线接口
//!
//! 定义 capture → encode → network → decode → playback 完整链路的接口。
//! 实际实现依赖 audio-capture、audio-playback、audio-codec、audio-processor、audio-mixer、network 等 crate。

/// 管线配置
#[derive(Debug, Clone)]
pub struct FullPipelineConfig {
    /// 采样率
    pub sample_rate: u32,

    /// 通道数
    pub channels: u16,

    /// 帧大小（样本数）
    pub frame_size: usize,

    /// Opus 比特率
    pub opus_bitrate: u32,

    /// 远端地址
    pub remote_addr: Option<std::net::SocketAddr>,

    /// 本地监听端口
    pub local_port: u16,
}

impl Default for FullPipelineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            frame_size: 960,
            opus_bitrate: 128000,
            remote_addr: None,
            local_port: 0,
        }
    }
}

/// 管线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullPipelineState {
    /// 停止
    Stopped,

    /// 运行中
    Running,

    /// 错误
    Error,
}

/// 管线统计
#[derive(Debug, Clone, Default)]
pub struct FullPipelineStats {
    /// 采集帧数
    pub frames_captured: u64,

    /// 编码帧数
    pub frames_encoded: u64,

    /// 发送包数
    pub packets_sent: u64,

    /// 接收包数
    pub packets_received: u64,

    /// 解码帧数
    pub frames_decoded: u64,

    /// 播放帧数
    pub frames_played: u64,
}

/// 完整音频管线 trait
///
/// 定义端到端音频管线的核心接口。
pub trait FullPipeline {
    /// 启动管线
    fn start(&mut self) -> std::result::Result<(), String>;

    /// 停止管线
    fn stop(&mut self);

    /// 获取状态
    fn state(&self) -> FullPipelineState;

    /// 获取统计
    fn stats(&self) -> FullPipelineStats;

    /// 获取配置
    fn config(&self) -> &FullPipelineConfig;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = FullPipelineConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.frame_size, 960);
        assert_eq!(config.opus_bitrate, 128000);
        assert!(config.remote_addr.is_none());
        assert_eq!(config.local_port, 0);
    }

    #[test]
    fn test_pipeline_state() {
        assert_eq!(FullPipelineState::Stopped, FullPipelineState::Stopped);
        assert_ne!(FullPipelineState::Stopped, FullPipelineState::Running);
        assert_ne!(FullPipelineState::Running, FullPipelineState::Error);
    }

    #[test]
    fn test_pipeline_stats_default() {
        let stats = FullPipelineStats::default();
        assert_eq!(stats.frames_captured, 0);
        assert_eq!(stats.frames_encoded, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.frames_decoded, 0);
        assert_eq!(stats.frames_played, 0);
    }

    #[test]
    fn test_pipeline_config_custom() {
        let config = FullPipelineConfig {
            sample_rate: 44100,
            channels: 1,
            frame_size: 480,
            opus_bitrate: 64000,
            remote_addr: None,
            local_port: 12345,
        };
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.channels, 1);
        assert_eq!(config.frame_size, 480);
        assert_eq!(config.opus_bitrate, 64000);
        assert_eq!(config.local_port, 12345);
    }
}
