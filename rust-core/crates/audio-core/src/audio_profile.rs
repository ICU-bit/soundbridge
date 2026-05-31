/// 音质档位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioProfile {
    /// 节省带宽: 24kHz, Mono, 32kbps
    BandwidthSaving,
    /// 标准: 48kHz, Mono, 128kbps
    Standard,
    /// 高质量: 48kHz, Stereo, 256kbps
    HighQuality,
    /// 无损: 96kHz, Stereo, 512kbps
    Lossless,
    /// 高解析度: 192kHz, Stereo, 1024kbps
    HighResolution,
    /// 录音室母带: 192kHz, Stereo, 9216kbps
    StudioMaster,
    /// 自动挡
    Auto,
    /// 自定义
    Custom,
}

/// 音频配置
#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate: u32,
    pub frame_size: u32,
    pub complexity: u32,
}

impl AudioConfig {
    pub fn for_profile(profile: AudioProfile) -> Option<Self> {
        match profile {
            AudioProfile::BandwidthSaving => Some(Self {
                sample_rate: 24_000,
                channels: 1,
                bitrate: 32_000,
                frame_size: 480,
                complexity: 5,
            }),
            AudioProfile::Standard => Some(Self {
                sample_rate: 48_000,
                channels: 1,
                bitrate: 128_000,
                frame_size: 960,
                complexity: 7,
            }),
            AudioProfile::HighQuality => Some(Self {
                sample_rate: 48_000,
                channels: 2,
                bitrate: 256_000,
                frame_size: 1920,
                complexity: 8,
            }),
            AudioProfile::Lossless => Some(Self {
                sample_rate: 96_000,
                channels: 2,
                bitrate: 512_000,
                frame_size: 3840,
                complexity: 9,
            }),
            AudioProfile::HighResolution => Some(Self {
                sample_rate: 192_000,
                channels: 2,
                bitrate: 1_024_000,
                frame_size: 7680,
                complexity: 10,
            }),
            AudioProfile::StudioMaster => Some(Self {
                sample_rate: 192_000,
                channels: 2,
                bitrate: 9_216_000,
                frame_size: 7680,
                complexity: 10,
            }),
            AudioProfile::Auto | AudioProfile::Custom => None,
        }
    }

    /// 网络带宽需求 (Mbps)，含 20% 余量
    pub fn network_requirement_mbps(&self) -> f32 {
        self.bitrate as f32 / 1_000_000.0 * 1.2
    }
}

impl AudioProfile {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BandwidthSaving => "节省带宽",
            Self::Standard => "标准",
            Self::HighQuality => "高质量",
            Self::Lossless => "无损",
            Self::HighResolution => "高解析度",
            Self::StudioMaster => "录音室母带",
            Self::Auto => "自动挡",
            Self::Custom => "自定义",
        }
    }

    pub fn all_profiles() -> &'static [AudioProfile] {
        &[
            Self::BandwidthSaving,
            Self::Standard,
            Self::HighQuality,
            Self::Lossless,
            Self::HighResolution,
            Self::StudioMaster,
            Self::Auto,
            Self::Custom,
        ]
    }
}
