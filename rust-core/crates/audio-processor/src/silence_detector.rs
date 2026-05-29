//! 静音检测模块

use crate::Result;

/// 静音检测器
pub struct SilenceDetector {
    threshold: f32,
}

impl SilenceDetector {
    /// 创建新的静音检测器
    pub fn new(threshold_db: f32) -> Result<Self> {
        let threshold = 10.0f32.powf(threshold_db / 20.0);
        Ok(Self { threshold })
    }

    /// 检测是否为静音
    pub fn is_silence(&self, buffer: &[f32]) -> bool {
        if buffer.is_empty() {
            return true;
        }
        let rms = self.calculate_rms(buffer);
        rms < self.threshold
    }

    /// 计算 RMS（均方根）
    pub fn calculate_rms(&self, buffer: &[f32]) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = buffer.iter().map(|&s| s * s).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection() {
        let detector = SilenceDetector::new(-60.0).unwrap();
        let silence = vec![0.0f32; 100];
        assert!(detector.is_silence(&silence));

        let signal = vec![0.5f32; 100];
        assert!(!detector.is_silence(&signal));
    }

    #[test]
    fn test_rms_calculation() {
        let detector = SilenceDetector::new(-60.0).unwrap();

        let silence = vec![0.0f32; 100];
        assert_eq!(detector.calculate_rms(&silence), 0.0);

        let constant = vec![0.5f32; 100];
        let rms = detector.calculate_rms(&constant);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_buffer() {
        let detector = SilenceDetector::new(-60.0).unwrap();
        let empty: Vec<f32> = vec![];
        assert!(detector.is_silence(&empty));
        assert_eq!(detector.calculate_rms(&empty), 0.0);
    }
}
