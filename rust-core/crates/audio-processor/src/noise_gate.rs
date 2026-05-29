//! 噪声门模块

use crate::Result;

/// 噪声门处理器
pub struct NoiseGate {
    threshold: f32,
}

impl NoiseGate {
    /// 创建新的噪声门处理器
    pub fn new(threshold_db: f32) -> Result<Self> {
        let threshold = 10.0f32.powf(threshold_db / 20.0);
        Ok(Self { threshold })
    }

    /// 处理音频数据（就地修改）
    ///
    /// 低于阈值的信号置零。
    pub fn process(&self, buffer: &mut [f32]) -> Result<()> {
        for sample in buffer.iter_mut() {
            if sample.abs() < self.threshold {
                *sample = 0.0;
            }
        }
        Ok(())
    }

    /// 获取阈值
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_gate() {
        let gate = NoiseGate::new(-40.0).unwrap();

        // 低于阈值的信号应该被置零
        let mut buffer = vec![0.005f32; 100]; // 约 -46 dB
        gate.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!(sample.abs() < 0.001);
        }

        // 高于阈值的信号应该保留
        let mut buffer = vec![0.1f32; 100]; // 约 -20 dB
        gate.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!(sample.abs() > 0.01);
        }
    }
}
