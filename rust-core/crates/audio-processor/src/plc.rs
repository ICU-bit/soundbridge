//! 丢包隐藏（PLC - Packet Loss Concealment）模块
//!
//! 当网络音频包丢失时，基于波形外推（Waveform Extrapolation）生成平滑的替代音频。
//!
//! ## 算法概述
//!
//! 1. **历史缓冲**：保留最近 N 帧（默认 4 帧）的音频数据
//! 2. **基音检测**：使用自相关（Autocorrelation）分析历史波形的周期性
//! 3. **波形外推**：基于检测到的基音周期，外推生成替代音频
//! 4. **汉宁窗平滑**：在边界处应用汉宁窗，避免跳变伪影
//! 5. **渐进衰减**：连续丢包时，每帧应用衰减系数（默认 0.95）
//! 6. **舒适噪声**：长时间丢包（>5 帧）时，输出静音 + 舒适噪声
//!
//! ## 丢包模式
//!
//! | 模式 | 帧数 | 策略 |
//! |------|------|------|
//! | 单帧丢失 | 1 | 直接波形外推 |
//! | 连续丢失 | 2-5 | 渐进衰减外推 |
//! | 长时间丢失 | >5 | 静音 + 舒适噪声 |
//!
//! 参考: ITU-T G.711 Appendix I, OPUS PLC

use crate::{ProcessorError, Result};

/// PLC 配置
#[derive(Debug, Clone)]
pub struct PlcConfig {
    /// 采样率（Hz）
    pub sample_rate: u32,

    /// 帧大小（样本数），SoundBridge 固定 960 (20ms @ 48kHz)
    pub frame_size: usize,

    /// 历史缓冲区大小（帧数），保留最近 N 帧用于外推
    pub history_frames: usize,

    /// 衰减系数（每帧），连续丢包时的能量衰减
    pub decay_factor: f32,

    /// 汉宁窗淡入长度（样本数），用于边界平滑
    pub fade_in_samples: usize,

    /// 舒适噪声幅度（线性），0.0 禁用
    pub comfort_noise_level: f32,

    /// 进入静音模式的连续丢帧阈值
    pub silence_threshold_frames: u32,

    /// 基音检测最小周期（样本数）
    pub min_pitch_period: usize,

    /// 基音检测最大周期（样本数）
    pub max_pitch_period: usize,
}

impl Default for PlcConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            frame_size: 960,            // 20ms @ 48kHz
            history_frames: 4,          // 保留最近 4 帧
            decay_factor: 0.95,         // 每帧衰减 5%
            fade_in_samples: 120,       // 2.5ms 淡入 @ 48kHz
            comfort_noise_level: 0.005, // -46 dB 舒适噪声
            silence_threshold_frames: 6, // >5 帧进入静音
            min_pitch_period: 120,      // 2.5ms（~400Hz）
            max_pitch_period: 960,      // 20ms（~50Hz）
        }
    }
}

/// PLC 处理器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlcState {
    /// 正常播放（收到有效包）
    Normal,
    /// 隐藏中（丢包，正在生成替代音频）
    Concealing,
    /// 静默（超过阈值，输出静音 + 舒适噪声）
    Silent,
}

/// 丢包隐藏处理器
///
/// 使用波形外推算法在网络音频包丢失时生成平滑的替代音频。
/// 保持最近几帧的历史数据，通过自相关检测基音周期，
/// 然后基于周期性外推生成听起来自然的替代音频。
pub struct PlcProcessor {
    /// 配置
    config: PlcConfig,

    /// 当前状态
    state: PlcState,

    /// 历史帧缓冲区（环形缓冲区）
    history: Vec<Vec<f32>>,

    /// 历史缓冲区写入位置
    history_pos: usize,

    /// 连续丢帧计数
    consecutive_lost: u32,

    /// 当前衰减因子（随连续丢帧递减）
    current_decay: f32,

    /// 最后一次检测到的基音周期（样本数）
    pitch_period: usize,

    /// 汉宁窗淡入系数（预计算）
    fade_in_window: Vec<f32>,

    /// 伪随机数状态（用于舒适噪声生成）
    noise_seed: u32,
}

impl PlcProcessor {
    /// 创建新的 PLC 处理器
    pub fn new(config: PlcConfig) -> Result<Self> {
        if config.frame_size == 0 {
            return Err(ProcessorError::ConfigError(
                "frame_size must be > 0".to_string(),
            ));
        }
        if config.decay_factor < 0.0 || config.decay_factor > 1.0 {
            return Err(ProcessorError::ConfigError(
                "decay_factor must be 0.0..1.0".to_string(),
            ));
        }
        if config.fade_in_samples > config.frame_size {
            return Err(ProcessorError::ConfigError(
                "fade_in_samples must be <= frame_size".to_string(),
            ));
        }
        if config.min_pitch_period >= config.max_pitch_period {
            return Err(ProcessorError::ConfigError(
                "min_pitch_period must be < max_pitch_period".to_string(),
            ));
        }

        let history_frames = config.history_frames.max(2);
        let frame_size = config.frame_size;
        let fade_len = config.fade_in_samples;

        // 预计算汉宁窗淡入系数
        let fade_in_window: Vec<f32> = (0..fade_len)
            .map(|i| {
                let phase = std::f32::consts::PI * i as f32 / fade_len as f32;
                0.5 * (1.0 - phase.cos())
            })
            .collect();

        Ok(Self {
            config,
            state: PlcState::Normal,
            history: vec![vec![0.0; frame_size]; history_frames],
            history_pos: 0,
            consecutive_lost: 0,
            current_decay: 1.0,
            pitch_period: frame_size, // 默认一帧长度
            fade_in_window,
            noise_seed: 0x12345678,
        })
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Result<Self> {
        Self::new(PlcConfig::default())
    }

    /// 处理一帧正常音频（收到有效包时调用）
    ///
    /// 将有效帧存入历史缓冲区，重置隐藏状态。
    pub fn process_good_frame(&mut self, buffer: &[f32]) -> Result<()> {
        if buffer.len() != self.config.frame_size {
            return Err(ProcessorError::BufferError(format!(
                "Expected {} samples, got {}",
                self.config.frame_size,
                buffer.len()
            )));
        }

        // 存入历史缓冲区
        self.history[self.history_pos].copy_from_slice(buffer);
        self.history_pos = (self.history_pos + 1) % self.history.len();

        // 重置隐藏状态
        if self.state != PlcState::Normal {
            self.state = PlcState::Normal;
            self.consecutive_lost = 0;
            self.current_decay = 1.0;
        }

        Ok(())
    }

    /// 生成隐藏帧（丢包时调用）
    ///
    /// 基于历史帧数据，使用波形外推生成替代音频。
    ///
    /// # Returns
    /// 生成的隐藏音频帧
    pub fn conceal(&mut self) -> Result<Vec<f32>> {
        let frame_size = self.config.frame_size;
        let mut output = vec![0.0f32; frame_size];

        self.consecutive_lost += 1;

        // 超过静音阈值 → 静音 + 舒适噪声
        if self.consecutive_lost >= self.config.silence_threshold_frames {
            self.state = PlcState::Silent;
            self.add_comfort_noise(&mut output);
            return Ok(output);
        }

        self.state = PlcState::Concealing;

        // 更新衰减因子
        self.current_decay = self.config.decay_factor.powi(self.consecutive_lost as i32);

        // 检测基音周期（使用历史数据）
        self.pitch_period = self.detect_pitch();

        // 波形外推
        self.extrapolate_waveform(&mut output);

        // 应用汉宁窗淡入（平滑边界跳变）
        self.apply_fade_in(&mut output);

        // 应用衰减
        for sample in output.iter_mut() {
            *sample *= self.current_decay;
        }

        // 添加舒适噪声
        self.add_comfort_noise(&mut output);

        Ok(output)
    }

    /// 获取当前状态
    pub fn state(&self) -> PlcState {
        self.state
    }

    /// 获取连续丢帧数
    pub fn consecutive_lost_frames(&self) -> u32 {
        self.consecutive_lost
    }

    /// 获取当前衰减因子
    pub fn current_decay(&self) -> f32 {
        self.current_decay
    }

    /// 重置处理器状态
    pub fn reset(&mut self) {
        self.state = PlcState::Normal;
        for frame in &mut self.history {
            frame.fill(0.0);
        }
        self.history_pos = 0;
        self.consecutive_lost = 0;
        self.current_decay = 1.0;
        self.pitch_period = self.config.frame_size;
    }

    /// 获取配置
    pub fn config(&self) -> &PlcConfig {
        &self.config
    }

    /// 自相关基音检测
    ///
    /// 在历史缓冲区上计算自相关，找到最强的周期性峰值。
    /// 返回基音周期（样本数）。
    fn detect_pitch(&self) -> usize {
        let history_len = self.history.len();
        let frame_size = self.config.frame_size;
        let min_lag = self.config.min_pitch_period;
        let max_lag = self.config.max_pitch_period.min(frame_size);

        // 拼接最近 2 帧历史数据用于分析
        let prev_idx = (self.history_pos + history_len - 1) % history_len;
        let prev2_idx = (self.history_pos + history_len - 2) % history_len;
        let analysis_len = frame_size * 2;
        let mut combined = Vec::with_capacity(analysis_len);
        combined.extend_from_slice(&self.history[prev2_idx]);
        combined.extend_from_slice(&self.history[prev_idx]);

        // 计算自相关
        let mut best_lag = min_lag;
        let mut best_corr = 0.0f32;

        for lag in min_lag..=max_lag {
            let mut corr = 0.0f32;
            let mut energy1 = 0.0f32;
            let mut energy2 = 0.0f32;
            let n = analysis_len - lag;

            for i in 0..n {
                let a = combined[i];
                let b = combined[i + lag];
                corr += a * b;
                energy1 += a * a;
                energy2 += b * b;
            }

            // 归一化自相关
            let norm = (energy1 * energy2).sqrt();
            if norm > 1e-10 {
                corr /= norm;
            }

            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }

        // 如果自相关太弱（无明显周期性），使用默认一帧长度
        if best_corr < 0.3 {
            frame_size
        } else {
            best_lag
        }
    }

    /// 波形外推
    ///
    /// 基于检测到的基音周期，从历史数据中重复波形片段来填充输出帧。
    fn extrapolate_waveform(&self, output: &mut [f32]) {
        let history_len = self.history.len();
        let last_idx = (self.history_pos + history_len - 1) % history_len;
        let last_frame = &self.history[last_idx];
        let period = self.pitch_period;
        let frame_size = self.config.frame_size;

        // 从最后一帧的末尾开始，按基音周期重复
        for (i, sample) in output.iter_mut().enumerate() {
            // 在最后一帧中按周期回溯
            let source_pos = if period > 0 {
                let offset = (i + 1) % period;
                frame_size.saturating_sub(period) + offset
            } else {
                frame_size.saturating_sub(1)
            };
            let idx = source_pos.min(frame_size - 1);
            *sample = last_frame[idx];
        }
    }

    /// 应用汉宁窗淡入
    ///
    /// 在输出帧的开头应用汉宁窗，平滑从历史数据到外推数据的过渡。
    fn apply_fade_in(&self, output: &mut [f32]) {
        let fade_len = self.fade_in_window.len().min(output.len());
        let history_len = self.history.len();
        let last_idx = (self.history_pos + history_len - 1) % history_len;
        let last_frame = &self.history[last_idx];
        let frame_size = self.config.frame_size;

        // 淡入区域：混合最后一帧末尾和外推输出
        for i in 0..fade_len {
            let window = self.fade_in_window[i];
            let history_sample = last_frame[frame_size - fade_len + i];
            // 交叉淡入：window=0 时用历史数据，window=1 时用外推数据
            output[i] = history_sample * (1.0 - window) + output[i] * window;
        }
    }

    /// 添加舒适噪声
    ///
    /// 在输出帧上叠加微量随机噪声，避免死寂感。
    fn add_comfort_noise(&mut self, output: &mut [f32]) {
        if self.config.comfort_noise_level > 0.0 {
            for sample in output.iter_mut() {
                let noise = self.next_noise() * self.config.comfort_noise_level;
                *sample += noise;
            }
        }
    }

    /// 生成伪随机噪声样本（-1.0 到 1.0）
    ///
    /// 使用简单的 LCG（线性同余生成器），足够用于舒适噪声。
    fn next_noise(&mut self) -> f32 {
        self.noise_seed = self
            .noise_seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        (self.noise_seed as f32) / (u32::MAX as f32) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 基础创建与配置 ─────────────────────────────────────────

    #[test]
    fn test_plc_creation() {
        let plc = PlcProcessor::with_default_config().unwrap();
        assert_eq!(plc.state(), PlcState::Normal);
        assert_eq!(plc.consecutive_lost_frames(), 0);
    }

    #[test]
    fn test_plc_config_default() {
        let config = PlcConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.frame_size, 960);
        assert_eq!(config.history_frames, 4);
        assert!((config.decay_factor - 0.95).abs() < 0.001);
        assert_eq!(config.fade_in_samples, 120);
        assert_eq!(config.silence_threshold_frames, 6);
    }

    #[test]
    fn test_plc_custom_config() {
        let config = PlcConfig {
            history_frames: 6,
            decay_factor: 0.9,
            fade_in_samples: 240,
            ..Default::default()
        };
        let plc = PlcProcessor::new(config).unwrap();
        assert_eq!(plc.config().history_frames, 6);
        assert!((plc.config().decay_factor - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_plc_invalid_config_zero_frame() {
        let config = PlcConfig {
            frame_size: 0,
            ..Default::default()
        };
        assert!(PlcProcessor::new(config).is_err());
    }

    #[test]
    fn test_plc_invalid_config_bad_decay() {
        let config = PlcConfig {
            decay_factor: 1.5,
            ..Default::default()
        };
        assert!(PlcProcessor::new(config).is_err());
    }

    #[test]
    fn test_plc_invalid_config_fade_too_long() {
        let config = PlcConfig {
            fade_in_samples: 1000, // > frame_size (960)
            ..Default::default()
        };
        assert!(PlcProcessor::new(config).is_err());
    }

    #[test]
    fn test_plc_invalid_config_pitch_range() {
        let config = PlcConfig {
            min_pitch_period: 500,
            max_pitch_period: 300,
            ..Default::default()
        };
        assert!(PlcProcessor::new(config).is_err());
    }

    // ── 有效帧处理 ─────────────────────────────────────────────

    #[test]
    fn test_plc_good_frame() {
        let mut plc = PlcProcessor::with_default_config().unwrap();
        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();
        assert_eq!(plc.state(), PlcState::Normal);
        assert_eq!(plc.consecutive_lost_frames(), 0);
    }

    #[test]
    fn test_plc_good_frame_wrong_size() {
        let mut plc = PlcProcessor::with_default_config().unwrap();
        let wrong_size = vec![0.5f32; 100];
        assert!(plc.process_good_frame(&wrong_size).is_err());
    }

    // ── 单帧丢失恢复质量 ──────────────────────────────────────

    #[test]
    fn test_plc_single_frame_concealment() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 喂入多个有效帧建立历史
        for i in 0..4 {
            let frame: Vec<f32> = (0..960)
                .map(|j| {
                    let t = (i * 960 + j) as f32 / 48000.0;
                    (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                })
                .collect();
            plc.process_good_frame(&frame).unwrap();
        }

        // 模拟单帧丢失
        let concealed = plc.conceal().unwrap();
        assert_eq!(concealed.len(), 960);
        assert_eq!(plc.state(), PlcState::Concealing);
        assert_eq!(plc.consecutive_lost_frames(), 1);

        // 隐藏帧应该有内容（非静音）
        let rms: f32 =
            (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();
        assert!(rms > 0.1, "Single frame concealment should have content, RMS: {}", rms);

        // 隐藏帧不应该有爆音（样本值不应超过输入幅度太多）
        let max_sample = concealed.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_sample < 1.0,
            "No clipping expected, max: {}",
            max_sample
        );
    }

    // ── 连续多帧丢失衰减 ─────────────────────────────────────

    #[test]
    fn test_plc_multi_frame_decay() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 喂入有效帧
        for i in 0..4 {
            let frame: Vec<f32> = (0..960)
                .map(|j| {
                    let t = (i * 960 + j) as f32 / 48000.0;
                    (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                })
                .collect();
            plc.process_good_frame(&frame).unwrap();
        }

        // 连续丢 2-5 帧，检查能量递减
        let mut prev_rms = f32::MAX;
        for _ in 0..5 {
            let concealed = plc.conceal().unwrap();
            let rms: f32 =
                (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();

            if plc.state() == PlcState::Concealing {
                assert!(
                    rms <= prev_rms + 0.01,
                    "Energy should decay: prev={}, current={}",
                    prev_rms,
                    rms
                );
            }
            prev_rms = rms;
        }
    }

    #[test]
    fn test_plc_decay_factor_applied() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        // 第一帧丢失
        plc.conceal().unwrap();
        let decay1 = plc.current_decay();
        assert!((decay1 - 0.95).abs() < 0.001, "First loss decay should be 0.95");

        // 第二帧丢失
        plc.conceal().unwrap();
        let decay2 = plc.current_decay();
        assert!(
            (decay2 - 0.95 * 0.95).abs() < 0.001,
            "Second loss decay should be 0.95^2"
        );
    }

    // ── 长时间丢包处理（>5 帧 → 静音 + 舒适噪声）───────────

    #[test]
    fn test_plc_long_loss_enters_silent() {
        let config = PlcConfig {
            silence_threshold_frames: 3, // 3 帧后进入静音
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        // 丢 2 帧：应该还在 Concealing 状态
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Concealing);
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Concealing);

        // 丢第 3 帧：应该进入 Silent 状态
        let concealed = plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Silent);

        // 静音帧应该接近零（只有舒适噪声）
        let rms: f32 =
            (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();
        assert!(
            rms < 0.05,
            "Silent frame should be near zero, RMS: {}",
            rms
        );
    }

    #[test]
    fn test_plc_silent_with_comfort_noise() {
        let config = PlcConfig {
            silence_threshold_frames: 2,
            comfort_noise_level: 0.01,
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();

        let silence = vec![0.0f32; 960];
        plc.process_good_frame(&silence).unwrap();

        // 丢 2 帧进入静音
        plc.conceal().unwrap();
        let concealed = plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Silent);

        // 有舒适噪声时，输出不应完全为零
        let rms: f32 =
            (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();
        assert!(rms > 0.0, "Comfort noise should produce some signal, RMS: {}", rms);
    }

    // ── 恢复测试 ──────────────────────────────────────────────

    #[test]
    fn test_plc_recovery_after_loss() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        // 模拟丢包
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Concealing);
        assert_eq!(plc.consecutive_lost_frames(), 1);

        // 恢复：收到有效帧
        plc.process_good_frame(&frame).unwrap();
        assert_eq!(plc.state(), PlcState::Normal);
        assert_eq!(plc.consecutive_lost_frames(), 0);
    }

    #[test]
    fn test_plc_recovery_after_silent() {
        let config = PlcConfig {
            silence_threshold_frames: 2,
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        // 进入静音状态
        plc.conceal().unwrap();
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Silent);

        // 恢复
        plc.process_good_frame(&frame).unwrap();
        assert_eq!(plc.state(), PlcState::Normal);
        assert_eq!(plc.consecutive_lost_frames(), 0);
    }

    // ── 汉宁窗平滑 ────────────────────────────────────────────

    #[test]
    fn test_plc_hanning_window_no_discontinuity() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 喂入正弦波帧
        for i in 0..4 {
            let frame: Vec<f32> = (0..960)
                .map(|j| {
                    let t = (i * 960 + j) as f32 / 48000.0;
                    (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                })
                .collect();
            plc.process_good_frame(&frame).unwrap();
        }

        let concealed = plc.conceal().unwrap();

        // 检查开头没有大的跳变（汉宁窗应该平滑过渡）
        // 计算最大相邻样本差
        let mut max_delta = 0.0f32;
        for i in 1..concealed.len().min(240) {
            let delta = (concealed[i] - concealed[i - 1]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
        }
        // 对于 440Hz 正弦波，相邻样本最大差应该远小于 1.0
        assert!(
            max_delta < 0.5,
            "Hanning window should smooth transitions, max_delta: {}",
            max_delta
        );
    }

    // ── 舒适噪声 ──────────────────────────────────────────────

    #[test]
    fn test_plc_comfort_noise_enabled() {
        let config = PlcConfig {
            comfort_noise_level: 0.01,
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();

        let silence = vec![0.0f32; 960];
        plc.process_good_frame(&silence).unwrap();

        let concealed = plc.conceal().unwrap();

        // 即使历史是静音，舒适噪声也应该产生一些信号
        let rms: f32 =
            (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();
        assert!(rms > 0.0, "Comfort noise should produce some signal, RMS: {}", rms);
    }

    #[test]
    fn test_plc_no_comfort_noise() {
        let config = PlcConfig {
            comfort_noise_level: 0.0,
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();

        let silence = vec![0.0f32; 960];
        plc.process_good_frame(&silence).unwrap();

        let concealed = plc.conceal().unwrap();

        // 没有舒适噪声 + 静音输入 → 输出应该全为零
        for sample in &concealed {
            assert!(
                sample.abs() < 0.001,
                "Without comfort noise, silent input should produce silent output"
            );
        }
    }

    // ── 重置 ──────────────────────────────────────────────────

    #[test]
    fn test_plc_reset() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();
        plc.conceal().unwrap();

        plc.reset();
        assert_eq!(plc.state(), PlcState::Normal);
        assert_eq!(plc.consecutive_lost_frames(), 0);
        assert!((plc.current_decay() - 1.0).abs() < 0.001);

        // 历史缓冲区应该清零
        for frame in &plc.history {
            for sample in frame {
                assert_eq!(*sample, 0.0);
            }
        }
    }

    // ── 多帧历史 ──────────────────────────────────────────────

    #[test]
    fn test_plc_multiple_history_frames() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 喂入 4 个不同的正弦波帧
        for i in 0..4 {
            let freq = 220.0 + i as f32 * 110.0; // 220, 330, 440, 550 Hz
            let frame: Vec<f32> = (0..960)
                .map(|j| {
                    let t = (i * 960 + j) as f32 / 48000.0;
                    (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5
                })
                .collect();
            plc.process_good_frame(&frame).unwrap();
        }

        // 丢包隐藏
        let concealed = plc.conceal().unwrap();
        let rms: f32 =
            (concealed.iter().map(|&s| s * s).sum::<f32>() / concealed.len() as f32).sqrt();
        assert!(rms > 0.0, "Should produce audible output from history");
    }

    // ── 状态转换 ──────────────────────────────────────────────

    #[test]
    fn test_plc_state_transitions() {
        let config = PlcConfig {
            silence_threshold_frames: 2,
            ..Default::default()
        };
        let mut plc = PlcProcessor::new(config).unwrap();
        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        // Normal → Concealing
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Concealing);

        // Concealing → Silent
        plc.conceal().unwrap();
        assert_eq!(plc.state(), PlcState::Silent);

        // Silent → Normal (恢复)
        plc.process_good_frame(&frame).unwrap();
        assert_eq!(plc.state(), PlcState::Normal);
    }

    // ── 噪声确定性 ────────────────────────────────────────────

    #[test]
    fn test_plc_noise_determinism() {
        let config = PlcConfig {
            comfort_noise_level: 0.01,
            ..Default::default()
        };

        let mut plc1 = PlcProcessor::new(config.clone()).unwrap();
        let mut plc2 = PlcProcessor::new(config).unwrap();

        let silence = vec![0.0f32; 960];
        plc1.process_good_frame(&silence).unwrap();
        plc2.process_good_frame(&silence).unwrap();

        let out1 = plc1.conceal().unwrap();
        let out2 = plc2.conceal().unwrap();

        for (a, b) in out1.iter().zip(out2.iter()) {
            assert!(
                (a - b).abs() < 0.0001,
                "Same config should produce same noise"
            );
        }
    }

    // ── 连续丢帧计数 ──────────────────────────────────────────

    #[test]
    fn test_plc_consecutive_counter() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        let frame = vec![0.5f32; 960];
        plc.process_good_frame(&frame).unwrap();

        for i in 1..=5 {
            plc.conceal().unwrap();
            assert_eq!(plc.consecutive_lost_frames(), i);
        }
    }

    // ── 与音频处理管线兼容性 ──────────────────────────────────

    #[test]
    fn test_plc_with_processor_pipeline() {
        use crate::AudioProcessor;

        let mut processor = AudioProcessor::with_default_config().unwrap();
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 模拟接收端：收到正常帧 → 处理 → 存入历史
        let frame = vec![0.3f32; 960];
        let mut processed = frame.clone();
        processor.process(&mut processed).unwrap();
        plc.process_good_frame(&processed).unwrap();

        // 丢包 → 生成隐藏帧 → 继续处理
        let concealed = plc.conceal().unwrap();
        let mut output = concealed;
        processor.process(&mut output).unwrap();

        // 输出应该是有效的
        for sample in &output {
            assert!(sample.is_finite(), "Output should be finite");
        }
        let rms: f32 = (output.iter().map(|&s| s * s).sum::<f32>() / output.len() as f32).sqrt();
        assert!(rms > 0.0, "Pipeline output should have content");
    }

    // ── 基音检测 ──────────────────────────────────────────────

    #[test]
    fn test_plc_pitch_detection_periodic_signal() {
        let mut plc = PlcProcessor::with_default_config().unwrap();

        // 喂入具有明确周期性的信号（440Hz → 周期 ≈ 109 样本 @ 48kHz）
        for i in 0..4 {
            let frame: Vec<f32> = (0..960)
                .map(|j| {
                    let t = (i * 960 + j) as f32 / 48000.0;
                    (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                })
                .collect();
            plc.process_good_frame(&frame).unwrap();
        }

        // 检测基音周期
        let period = plc.detect_pitch();
        // 440Hz @ 48kHz 的周期约 109 样本，自相关可能找到谐波（2×=218）
        // 对于 PLC 外推，2× 周期同样有效
        assert!(
            period >= 80 && period <= 300,
            "Pitch period for 440Hz should be ~109 or harmonic ~218, got: {}",
            period
        );
    }

    // ── 空帧缓冲区安全 ────────────────────────────────────────

    #[test]
    fn test_plc_conceal_without_good_frame() {
        let mut plc = PlcProcessor::with_default_config().unwrap();
        // 没有喂入任何有效帧就调用 conceal
        let concealed = plc.conceal().unwrap();
        assert_eq!(concealed.len(), 960);
        // 应该不会 panic，输出是静音或零
    }
}
