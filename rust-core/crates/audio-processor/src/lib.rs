//! SoundBridge 音频处理模块
//!
//! 提供音频处理功能：增益控制、静音检测、噪声门、回声消除、噪声抑制、自动增益控制。

pub mod aec;
pub mod agc;
pub mod eq;
pub mod gain;
pub mod noise_gate;
pub mod ns;
pub mod plc;
pub mod silence_detector;

pub use aec::AecProcessor;
pub use agc::AgcProcessor;
pub use eq::{BiquadFilter, EqPreset, ParametricEq};
pub use gain::GainProcessor;
pub use noise_gate::NoiseGate;
pub use ns::NsProcessor;
pub use plc::{PlcConfig, PlcProcessor, PlcState};
pub use silence_detector::SilenceDetector;

/// 处理配置
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// 增益（dB）
    pub gain_db: f32,

    /// 静音检测阈值（dB）
    pub silence_threshold_db: f32,

    /// 噪声门阈值（dB）
    pub noise_gate_threshold_db: f32,

    /// AEC 回声消除尾长（毫秒）
    pub aec_tail_ms: u32,

    /// NS 噪声抑制强度（dB）
    pub ns_suppression_db: f32,

    /// AGC 目标电平（dBFS）
    pub agc_target_dbfs: f32,

    /// AGC 最大增益（dB）
    pub agc_max_gain_db: f32,

    /// PLC 衰减系数（每帧）
    pub plc_decay_factor: f32,

    /// PLC 历史缓冲帧数
    pub plc_history_frames: usize,

    /// PLC 静音阈值（连续丢帧数）
    pub plc_silence_threshold: u32,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            silence_threshold_db: -60.0,
            noise_gate_threshold_db: -50.0,
            aec_tail_ms: 50, // 技术规格 §3.1: 尾部长度 50ms
            ns_suppression_db: 12.0,
            agc_target_dbfs: -3.0, // 技术规格 §3.3: 目标电平 -3 dBFS
            agc_max_gain_db: 30.0,
            plc_decay_factor: 0.95,
            plc_history_frames: 4,
            plc_silence_threshold: 6,
        }
    }
}

/// 处理错误类型
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("处理失败: {0}")]
    ProcessingFailed(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("缓冲区错误: {0}")]
    BufferError(String),
}

/// 处理结果类型
pub type Result<T> = std::result::Result<T, ProcessorError>;

/// 音频处理器
pub struct AudioProcessor {
    config: ProcessorConfig,
    gain: GainProcessor,
    silence_detector: SilenceDetector,
    noise_gate: NoiseGate,
    aec: AecProcessor,
    ns: NsProcessor,
    agc: AgcProcessor,
    plc: PlcProcessor,
}

impl AudioProcessor {
    /// 创建新的处理器
    pub fn new(config: ProcessorConfig) -> Result<Self> {
        let gain = GainProcessor::new(config.gain_db)?;
        let silence_detector = SilenceDetector::new(config.silence_threshold_db)?;
        let noise_gate = NoiseGate::new(config.noise_gate_threshold_db)?;
        let aec = AecProcessor::new(aec::AecConfig {
            filter_length: (config.aec_tail_ms as usize * 48000) / 1000,
            step_size: 0.5,
            regularization: 1e-6,
        });
        let ns = NsProcessor::new(ns::NsConfig {
            suppression_db: config.ns_suppression_db,
            ..Default::default()
        });
        let agc = AgcProcessor::new(agc::AgcConfig {
            target_dbfs: config.agc_target_dbfs,
            max_gain_db: config.agc_max_gain_db,
            ..Default::default()
        });
        let plc = PlcProcessor::new(PlcConfig {
            decay_factor: config.plc_decay_factor,
            history_frames: config.plc_history_frames,
            silence_threshold_frames: config.plc_silence_threshold,
            ..Default::default()
        })?;

        Ok(Self {
            config,
            gain,
            silence_detector,
            noise_gate,
            aec,
            ns,
            agc,
            plc,
        })
    }

    /// 使用默认配置创建处理器
    pub fn with_default_config() -> Result<Self> {
        Self::new(ProcessorConfig::default())
    }

    /// 处理音频数据（就地修改）
    ///
    /// 处理流程：增益 → 噪声门 → NS → AGC
    pub fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        // 1. 应用增益
        self.gain.process(buffer)?;

        // 2. 噪声门
        self.noise_gate.process(buffer)?;

        // 3. 噪声抑制
        self.ns.process(buffer)?;

        // 4. 自动增益控制
        self.agc.process(buffer)?;

        Ok(())
    }

    /// 处理音频数据（带回声消除）
    ///
    /// 处理流程：AEC → 增益 → 噪声门 → NS → AGC
    pub fn process_with_aec(&mut self, buffer: &mut [f32], reference: &[f32]) -> Result<()> {
        // 1. 回声消除
        self.aec.process(buffer, reference)?;

        // 2. 应用增益
        self.gain.process(buffer)?;

        // 3. 噪声门
        self.noise_gate.process(buffer)?;

        // 4. 噪声抑制
        self.ns.process(buffer)?;

        // 5. 自动增益控制
        self.agc.process(buffer)?;

        Ok(())
    }

    /// 检测是否为静音
    pub fn is_silence(&self, buffer: &[f32]) -> bool {
        self.silence_detector.is_silence(buffer)
    }

    /// 计算 RMS（均方根）
    pub fn calculate_rms(&self, buffer: &[f32]) -> f32 {
        self.silence_detector.calculate_rms(buffer)
    }

    /// 获取配置
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// 喂入有效音频帧到 PLC 历史缓冲区
    ///
    /// 收到正常音频包时调用，用于更新 PLC 的历史数据。
    pub fn plc_feed_good_frame(&mut self, buffer: &[f32]) -> Result<()> {
        self.plc.process_good_frame(buffer)
    }

    /// 生成丢包隐藏帧
    ///
    /// 丢包时调用，返回基于历史数据外推的替代音频帧。
    pub fn plc_conceal(&mut self) -> Result<Vec<f32>> {
        self.plc.conceal()
    }

    /// 获取 PLC 当前状态
    pub fn plc_state(&self) -> PlcState {
        self.plc.state()
    }

    /// 获取 PLC 处理器的可变引用
    pub fn plc_processor(&mut self) -> &mut PlcProcessor {
        &mut self.plc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = AudioProcessor::with_default_config().unwrap();
        let _config = processor.config();
    }

    #[test]
    fn test_process_basic() {
        let mut processor = AudioProcessor::with_default_config().unwrap();
        let mut buffer = vec![0.5f32; 100];
        processor.process(&mut buffer).unwrap();
        // 默认配置下，输出应该接近输入
        for sample in &buffer {
            assert!(sample.abs() > 0.0, "Output should not be silent");
        }
    }

    #[test]
    fn test_process_with_aec() {
        let mut processor = AudioProcessor::with_default_config().unwrap();
        let mut buffer = vec![0.5f32; 100];
        let reference = vec![0.3f32; 100];
        processor.process_with_aec(&mut buffer, &reference).unwrap();
        // AEC 处理后，输出应该存在
        for sample in &buffer {
            assert!(sample.is_finite(), "Output should be finite");
        }
    }

    #[test]
    fn test_silence_detection() {
        let processor = AudioProcessor::with_default_config().unwrap();
        let silence = vec![0.0f32; 100];
        assert!(processor.is_silence(&silence));

        let signal = vec![0.5f32; 100];
        assert!(!processor.is_silence(&signal));
    }

    #[test]
    fn test_rms_calculation() {
        let processor = AudioProcessor::with_default_config().unwrap();

        let silence = vec![0.0f32; 100];
        assert_eq!(processor.calculate_rms(&silence), 0.0);

        let constant = vec![0.5f32; 100];
        let rms = processor.calculate_rms(&constant);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_buffer() {
        let processor = AudioProcessor::with_default_config().unwrap();
        let empty: Vec<f32> = vec![];
        assert!(processor.is_silence(&empty));
        assert_eq!(processor.calculate_rms(&empty), 0.0);
    }
}
