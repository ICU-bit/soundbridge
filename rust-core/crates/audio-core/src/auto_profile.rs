use std::time::Instant;

use crate::audio_profile::AudioProfile;

/// 自动挡配置
#[derive(Debug, Clone)]
pub struct AutoProfileConfig {
    pub min_quality: AudioProfile,
    pub max_quality: AudioProfile,
    pub prefer_quality: bool,
    pub lock_when_stable: bool,
    pub stability_threshold_ms: u64,
}

impl Default for AutoProfileConfig {
    fn default() -> Self {
        Self {
            min_quality: AudioProfile::BandwidthSaving,
            max_quality: AudioProfile::HighResolution,
            prefer_quality: true,
            lock_when_stable: false,
            stability_threshold_ms: 30_000,
        }
    }
}

/// 网络状况评分
#[derive(Debug, Clone, Copy)]
pub struct NetworkScore {
    pub bandwidth_mbps: f32,
    pub latency_ms: f32,
    pub loss_rate: f32,
}

/// 自动档管理器
///
/// 根据网络状况自动选择合适的音质档位。
/// 降档立即生效，升档需要持续 3 秒且间隔至少 10 秒。
pub struct AutoProfileManager {
    config: AutoProfileConfig,
    current_profile: AudioProfile,
    target_profile: AudioProfile,
    last_change: Instant,
    last_upgrade: Instant,
    _stable_since: Option<Instant>,
    debounce_start: Option<Instant>,
}

impl AutoProfileManager {
    pub fn new(config: AutoProfileConfig) -> Self {
        Self {
            config,
            current_profile: AudioProfile::Standard,
            target_profile: AudioProfile::Standard,
            last_change: Instant::now(),
            last_upgrade: Instant::now(),
            _stable_since: None,
            debounce_start: None,
        }
    }

    pub fn current_profile(&self) -> AudioProfile {
        self.current_profile
    }

    pub fn set_profile(&mut self, profile: AudioProfile) {
        self.current_profile = profile;
        self.target_profile = profile;
        self.last_change = Instant::now();
    }

    pub fn update(&mut self, score: NetworkScore) -> AudioProfile {
        let new_profile = self.calculate_profile(score);

        // 防抖动逻辑
        if new_profile != self.target_profile {
            self.target_profile = new_profile;
            self.debounce_start = Some(Instant::now());
        }

        if let Some(debounce_start) = self.debounce_start {
            let elapsed = debounce_start.elapsed().as_millis() as u64;

            // 降档立即生效
            if self.is_downgrade(new_profile) {
                self.current_profile = new_profile;
                self.last_change = Instant::now();
                self.debounce_start = None;
            }
            // 升档需要持续 3 秒
            else if elapsed >= 3000 {
                // 升档间隔至少 10 秒
                if self.last_upgrade.elapsed().as_secs() >= 10 {
                    self.current_profile = new_profile;
                    self.last_change = Instant::now();
                    self.last_upgrade = Instant::now();
                    self.debounce_start = None;
                }
            }
        }

        self.current_profile
    }

    fn calculate_profile(&self, score: NetworkScore) -> AudioProfile {
        let bandwidth_score = if score.bandwidth_mbps > 0.0 {
            (score.bandwidth_mbps / 10.0).min(1.0) * 0.4
        } else {
            0.0
        };

        let latency_score = if score.latency_ms > 0.0 {
            (50.0 / score.latency_ms).min(1.0) * 0.3
        } else {
            0.3
        };

        let loss_score = (1.0 - score.loss_rate).max(0.0) * 0.3;

        let total_score = bandwidth_score + latency_score + loss_score;

        let profile = if total_score >= 0.9 {
            AudioProfile::HighResolution
        } else if total_score >= 0.7 {
            AudioProfile::Lossless
        } else if total_score >= 0.5 {
            AudioProfile::HighQuality
        } else if total_score >= 0.3 {
            AudioProfile::Standard
        } else {
            AudioProfile::BandwidthSaving
        };

        // 应用配置限制
        let profile_index = Self::profile_index(profile);
        let min_index = Self::profile_index(self.config.min_quality);
        let max_index = Self::profile_index(self.config.max_quality);

        let clamped_index = profile_index.clamp(min_index, max_index);
        Self::index_to_profile(clamped_index)
    }

    fn is_downgrade(&self, new_profile: AudioProfile) -> bool {
        Self::profile_index(new_profile) < Self::profile_index(self.current_profile)
    }

    /// 获取档位的排序索引（用于比较和 clamp）
    pub fn profile_index(profile: AudioProfile) -> u8 {
        match profile {
            AudioProfile::BandwidthSaving => 0,
            AudioProfile::Standard => 1,
            AudioProfile::HighQuality => 2,
            AudioProfile::Lossless => 3,
            AudioProfile::HighResolution => 4,
            AudioProfile::StudioMaster => 5,
            _ => 1,
        }
    }

    fn index_to_profile(index: u8) -> AudioProfile {
        match index {
            0 => AudioProfile::BandwidthSaving,
            1 => AudioProfile::Standard,
            2 => AudioProfile::HighQuality,
            3 => AudioProfile::Lossless,
            4 => AudioProfile::HighResolution,
            5 => AudioProfile::StudioMaster,
            _ => AudioProfile::Standard,
        }
    }
}
