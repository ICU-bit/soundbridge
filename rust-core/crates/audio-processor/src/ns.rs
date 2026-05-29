//! 噪声抑制（NS）模块
//!
//! 实现基于 SNR 估计的噪声抑制。
//! 使用帧能量对比和信噪比计算自适应增益，抑制低信噪比信号。

use crate::Result;

/// 噪声抑制配置
#[derive(Debug, Clone)]
pub struct NsConfig {
    /// 抑制强度（dB），典型值 12-20 dB
    pub suppression_db: f32,

    /// 噪声估计窗口大小（样本数）
    pub window_size: usize,

    /// 噪声估计更新因子（0.0 - 1.0）
    pub noise_update_factor: f32,

    /// 最小增益（防止过度抑制）
    pub min_gain: f32,
}

impl Default for NsConfig {
    fn default() -> Self {
        Self {
            suppression_db: 12.0,
            window_size: 480,  // 10ms @ 48kHz
            noise_update_factor: 0.98,
            min_gain: 0.01,  // -40 dB
        }
    }
}

/// 噪声状态
struct NoiseState {
    /// 噪声功率谱估计
    noise_spectrum: Vec<f32>,

    /// 是否已初始化
    initialized: bool,
}

/// 噪声抑制处理器
///
/// 使用简化的频域噪声抑制算法：
/// 1. 估计噪声功率谱（在静音段更新）
/// 2. 对每个频率 bin 计算增益
/// 3. 应用增益抑制噪声
pub struct NsProcessor {
    /// 配置
    config: NsConfig,

    /// 噪声状态
    noise_state: NoiseState,

    /// 抑制因子
    suppression_factor: f32,
}

impl NsProcessor {
    /// 创建新的噪声抑制处理器
    pub fn new(config: NsConfig) -> Self {
        let suppression_factor = 10.0f32.powf(-config.suppression_db.abs() / 20.0);
        Self {
            config,
            noise_state: NoiseState {
                noise_spectrum: vec![0.0; 256],  // 简化：固定频谱大小
                initialized: false,
            },
            suppression_factor,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(NsConfig::default())
    }

    /// 处理音频数据（就地修改）
    ///
    /// 使用简化的噪声抑制算法：
    /// 1. 计算信号能量
    /// 2. 如果信号很弱，认为是噪声，更新噪声估计
    /// 3. 对弱信号应用噪声抑制
    pub fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        // 计算当前帧能量
        let frame_energy: f32 = buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32;

        // 初始化噪声估计（第一帧）
        if !self.noise_state.initialized {
            self.noise_state.noise_spectrum.fill(frame_energy);
            self.noise_state.initialized = true;
        }

        // 判断是否为噪声帧（能量低于噪声估计的 2 倍）
        let avg_noise: f32 = self.noise_state.noise_spectrum.iter().sum::<f32>()
            / self.noise_state.noise_spectrum.len() as f32;
        let is_noise_frame = frame_energy < avg_noise * 2.0;

        // 更新噪声估计（在噪声帧时更新）
        if is_noise_frame {
            for noise in &mut self.noise_state.noise_spectrum {
                *noise = *noise * self.config.noise_update_factor
                    + frame_energy * (1.0 - self.config.noise_update_factor);
            }
        }

        // 计算增益并应用
        if frame_energy > 0.0 {
            // 信噪比估计
            let snr = frame_energy / (avg_noise + 1e-10);
            let snr_db = 10.0 * snr.log10();

            // 根据 SNR 计算增益
            let gain = if snr_db > 20.0 {
                // 高 SNR，保留信号
                1.0
            } else if snr_db < -10.0 {
                // 低 SNR，强抑制
                self.suppression_factor
            } else {
                // 线性插值
                let t = (snr_db + 10.0) / 30.0;
                self.suppression_factor + (1.0 - self.suppression_factor) * t
            };

            // 应用增益，限制最小增益
            let gain = gain.max(self.config.min_gain);
            for sample in buffer.iter_mut() {
                *sample *= gain;
            }
        }

        Ok(())
    }

    /// 重置处理器状态
    pub fn reset(&mut self) {
        self.noise_state.noise_spectrum.fill(0.0);
        self.noise_state.initialized = false;
    }

    /// 获取配置
    pub fn config(&self) -> &NsConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ns_creation() {
        let ns = NsProcessor::with_default_config();
        assert_eq!(ns.config().suppression_db, 12.0);
        assert_eq!(ns.config().window_size, 480);
    }

    #[test]
    fn test_ns_process_silence() {
        let mut ns = NsProcessor::with_default_config();

        // 静音信号应该被抑制
        let mut buffer = vec![0.001f32; 100];
        ns.process(&mut buffer).unwrap();

        // 静音信号应该被进一步衰减
        let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
        assert!(rms < 0.001, "Silence should be suppressed, RMS: {}", rms);
    }

    #[test]
    fn test_ns_process_signal() {
        let mut ns = NsProcessor::with_default_config();

        // 强信号应该被保留
        let mut buffer = vec![0.5f32; 100];
        ns.process(&mut buffer).unwrap();

        // 强信号应该保留大部分能量
        let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
        assert!(rms > 0.1, "Strong signal should be preserved, RMS: {}", rms);
    }

    #[test]
    fn test_ns_process_mixed() {
        let mut ns = NsProcessor::with_default_config();

        // 先处理噪声帧建立噪声模型
        let mut noise = vec![0.005f32; 100];
        ns.process(&mut noise).unwrap();

        // 然后处理信号帧
        let mut signal = vec![0.5f32; 100];
        ns.process(&mut signal).unwrap();

        // 信号帧应该被保留
        let rms: f32 = (signal.iter().map(|&s| s * s).sum::<f32>() / signal.len() as f32).sqrt();
        assert!(rms > 0.1, "Signal after noise model should be preserved, RMS: {}", rms);
    }

    #[test]
    fn test_ns_reset() {
        let mut ns = NsProcessor::with_default_config();

        // 处理一些数据
        let mut buffer = vec![0.5f32; 100];
        ns.process(&mut buffer).unwrap();

        // 重置后状态应该清零
        ns.reset();
        assert!(!ns.noise_state.initialized);
    }

    #[test]
    fn test_ns_empty_buffer() {
        let mut ns = NsProcessor::with_default_config();
        let mut buffer: Vec<f32> = vec![];
        assert!(ns.process(&mut buffer).is_ok());
    }
}

