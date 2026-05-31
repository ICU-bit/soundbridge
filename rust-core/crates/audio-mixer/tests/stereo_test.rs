use audio_mixer::stereo::*;

#[test]
fn test_mono_to_stereo() {
    let mono = vec![0.1, 0.2, 0.3, 0.4];
    let mut stereo = vec![0.0; 8];

    mono_to_stereo(&mono, &mut stereo).unwrap();

    assert_eq!(stereo, [0.1, 0.1, 0.2, 0.2, 0.3, 0.3, 0.4, 0.4]);
}

#[test]
fn test_mono_to_stereo_buffer_mismatch() {
    let mono = vec![0.1, 0.2];
    let mut stereo = vec![0.0; 3]; // 应该是 4

    let result = mono_to_stereo(&mono, &mut stereo);
    assert!(result.is_err());
}

#[test]
fn test_stereo_to_mono() {
    let stereo = vec![0.1, 0.3, 0.2, 0.4];
    let mut mono = vec![0.0; 2];

    stereo_to_mono(&stereo, &mut mono).unwrap();

    assert!((mono[0] - 0.2).abs() < 0.001);
    assert!((mono[1] - 0.3).abs() < 0.001);
}

#[test]
fn test_stereo_to_mono_buffer_mismatch() {
    let stereo = vec![0.1, 0.3, 0.2, 0.4];
    let mut mono = vec![0.0; 3]; // 应该是 2

    let result = stereo_to_mono(&stereo, &mut mono);
    assert!(result.is_err());
}

#[test]
fn test_stereo_mixer_mix_mono_passthrough() {
    let mixer = StereoMixer::new(1);
    let input = vec![0.5, 0.3];
    let mut output = vec![0.0; 2];

    mixer.mix(&input, &mut output).unwrap();

    assert_eq!(output, [0.5, 0.3]);
}

#[test]
fn test_stereo_mixer_mix_to_stereo() {
    let mixer = StereoMixer::new(2);
    let mono = vec![0.5, 0.5];
    let mut stereo = vec![0.0; 4];

    mixer.mix(&mono, &mut stereo).unwrap();

    assert_eq!(stereo, [0.5, 0.5, 0.5, 0.5]);
}

#[test]
fn test_stereo_mixer_downmix_mono_passthrough() {
    let mixer = StereoMixer::new(1);
    let input = vec![0.5, 0.3];
    let mut output = vec![0.0; 2];

    mixer.downmix(&input, &mut output).unwrap();

    assert_eq!(output, [0.5, 0.3]);
}

#[test]
fn test_stereo_mixer_downmix_to_mono() {
    let mixer = StereoMixer::new(2);
    let stereo = vec![0.2, 0.4, 0.6, 0.8];
    let mut mono = vec![0.0; 2];

    mixer.downmix(&stereo, &mut mono).unwrap();

    assert!((mono[0] - 0.3).abs() < 0.001);
    assert!((mono[1] - 0.7).abs() < 0.001);
}

#[test]
fn test_stereo_mixer_set_channels() {
    let mut mixer = StereoMixer::new(1);
    assert_eq!(mixer.channels(), 1);

    mixer.set_channels(2);
    assert_eq!(mixer.channels(), 2);
}

#[test]
fn test_roundtrip_mono_stereo_mono() {
    let original = vec![0.1, 0.2, 0.3, 0.4];
    let mut stereo = vec![0.0; 8];
    let mut recovered = vec![0.0; 4];

    mono_to_stereo(&original, &mut stereo).unwrap();
    stereo_to_mono(&stereo, &mut recovered).unwrap();

    for (a, b) in original.iter().zip(recovered.iter()) {
        assert!((a - b).abs() < 0.001, "roundtrip mismatch: {a} vs {b}");
    }
}
