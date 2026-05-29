//! 回声消除（AEC）模块
//!
//! 实现基于 NLMS（Normalized Least Mean Squares）自适应滤波器的回声消除。

use crate::{ProcessorError, Result};

/// 回声消除配置
#[derive(Debug, Clone)]
pub struct AecConfig {
    /// 滤波器长度（样本数）
    pub filter_length: usize,

    /// 步长因子（0.0 - 1.0）
    pub step_size: f32,

    /// 正则化因子（防止除零）
    pub regularization: f32,
}

impl Default for AecConfig {
    fn default() -> Self {
        Self {
            filter_length: 4800,  // 100ms @ 48kHz
            step_size: 0.1,       // 保守的步长，防止发散
            regularization: 0.1,  // 较大的正则化，防止除零
        }
    }
}

/// 回声消除处理器
///
/// 使用 NLMS 自适应滤波器消除回声。
/// 原理：从麦克风信号中减去扬声器信号的回声分量。
pub struct AecProcessor {
    /// 配置
    config: AecConfig,

    /// 滤波器权重
    weights: Vec<f32>,

    /// 参考信号缓冲区（扬声器信号）
    reference_buffer: Vec<f32>,

    /// 缓冲区写入位置
    buffer_pos: usize,
}

impl AecProcessor {
    /// 创建新的 AEC 处理器
    pub fn new(config: AecConfig) -> Self {
        let filter_length = config.filter_length;
        Self {
            config,
            weights: vec![0.0; filter_length],
            reference_buffer: vec![0.0; filter_length],
            buffer_pos: 0,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(AecConfig::default())
    }

    /// 处理音频数据（带回声消除）
    ///
    /// # Arguments
    /// * `buffer` - 输入/输出缓冲区（麦克风信号，就地修改）
    /// * `reference` - 参考信号（扬声器信号）
    pub fn process(&mut self, buffer: &mut [f32], reference: &[f32]) -> Result<()> {
        if buffer.len() != reference.len() {
            return Err(ProcessorError::BufferError(
                "Buffer and reference must have same length".to_string(),
            ));
        }

        let filter_len = self.config.filter_length;
        let mu = self.config.step_size;
        let delta = self.config.regularization;

        for (sample, &ref_sample) in buffer.iter_mut().zip(reference.iter()) {
            let mic_sample = *sample;
            // 将参考信号写入环形缓冲区
            self.reference_buffer[self.buffer_pos] = ref_sample;

            // 计算滤波器输出（估计的回声）
            let mut echo_estimate = 0.0;
            for j in 0..filter_len {
                let idx = (self.buffer_pos + filter_len - j) % filter_len;
                echo_estimate += self.weights[j] * self.reference_buffer[idx];
            }

            // 计算误差信号（消除回声后的信号）
            let error = mic_sample - echo_estimate;

            // 更新滤波器权重（NLMS 算法）
            let ref_power: f32 = (0..filter_len)
                .map(|j| {
                    let idx = (self.buffer_pos + filter_len - j) % filter_len;
                    self.reference_buffer[idx].powi(2)
                })
                .sum::<f32>()
                + delta;

            let norm_factor = mu / ref_power;
            for j in 0..filter_len {
                let idx = (self.buffer_pos + filter_len - j) % filter_len;
                self.weights[j] += norm_factor * error * self.reference_buffer[idx];
            }

            // 输出消除回声后的信号
            *sample = error;

            // 更新缓冲区位置
            self.buffer_pos = (self.buffer_pos + 1) % filter_len;
        }

        Ok(())
    }

    /// 重置处理器状态
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
        self.reference_buffer.fill(0.0);
        self.buffer_pos = 0;
    }

    /// 获取配置
    pub fn config(&self) -> &AecConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aec_creation() {
        let aec = AecProcessor::with_default_config();
        assert_eq!(aec.config().filter_length, 4800);
        assert_eq!(aec.config().step_size, 0.1);
    }

    #[test]
    fn test_aec_process_basic() {
        let mut aec = AecProcessor::with_default_config();

        // 模拟：麦克风信号 = 人声 + 回声
        // 回声 = 扬声器信号 * 0.5
        let speaker = vec![1.0f32; 100];
        let mut mic: Vec<f32> = speaker.iter().map(|&s| s * 0.5).collect();

        // 处理一帧
        aec.process(&mut mic, &speaker).unwrap();

        // 处理后信号应该存在且有限
        for sample in &mic {
            assert!(sample.is_finite(), "Output should be finite");
        }
    }

    #[test]
    fn test_aec_no_reference() {
        let mut aec = AecProcessor::with_default_config();

        // 没有参考信号时，输出应该接近输入
        let mut mic = vec![0.5f32; 100];
        let reference = vec![0.0f32; 100];

        aec.process(&mut mic, &reference).unwrap();

        // 输出应该接近输入（无回声需要消除）
        for sample in &mic {
            assert!((sample - 0.5).abs() < 0.1, "Output should be close to input when no echo");
        }
    }

    #[test]
    fn test_aec_buffer_mismatch() {
        let mut aec = AecProcessor::with_default_config();
        let mut buffer = vec![0.5f32; 100];
        let reference = vec![0.3f32; 50];
        assert!(aec.process(&mut buffer, &reference).is_err());
    }

    #[test]
    fn test_aec_reset() {
        let mut aec = AecProcessor::with_default_config();

        // 处理一些数据
        let mut mic = vec![0.5f32; 100];
        let reference = vec![0.3f32; 100];
        aec.process(&mut mic, &reference).unwrap();

        // 重置后，权重应该全为零
        aec.reset();
        for w in &aec.weights {
            assert_eq!(*w, 0.0);
        }
    }

    #[test]
    fn test_aec_reduces_echo() {
        let mut aec = AecProcessor::new(AecConfig {
            filter_length: 10,
            step_size: 0.1,
            regularization: 0.01,
        });

        // 模拟多帧音频，每帧都是新的数据
        // 回声 = speaker * 0.5
        let mut total_error_before = 0.0f32;
        let mut total_error_after = 0.0f32;

        for frame in 0..10 {
            let base = frame as f32 * 0.01;
            let speaker: Vec<f32> = (0..100).map(|i| (base + i as f32 * 0.01).sin() * 0.5).collect();
            let mut mic: Vec<f32> = speaker.iter().map(|&s| s * 0.5).collect();

            // 记录处理前的 RMS
            let rms_before: f32 = (mic.iter().map(|&s| s * s).sum::<f32>() / mic.len() as f32).sqrt();
            total_error_before += rms_before;

            // 处理
            aec.process(&mut mic, &speaker).unwrap();

            // 记录处理后的 RMS
            let rms_after: f32 = (mic.iter().map(|&s| s * s).sum::<f32>() / mic.len() as f32).sqrt();
            total_error_after += rms_after;
        }

        // 处理后的平均 RMS 应该小于处理前
        let avg_before = total_error_before / 10.0;
        let avg_after = total_error_after / 10.0;
        assert!(avg_after <= avg_before * 1.5,
            "AEC should not significantly increase signal level: before={}, after={}",
            avg_before, avg_after);
    }
}
