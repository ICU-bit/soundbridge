//! 电平指示器模块
//!
//! 提供音频电平检测和可视化数据功能。

/// 电平指示器配置
#[derive(Debug, Clone)]
pub struct LevelIndicatorConfig {
    /// 更新间隔（毫秒）
    pub update_interval_ms: u32,

    /// 峰值保持时间（毫秒）
    pub peak_hold_ms: u32,

    /// 平滑因子（0.0 - 1.0）
    pub smoothing_factor: f32,
}

impl Default for LevelIndicatorConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 50,
            peak_hold_ms: 1000,
            smoothing_factor: 0.3,
        }
    }
}

/// 电平数据
#[derive(Debug, Clone)]
pub struct LevelData {
    /// 当前 RMS 电平（0.0 - 1.0）
    pub rms: f32,

    /// 峰值电平（0.0 - 1.0）
    pub peak: f32,

    /// 峰值保持（0.0 - 1.0）
    pub peak_hold: f32,

    /// 分贝值（dBFS）
    pub db: f32,
}

/// 电平指示器
pub struct LevelIndicator {
    config: LevelIndicatorConfig,
    current_rms: f32,
    current_peak: f32,
    peak_hold: f32,
    peak_hold_timer: std::time::Instant,
}

impl LevelIndicator {
    /// 创建新的电平指示器
    pub fn new(config: LevelIndicatorConfig) -> Self {
        Self {
            config,
            current_rms: 0.0,
            current_peak: 0.0,
            peak_hold: 0.0,
            peak_hold_timer: std::time::Instant::now(),
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(LevelIndicatorConfig::default())
    }

    /// 更新电平数据
    pub fn update(&mut self, buffer: &[f32]) -> LevelData {
        if buffer.is_empty() {
            return LevelData {
                rms: 0.0,
                peak: 0.0,
                peak_hold: self.peak_hold,
                db: -f32::INFINITY,
            };
        }

        // 计算 RMS
        let sum_squares: f32 = buffer.iter().map(|&s| s * s).sum();
        let rms = (sum_squares / buffer.len() as f32).sqrt();

        // 计算峰值
        let peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // 平滑处理
        self.current_rms = self.current_rms * (1.0 - self.config.smoothing_factor)
            + rms * self.config.smoothing_factor;
        self.current_peak = self.current_peak * (1.0 - self.config.smoothing_factor)
            + peak * self.config.smoothing_factor;

        // 峰值保持
        if self.current_peak > self.peak_hold {
            self.peak_hold = self.current_peak;
            self.peak_hold_timer = std::time::Instant::now();
        } else if self.peak_hold_timer.elapsed()
            > std::time::Duration::from_millis(self.config.peak_hold_ms as u64)
        {
            self.peak_hold = self.current_peak;
        }

        // 计算分贝值
        let db = if self.current_rms > 0.0 {
            20.0 * self.current_rms.log10()
        } else {
            -f32::INFINITY
        };

        LevelData {
            rms: self.current_rms,
            peak: self.current_peak,
            peak_hold: self.peak_hold,
            db,
        }
    }

    /// 重置指示器
    pub fn reset(&mut self) {
        self.current_rms = 0.0;
        self.current_peak = 0.0;
        self.peak_hold = 0.0;
    }

    /// 获取配置
    pub fn config(&self) -> &LevelIndicatorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_indicator_creation() {
        let indicator = LevelIndicator::with_default_config();
        assert_eq!(indicator.config().update_interval_ms, 50);
        assert_eq!(indicator.config().peak_hold_ms, 1000);
    }

    #[test]
    fn test_level_indicator_silence() {
        let mut indicator = LevelIndicator::with_default_config();
        let buffer = vec![0.0f32; 100];
        let data = indicator.update(&buffer);

        assert_eq!(data.rms, 0.0);
        assert_eq!(data.peak, 0.0);
        assert_eq!(data.db, -f32::INFINITY);
    }

    #[test]
    fn test_level_indicator_signal() {
        let mut indicator = LevelIndicator::with_default_config();
        let buffer = vec![0.5f32; 100];
        let data = indicator.update(&buffer);

        assert!(data.rms > 0.0);
        assert!(data.peak > 0.0);
        assert!(data.db > -f32::INFINITY);
    }

    #[test]
    fn test_level_indicator_peak_hold() {
        let mut indicator = LevelIndicator::with_default_config();

        // 第一次更新
        let buffer1 = vec![0.8f32; 100];
        let data1 = indicator.update(&buffer1);
        assert!(data1.peak_hold > 0.0);

        // 第二次更新（较小值）
        let buffer2 = vec![0.2f32; 100];
        let data2 = indicator.update(&buffer2);

        // 峰值保持应该保留之前的较大值
        assert!(data2.peak_hold >= data1.peak_hold * 0.9);
    }

    #[test]
    fn test_level_indicator_reset() {
        let mut indicator = LevelIndicator::with_default_config();
        let buffer = vec![0.5f32; 100];
        indicator.update(&buffer);

        indicator.reset();
        let data = indicator.update(&vec![0.0f32; 100]);
        assert_eq!(data.rms, 0.0);
    }

    #[test]
    fn test_level_indicator_db() {
        let mut indicator = LevelIndicator::with_default_config();

        // 多次更新以达到稳定状态
        let buffer = vec![1.0f32; 100];
        for _ in 0..10 {
            indicator.update(&buffer);
        }
        let data = indicator.update(&buffer);
        assert!(
            data.db > -6.0,
            "dB should be reasonable for full-scale signal, got {}",
            data.db
        );

        // -inf dBFS = 0.0
        indicator.reset();
        let buffer = vec![0.0f32; 100];
        let data = indicator.update(&buffer);
        assert_eq!(data.db, -f32::INFINITY);
    }
}
