//! 立体声混音模块
//!
//! 提供单声道↔立体声转换和立体声混音功能。

/// 立体声处理错误类型
#[derive(Debug, thiserror::Error)]
pub enum StereoError {
    #[error("缓冲区大小不匹配: 期望 {expected}, 实际 {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },
}

/// 立体声处理结果类型
pub type Result<T> = std::result::Result<T, StereoError>;

/// 单声道转立体声
///
/// 将单声道采样复制到左右两个声道。
pub fn mono_to_stereo(mono: &[f32], stereo: &mut [f32]) -> Result<()> {
    if stereo.len() != mono.len() * 2 {
        return Err(StereoError::BufferSizeMismatch {
            expected: mono.len() * 2,
            actual: stereo.len(),
        });
    }

    for (i, &sample) in mono.iter().enumerate() {
        stereo[i * 2] = sample;
        stereo[i * 2 + 1] = sample;
    }

    Ok(())
}

/// 立体声转单声道
///
/// 将左右声道取平均值合并为单声道。
pub fn stereo_to_mono(stereo: &[f32], mono: &mut [f32]) -> Result<()> {
    if mono.len() != stereo.len() / 2 {
        return Err(StereoError::BufferSizeMismatch {
            expected: stereo.len() / 2,
            actual: mono.len(),
        });
    }

    for i in 0..mono.len() {
        mono[i] = (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5;
    }

    Ok(())
}

/// 立体声混音器
///
/// 根据配置的声道数进行上混（mono→stereo）或下混（stereo→mono）。
pub struct StereoMixer {
    channels: u32,
}

impl StereoMixer {
    /// 创建新的立体声混音器
    pub fn new(channels: u32) -> Self {
        Self { channels }
    }

    /// 设置输出声道数
    pub fn set_channels(&mut self, channels: u32) {
        self.channels = channels;
    }

    /// 获取当前声道数
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// 上混：单声道输入 → 多声道输出
    ///
    /// 当 channels=1 时直通，channels=2 时复制到左右声道。
    pub fn mix(&self, input: &[f32], output: &mut [f32]) -> Result<()> {
        match self.channels {
            1 => {
                output[..input.len()].copy_from_slice(input);
                Ok(())
            }
            2 => mono_to_stereo(input, output),
            _ => Err(StereoError::BufferSizeMismatch {
                expected: input.len(),
                actual: output.len(),
            }),
        }
    }

    /// 下混：多声道输入 → 单声道输出
    ///
    /// 当 channels=1 时直通，channels=2 时左右声道取平均。
    pub fn downmix(&self, input: &[f32], output: &mut [f32]) -> Result<()> {
        match self.channels {
            1 => {
                output[..input.len()].copy_from_slice(input);
                Ok(())
            }
            2 => stereo_to_mono(input, output),
            _ => Err(StereoError::BufferSizeMismatch {
                expected: input.len(),
                actual: output.len(),
            }),
        }
    }
}
