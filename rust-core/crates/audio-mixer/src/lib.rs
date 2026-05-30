//! SoundBridge 音频混音模块
//!
//! 提供多路音频混音功能，支持音量控制和防削波。

/// 混音配置
#[derive(Debug, Clone)]
pub struct MixerConfig {
    /// 输出采样率
    pub sample_rate: u32,

    /// 输出通道数
    pub channels: u16,

    /// 是否启用防削波保护
    pub clipping_protection: bool,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            clipping_protection: true,
        }
    }
}

/// 混音错误类型
#[derive(Debug, thiserror::Error)]
pub enum MixerError {
    #[error("缓冲区格式不匹配")]
    FormatMismatch,

    #[error("缓冲区为空")]
    EmptyBuffers,

    #[error("缓冲区长度不一致: 期望 {expected}, 实际 {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("音量数量不匹配: 期望 {expected}, 实际 {actual}")]
    VolumeCountMismatch { expected: usize, actual: usize },
}

/// 混音结果类型
pub type Result<T> = std::result::Result<T, MixerError>;

/// 音频混音器
pub struct AudioMixer {
    config: MixerConfig,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(MixerConfig::default())
    }
}

impl AudioMixer {
    /// 创建新的混音器
    pub fn new(config: MixerConfig) -> Self {
        Self { config }
    }

    /// 混音多路音频
    ///
    /// # Arguments
    /// * `inputs` - 输入音频缓冲区列表
    /// * `volumes` - 每路音频的音量（0.0 到 1.0）
    ///
    /// # Returns
    /// 混音后的音频数据
    pub fn mix(&self, inputs: &[&[f32]], volumes: &[f32]) -> Result<Vec<f32>> {
        if inputs.is_empty() {
            return Err(MixerError::EmptyBuffers);
        }

        if inputs.len() != volumes.len() {
            return Err(MixerError::VolumeCountMismatch {
                expected: inputs.len(),
                actual: volumes.len(),
            });
        }

        // 检查所有输入长度一致
        let len = inputs[0].len();
        for input in inputs.iter() {
            if input.len() != len {
                return Err(MixerError::LengthMismatch {
                    expected: len,
                    actual: input.len(),
                });
            }
        }

        // 混音：加权求和
        let mut output = vec![0.0f32; len];
        for (input, &volume) in inputs.iter().zip(volumes.iter()) {
            for (out, &sample) in output.iter_mut().zip(input.iter()) {
                *out += sample * volume;
            }
        }

        // 防削波保护
        if self.config.clipping_protection {
            for sample in output.iter_mut() {
                *sample = self.soft_clip(*sample);
            }
        }

        Ok(output)
    }

    /// 混音两路音频（便捷方法）
    pub fn mix_two(&self, input1: &[f32], volume1: f32, input2: &[f32], volume2: f32) -> Result<Vec<f32>> {
        self.mix(&[input1, input2], &[volume1, volume2])
    }

    /// Soft clipping（tanh 压缩）
    ///
    /// 使用双曲正切函数进行软削波，避免硬削波产生的失真。
    /// 只有当值超出 [-1.0, 1.0] 范围时才应用压缩。
    fn soft_clip(&self, sample: f32) -> f32 {
        if sample.abs() > 1.0 {
            // tanh 软削波
            sample.signum() * (sample.abs() - 1.0).tanh()
        } else {
            sample
        }
    }

    /// 获取配置
    pub fn config(&self) -> &MixerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_creation() {
        let config = MixerConfig::default();
        let _mixer = AudioMixer::new(config);
    }

    #[test]
    fn test_mix_single_input() {
        let mixer = AudioMixer::default();
        let input = vec![0.5f32; 100];
        let result = mixer.mix(&[&input], &[1.0]).unwrap();
        assert_eq!(result.len(), 100);
        // 单路输入，音量 1.0，输出应该接近输入（经过 soft clip）
        for (out, &inp) in result.iter().zip(input.iter()) {
            assert!((out - inp).abs() < 0.01);
        }
    }

    #[test]
    fn test_mix_two_inputs_equal_volume() {
        let mixer = AudioMixer::default();
        let input1 = vec![0.5f32; 100];
        let input2 = vec![0.5f32; 100];
        let result = mixer.mix_two(&input1, 0.5, &input2, 0.5).unwrap();
        assert_eq!(result.len(), 100);
        // 两路 0.5，音量各 0.5，输出应该是 0.5
        for sample in result.iter() {
            assert!((sample - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_mix_silent_input() {
        let mixer = AudioMixer::default();
        let input1 = vec![0.0f32; 100];
        let input2 = vec![0.5f32; 100];
        let result = mixer.mix_two(&input1, 1.0, &input2, 0.0).unwrap();
        assert_eq!(result.len(), 100);
        // 一路静音，另一路音量 0.0，输出应该是 0.0
        for sample in result.iter() {
            assert!(sample.abs() < 0.001);
        }
    }

    #[test]
    fn test_mix_clipping_protection() {
        let mixer = AudioMixer::default();
        // 输入超过 1.0，测试防削波
        let input1 = vec![1.0f32; 100];
        let input2 = vec![1.0f32; 100];
        let result = mixer.mix_two(&input1, 1.0, input2.as_slice(), 1.0).unwrap();
        // 输出应该被限制在合理范围内
        for sample in result.iter() {
            assert!(*sample <= 2.0 && *sample >= -2.0);
        }
    }

    #[test]
    fn test_mix_no_clipping() {
        let config = MixerConfig {
            clipping_protection: false,
            ..Default::default()
        };
        let mixer = AudioMixer::new(config);
        let input1 = vec![1.0f32; 100];
        let input2 = vec![1.0f32; 100];
        let result = mixer.mix_two(&input1, 1.0, input2.as_slice(), 1.0).unwrap();
        // 不做防削波，输出应该是 2.0
        for sample in result.iter() {
            assert!((sample - 2.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_mix_empty_inputs() {
        let mixer = AudioMixer::default();
        let result = mixer.mix(&[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mix_volume_mismatch() {
        let mixer = AudioMixer::default();
        let input = vec![0.5f32; 100];
        let result = mixer.mix(&[&input], &[1.0, 0.5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mix_length_mismatch() {
        let mixer = AudioMixer::default();
        let input1 = vec![0.5f32; 100];
        let input2 = vec![0.5f32; 50];
        let result = mixer.mix_two(&input1, 1.0, &input2, 1.0);
        assert!(result.is_err());
    }
}
