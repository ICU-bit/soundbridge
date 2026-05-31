use audio_core::audio_profile::*;

#[test]
fn test_profile_configs() {
    let config = AudioConfig::for_profile(AudioProfile::Standard).unwrap();
    assert_eq!(config.sample_rate, 48_000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.bitrate, 128_000);
    assert_eq!(config.frame_size, 960);
    assert_eq!(config.complexity, 7);
}

#[test]
fn test_all_profiles_have_configs() {
    // 固定档位都有配置
    for profile in [
        AudioProfile::BandwidthSaving,
        AudioProfile::Standard,
        AudioProfile::HighQuality,
        AudioProfile::Lossless,
        AudioProfile::HighResolution,
        AudioProfile::StudioMaster,
    ] {
        let config = AudioConfig::for_profile(profile);
        assert!(config.is_some(), "{:?} should have config", profile);
    }

    // Auto 和 Custom 没有固定配置
    assert!(AudioConfig::for_profile(AudioProfile::Auto).is_none());
    assert!(AudioConfig::for_profile(AudioProfile::Custom).is_none());
}

#[test]
fn test_profile_names() {
    assert_eq!(AudioProfile::BandwidthSaving.name(), "节省带宽");
    assert_eq!(AudioProfile::Standard.name(), "标准");
    assert_eq!(AudioProfile::HighQuality.name(), "高质量");
    assert_eq!(AudioProfile::Lossless.name(), "无损");
    assert_eq!(AudioProfile::HighResolution.name(), "高解析度");
    assert_eq!(AudioProfile::StudioMaster.name(), "录音室母带");
    assert_eq!(AudioProfile::Auto.name(), "自动挡");
    assert_eq!(AudioProfile::Custom.name(), "自定义");
}

#[test]
fn test_network_requirements() {
    let config = AudioConfig::for_profile(AudioProfile::HighQuality).unwrap();
    let mbps = config.network_requirement_mbps();
    // 256kbps * 1.2 = 0.3072 Mbps
    assert!(mbps > 0.2 && mbps < 0.4);
}

#[test]
fn test_all_profiles_list() {
    let all = AudioProfile::all_profiles();
    assert_eq!(all.len(), 8);
    assert_eq!(all[0], AudioProfile::BandwidthSaving);
    assert_eq!(all[7], AudioProfile::Custom);
}

#[test]
fn test_bandwidth_saving_config() {
    let config = AudioConfig::for_profile(AudioProfile::BandwidthSaving).unwrap();
    assert_eq!(config.sample_rate, 24_000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.bitrate, 32_000);
    assert_eq!(config.frame_size, 480);
    assert_eq!(config.complexity, 5);
}

#[test]
fn test_studio_master_config() {
    let config = AudioConfig::for_profile(AudioProfile::StudioMaster).unwrap();
    assert_eq!(config.sample_rate, 192_000);
    assert_eq!(config.channels, 2);
    assert_eq!(config.bitrate, 9_216_000);
    assert_eq!(config.frame_size, 7680);
    assert_eq!(config.complexity, 10);
}
