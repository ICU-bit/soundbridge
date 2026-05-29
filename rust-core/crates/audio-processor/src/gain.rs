//! 增益控制模块

use crate::Result;

/// 增益处理器
pub struct GainProcessor {
    gain_factor: f32,
}

impl GainProcessor {
    /// 创建新的增益处理器
    pub fn new(gain_db: f32) -> Result<Self> {
        let gain_factor = 10.0f32.powf(gain_db / 20.0);
        Ok(Self { gain_factor })
    }

    /// 处理音频数据（就地修改）
    pub fn process(&self, buffer: &mut [f32]) -> Result<()> {
        for sample in buffer.iter_mut() {
            *sample *= self.gain_factor;
        }
        Ok(())
    }

    /// 获取增益因子
    pub fn gain_factor(&self) -> f32 {
        self.gain_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain_zero_db() {
        let gain = GainProcessor::new(0.0).unwrap();
        let mut buffer = vec![0.5f32; 100];
        gain.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!((sample - 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn test_gain_6db() {
        let gain = GainProcessor::new(6.0).unwrap();
        let mut buffer = vec![0.5f32; 100];
        gain.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!((sample - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_gain_minus_6db() {
        let gain = GainProcessor::new(-6.0).unwrap();
        let mut buffer = vec![1.0f32; 100];
        gain.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!((sample - 0.5).abs() < 0.01);
        }
    }
}
