use audio_core::audio_profile::AudioProfile;
use audio_core::auto_profile::*;

#[test]
fn test_auto_profile_good_network() {
    let config = AutoProfileConfig::default();
    let mut manager = AutoProfileManager::new(config);

    let score = NetworkScore {
        bandwidth_mbps: 20.0,
        latency_ms: 20.0,
        loss_rate: 0.001,
    };

    let profile = manager.update(score);
    assert_eq!(profile, AudioProfile::Standard); // 需要防抖动，首次不升档
}

#[test]
fn test_auto_profile_poor_network() {
    let config = AutoProfileConfig::default();
    let mut manager = AutoProfileManager::new(config);
    manager.set_profile(AudioProfile::HighQuality);

    let score = NetworkScore {
        bandwidth_mbps: 0.5,
        latency_ms: 300.0,
        loss_rate: 0.25,
    };

    let profile = manager.update(score);
    assert_eq!(profile, AudioProfile::BandwidthSaving); // 降档立即生效
}

#[test]
fn test_auto_profile_config_limits() {
    let config = AutoProfileConfig {
        min_quality: AudioProfile::Standard,
        max_quality: AudioProfile::Lossless,
        ..Default::default()
    };
    let mut manager = AutoProfileManager::new(config);

    let score = NetworkScore {
        bandwidth_mbps: 100.0,
        latency_ms: 5.0,
        loss_rate: 0.0,
    };

    let profile = manager.update(score);
    assert!(
        AutoProfileManager::profile_index(profile)
            <= AutoProfileManager::profile_index(AudioProfile::Lossless)
    );
}

#[test]
fn test_profile_index_ordering() {
    assert!(
        AutoProfileManager::profile_index(AudioProfile::BandwidthSaving)
            < AutoProfileManager::profile_index(AudioProfile::Standard)
    );
    assert!(
        AutoProfileManager::profile_index(AudioProfile::Standard)
            < AutoProfileManager::profile_index(AudioProfile::HighQuality)
    );
    assert!(
        AutoProfileManager::profile_index(AudioProfile::HighQuality)
            < AutoProfileManager::profile_index(AudioProfile::Lossless)
    );
    assert!(
        AutoProfileManager::profile_index(AudioProfile::Lossless)
            < AutoProfileManager::profile_index(AudioProfile::HighResolution)
    );
    assert!(
        AutoProfileManager::profile_index(AudioProfile::HighResolution)
            < AutoProfileManager::profile_index(AudioProfile::StudioMaster)
    );
}

#[test]
fn test_auto_profile_config_default() {
    let config = AutoProfileConfig::default();
    assert_eq!(config.min_quality, AudioProfile::BandwidthSaving);
    assert_eq!(config.max_quality, AudioProfile::HighResolution);
    assert!(config.prefer_quality);
    assert!(!config.lock_when_stable);
    assert_eq!(config.stability_threshold_ms, 30_000);
}
