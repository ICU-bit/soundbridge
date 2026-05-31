//! 参数均衡器
//!
//! 10 段参数均衡器，基于 Biquad 滤波器实现。
//! 支持预设模式（Flat / Gaming / Music / Voice / Bass / Treble）和自定义频段调节。

use std::f32::consts::PI;

/// Biquad 滤波器（Direct Form I）
///
/// 使用 peaking EQ 类型实现参数均衡器的各频段。
#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    /// 从滤波器系数创建
    pub fn new(a0: f32, a1: f32, a2: f32, b1: f32, b2: f32) -> Self {
        Self {
            a0,
            a1,
            a2,
            b1,
            b2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// 创建 peaking EQ 滤波器
    ///
    /// # Arguments
    /// * `sample_rate` - 采样率（Hz）
    /// * `center_freq` - 中心频率（Hz）
    /// * `gain_db` - 增益（dB）
    /// * `q` - Q 值（品质因数）
    pub fn peaking(sample_rate: f32, center_freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * center_freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * w0.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha / a;

        Self::new(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    /// 处理单个采样点
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2
            - self.b1 * self.y1
            - self.b2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    /// 处理整个缓冲区
    pub fn process_buffer(&mut self, input: &[f32], output: &mut [f32]) {
        for (i, &sample) in input.iter().enumerate() {
            output[i] = self.process(sample);
        }
    }

    /// 重置滤波器状态
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// 均衡器预设
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqPreset {
    /// 平坦（无调节）
    Flat,
    /// 游戏模式
    Gaming,
    /// 音乐模式
    Music,
    /// 人声模式
    Voice,
    /// 低音增强
    Bass,
    /// 高音增强
    Treble,
}

impl EqPreset {
    /// 获取预设的 10 段参数，每段为 (gain_db, q)
    pub fn bands(&self) -> [(f32, f32); 10] {
        match self {
            Self::Flat => [(0.0, 1.0); 10],
            Self::Gaming => [
                (3.0, 1.0),
                (2.0, 1.0),
                (1.0, 1.0),
                (0.0, 1.0),
                (2.0, 1.0),
                (3.0, 1.0),
                (2.0, 1.0),
                (1.0, 1.0),
                (0.0, 1.0),
                (-1.0, 1.0),
            ],
            Self::Music => [
                (3.0, 1.0),
                (2.0, 1.0),
                (0.0, 1.0),
                (-1.0, 1.0),
                (-2.0, 1.0),
                (-1.0, 1.0),
                (0.0, 1.0),
                (2.0, 1.0),
                (3.0, 1.0),
                (4.0, 1.0),
            ],
            Self::Voice => [
                (-3.0, 1.0),
                (-2.0, 1.0),
                (0.0, 1.0),
                (2.0, 1.0),
                (4.0, 1.0),
                (4.0, 1.0),
                (3.0, 1.0),
                (1.0, 1.0),
                (0.0, 1.0),
                (-2.0, 1.0),
            ],
            Self::Bass => [
                (6.0, 1.0),
                (5.0, 1.0),
                (4.0, 1.0),
                (2.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
            ],
            Self::Treble => [
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (2.0, 1.0),
                (4.0, 1.0),
                (5.0, 1.0),
                (6.0, 1.0),
            ],
        }
    }
}

/// 10 段参数均衡器
pub struct ParametricEq {
    bands: [BiquadFilter; 10],
    sample_rate: u32,
    enabled: bool,
}

impl ParametricEq {
    /// 10 段中心频率（Hz）
    const CENTER_FREQUENCIES: [f32; 10] = [
        31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];

    /// 创建新的参数均衡器
    pub fn new(sample_rate: u32) -> Self {
        let default_filter = BiquadFilter::peaking(sample_rate as f32, 1000.0, 0.0, 1.0);
        Self {
            bands: [default_filter; 10],
            sample_rate,
            enabled: true,
        }
    }

    /// 设置单个频段
    pub fn set_band(&mut self, index: usize, gain_db: f32, q: f32) {
        if index < 10 {
            let freq = Self::CENTER_FREQUENCIES[index];
            self.bands[index] = BiquadFilter::peaking(self.sample_rate as f32, freq, gain_db, q);
        }
    }

    /// 应用预设
    pub fn set_preset(&mut self, preset: EqPreset) {
        let bands = preset.bands();
        for (i, (gain_db, q)) in bands.iter().enumerate() {
            self.set_band(i, *gain_db, *q);
        }
    }

    /// 启用/禁用均衡器
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 均衡器是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 处理音频缓冲区
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        if !self.enabled {
            output[..input.len()].copy_from_slice(input);
            return;
        }

        let mut temp = vec![0.0; input.len()];
        self.bands[0].process_buffer(input, &mut temp);

        for i in 1..10 {
            let mut next = vec![0.0; temp.len()];
            self.bands[i].process_buffer(&temp, &mut next);
            temp = next;
        }

        output[..temp.len()].copy_from_slice(&temp);
    }

    /// 重置所有滤波器状态
    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}
