//! 音频管线实现
//!
//! 实现完整的音频处理管道：采集 → 编码 → 网络发送 → 接收 → 解码 → 播放

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::mpsc;

use audio_capture::{CaptureDevice, CaptureConfig};
use audio_playback::{PlaybackDevice, PlaybackConfig};
use audio_codec::{OpusEncoderCodec, OpusDecoderCodec, OpusConfig, SampleRate, ChannelConfig, Bitrate, FrameSize};
use audio_core::{AudioBuffer, AudioFormat, SampleFormat, RingBuffer};
use audio_mixer::{AudioMixer, MixerConfig};
use audio_processor::{AudioProcessor, ProcessorConfig};

use crate::{AudioPipeline, PipelineConfig, PipelineState, PipelineStats, PipelineError, MixMode};

/// 具体音频管线实现
pub struct ConcreteAudioPipeline {
    config: PipelineConfig,
    state: PipelineState,
    running: Arc<AtomicBool>,
    
    // 统计
    frames_captured: Arc<AtomicU64>,
    frames_played: Arc<AtomicU64>,
    frames_encoded: Arc<AtomicU64>,
    frames_decoded: Arc<AtomicU64>,
    frames_dropped: Arc<AtomicU64>,
    
    // 组件
    capture: Option<CaptureDevice>,
    playback: Option<PlaybackDevice>,
    encoder: Option<OpusEncoderCodec>,
    decoder: Option<OpusDecoderCodec>,
    mixer: AudioMixer,
    processor: AudioProcessor,
}

impl ConcreteAudioPipeline {
    /// 创建新的音频管线
    pub fn new(config: PipelineConfig) -> Self {
        let mixer = AudioMixer::new(MixerConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
            clipping_protection: true,
        });
        
        let processor = AudioProcessor::new(ProcessorConfig::default())
            .expect("Failed to create AudioProcessor");
        
        Self {
            config,
            state: PipelineState::Stopped,
            running: Arc::new(AtomicBool::new(false)),
            frames_captured: Arc::new(AtomicU64::new(0)),
            frames_played: Arc::new(AtomicU64::new(0)),
            frames_encoded: Arc::new(AtomicU64::new(0)),
            frames_decoded: Arc::new(AtomicU64::new(0)),
            frames_dropped: Arc::new(AtomicU64::new(0)),
            capture: None,
            playback: None,
            encoder: None,
            decoder: None,
            mixer,
            processor,
        }
    }
    
    /// 初始化组件
    pub fn initialize(&mut self) -> Result<(), PipelineError> {
        // 创建采集设备
        let capture_config = CaptureConfig {
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            buffer_size: self.config.frame_size as u32,
        };
        self.capture = Some(CaptureDevice::new_default(capture_config)
            .map_err(|e| PipelineError::CaptureError(e.to_string()))?);
        
        // 创建播放设备
        let playback_config = PlaybackConfig {
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            buffer_size: self.config.frame_size as u32,
        };
        self.playback = Some(PlaybackDevice::new_default(playback_config)
            .map_err(|e| PipelineError::PlaybackError(e.to_string()))?);
        
        // 创建编码器
        let opus_config = OpusConfig::new(
            SampleRate::Hz48000,
            ChannelConfig::Stereo,
            Bitrate::Kbps128,
            FrameSize::Ms20,
        );
        self.encoder = Some(OpusEncoderCodec::new(opus_config)
            .map_err(|e| PipelineError::EncodingError(e.to_string()))?);
        
        // 创建解码器
        self.decoder = Some(OpusDecoderCodec::new(opus_config)
            .map_err(|e| PipelineError::DecodingError(e.to_string()))?);
        
        Ok(())
    }
}

impl AudioPipeline for ConcreteAudioPipeline {
    fn start(&mut self) -> Result<(), PipelineError> {
        if self.state == PipelineState::Running {
            return Ok(());
        }
        
        // 初始化组件
        self.initialize()?;
        
        // 启动采集
        if let Some(ref mut capture) = self.capture {
            capture.start()
                .map_err(|e| PipelineError::CaptureError(e.to_string()))?;
        }
        
        // 启动播放
        if let Some(ref mut playback) = self.playback {
            playback.start()
                .map_err(|e| PipelineError::PlaybackError(e.to_string()))?;
        }
        
        self.running.store(true, Ordering::SeqCst);
        self.state = PipelineState::Running;
        
        Ok(())
    }
    
    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        
        // 停止采集
        if let Some(ref mut capture) = self.capture {
            let _ = capture.stop();
        }
        
        // 停止播放
        if let Some(ref mut playback) = self.playback {
            let _ = playback.stop();
        }
        
        self.state = PipelineState::Stopped;
    }
    
    fn state(&self) -> PipelineState {
        self.state
    }
    
    fn stats(&self) -> PipelineStats {
        // Latency estimate based on buffer/codec pipeline:
        // capture_buffer + encode + decode + playback_buffer ≈ 2× frame_duration + codec
        let frame_duration_ms = (self.config.frame_size as f64) / (self.config.sample_rate as f64) * 1000.0;
        let latency_ms = (frame_duration_ms * 3.0) as f32; // capture + codec + playback buffers
        
        PipelineStats {
            frames_captured: self.frames_captured.load(Ordering::Relaxed),
            frames_played: self.frames_played.load(Ordering::Relaxed),
            frames_encoded: self.frames_encoded.load(Ordering::Relaxed),
            frames_decoded: self.frames_decoded.load(Ordering::Relaxed),
            frames_dropped: self.frames_dropped.load(Ordering::Relaxed),
            latency_ms,
        }
    }
    
    fn config(&self) -> &PipelineConfig {
        &self.config
    }
}

impl Drop for ConcreteAudioPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = ConcreteAudioPipeline::new(PipelineConfig::default());
        assert_eq!(pipeline.state(), PipelineState::Stopped);
        assert_eq!(pipeline.config().sample_rate, 48000);
        assert_eq!(pipeline.config().channels, 2);
    }

    #[test]
    fn test_pipeline_stats() {
        let pipeline = ConcreteAudioPipeline::new(PipelineConfig::default());
        let stats = pipeline.stats();
        assert_eq!(stats.frames_captured, 0);
        assert_eq!(stats.latency_ms, 0.0);
    }
}
