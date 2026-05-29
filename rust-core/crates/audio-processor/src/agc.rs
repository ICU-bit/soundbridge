//! 自动增益控制（AGC）模块
//!
//! 实现带有攻击/释放时间的自动增益控制。

use crate::Result;

/// AGC 配置
#[derive(Debug, Clone)]
pub struct AgcConfig {
    /// 目标电平（dBFS）
    pub target_dbfs: f32,

    /// 最大增益（dB）
    pub max_gain_db: f32,

    /// 攻击时间（毫秒）- 信号增大时的响应速度
    pub attack_ms: f32,

    /// 释放时间（毫秒）- 信号减小时的响应速度
    pub release_ms: f32,

    /// 采样率
    pub sample_rate: u32,
}

impl Default for AgcConfig {
    fn default() -> Self {
        Self {
            target_dbfs: -3.0,  // 技术规格 §3.3: 目标电平 -3 dBFS
            max_gain_db: 30.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            sample_rate: 48000,
        }
    }
}

/// 自动增益控制处理器
///
/// 使用攻击/释放时间平滑增益变化，避免"泵浦效应"。
pub struct AgcProcessor {
    /// 配置
    config: AgcConfig,

    /// 当前增益（线性）
    current_gain: f32,

    /// 攻击系数（每样本）
    attack_coeff: f32,

    /// 释放系数（每样本）
    release_coeff: f32,

    /// 最大增益（线性）
    max_gain_linear: f32,

    /// 目标 RMS（线性）
    target_rms: f32,
}

impl AgcProcessor {
    /// 创建新的 AGC 处理器
    pub fn new(config: AgcConfig) -> Self {
        let attack_coeff = (-1.0 / (config.attack_ms * config.sample_rate as f32 / 1000.0)).exp();
        let release_coeff = (-1.0 / (config.release_ms * config.sample_rate as f32 / 1000.0)).exp();
        let max_gain_linear = 10.0f32.powf(config.max_gain_db / 20.0);
        let target_rms = 10.0f32.powf(config.target_dbfs / 20.0);

        Self {
            config,
            current_gain: 1.0,
            attack_coeff,
            release_coeff,
            max_gain_linear,
            target_rms,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(AgcConfig::default())
    }

    /// 处理音频数据（就地修改）
    pub fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        for sample in buffer.iter_mut() {
            // 计算当前样本的幅度
            let input_level = sample.abs();

            if input_level > 0.0001 {
                // 计算所需增益
                let required_gain = self.target_rms / input_level;

                // 限制增益范围
                let target_gain = required_gain.clamp(0.1, self.max_gain_linear);

                // 平滑增益变化（攻击/释放）
                let coeff = if target_gain < self.current_gain {
                    self.attack_coeff  // 信号增大，快速响应
                } else {
                    self.release_coeff // 信号减小，慢速响应
                };

                self.current_gain = coeff * self.current_gain + (1.0 - coeff) * target_gain;
            }

            // 应用增益
            *sample *= self.current_gain;
        }

        Ok(())
    }

    /// 重置处理器状态
    pub fn reset(&mut self) {
        self.current_gain = 1.0;
    }

    /// 获取当前增益（dB）
    pub fn current_gain_db(&self) -> f32 {
        20.0 * self.current_gain.log10()
    }

    /// 获取配置
    pub fn config(&self) -> &AgcConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agc_creation() {
        let agc = AgcProcessor::with_default_config();
        assert_eq!(agc.config().target_dbfs, -3.0);  // 技术规格 §3.3
        assert_eq!(agc.config().max_gain_db, 30.0);
    }

    #[test]
    fn test_agc_process_quiet_signal() {
        let mut agc = AgcProcessor::with_default_config();

        // 处理安静信号
        let mut buffer = vec![0.01f32; 1000];
        agc.process(&mut buffer).unwrap();

        // 安静信号应该被放大
        let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
        assert!(rms > 0.01, "Signal should be amplified, RMS: {}", rms);
    }

    #[test]
    fn test_agc_process_loud_signal() {
        let mut agc = AgcProcessor::with_default_config();

        // 处理响亮信号
        let mut buffer = vec![0.9f32; 1000];
        agc.process(&mut buffer).unwrap();

        // 响亮信号应该被衰减
        let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
        assert!(rms < 0.9, "Signal should be attenuated, RMS: {}", rms);
    }

    #[test]
    fn test_agc_temporal_smoothing() {
        let mut agc = AgcProcessor::with_default_config();

        // 处理一系列安静帧
        for _ in 0..10 {
            let mut buffer = vec![0.01f32; 100];
            agc.process(&mut buffer).unwrap();
        }
        let gain_after_quiet = agc.current_gain_db();

        // 处理一系列响亮帧
        for _ in 0..10 {
            let mut buffer = vec![0.9f32; 100];
            agc.process(&mut buffer).unwrap();
        }
        let gain_after_loud = agc.current_gain_db();

        // 增益应该平滑变化，而不是突变
        assert!(gain_after_loud < gain_after_quiet, 
            "Gain should decrease for loud signal: quiet={}, loud={}", 
            gain_after_quiet, gain_after_loud);
    }

    #[test]
    fn test_agc_no_pumping() {
        let mut agc = AgcProcessor::with_default_config();

        // 交替处理安静和响亮信号
        let mut gains = Vec::new();
        for i in 0..20 {
            let mut buffer = if i % 2 == 0 {
                vec![0.01f32; 100]
            } else {
                vec![0.9f32; 100]
            };
            agc.process(&mut buffer).unwrap();
            gains.push(agc.current_gain_db());
        }

        // 增益变化应该平滑，没有剧烈跳动
        for i in 1..gains.len() {
            let delta = (gains[i] - gains[i-1]).abs();
            assert!(delta < 10.0, "Gain change should be smooth: delta={}", delta);
        }
    }

    #[test]
    fn test_agc_reset() {
        let mut agc = AgcProcessor::with_default_config();

        // 处理一些数据
        let mut buffer = vec![0.5f32; 100];
        agc.process(&mut buffer).unwrap();

        // 重置后增益应该回到 1.0
        agc.reset();
        assert_eq!(agc.current_gain_db(), 0.0);
    }

    #[test]
    fn test_agc_empty_buffer() {
        let mut agc = AgcProcessor::with_default_config();
        let mut buffer: Vec<f32> = vec![];
        assert!(agc.process(&mut buffer).is_ok());
    }
}
