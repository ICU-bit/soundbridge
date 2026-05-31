//! Integration tests for audio-mixer crate.
//!
//! Tests all public API types: MixerConfig, AudioMixer, MixerError,
//! including mix, mix_two, mix_two_into, soft_clip, and edge cases.

use audio_mixer::*;

// ============================================================================
// MixerConfig tests
// ============================================================================

#[test]
fn test_mixer_config_default() {
    let config = MixerConfig::default();
    assert_eq!(config.sample_rate, 48000);
    assert_eq!(config.channels, 1);
    assert!(config.clipping_protection);
}

#[test]
fn test_mixer_config_custom() {
    let config = MixerConfig {
        sample_rate: 44100,
        channels: 1,
        clipping_protection: false,
    };
    assert_eq!(config.sample_rate, 44100);
    assert_eq!(config.channels, 1);
    assert!(!config.clipping_protection);
}

#[test]
fn test_mixer_config_clone() {
    let config = MixerConfig::default();
    let cloned = config.clone();
    assert_eq!(config.sample_rate, cloned.sample_rate);
    assert_eq!(config.channels, cloned.channels);
    assert_eq!(config.clipping_protection, cloned.clipping_protection);
}

#[test]
fn test_mixer_config_debug() {
    let config = MixerConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("MixerConfig"));
}

// ============================================================================
// AudioMixer creation tests
// ============================================================================

#[test]
fn test_mixer_new() {
    let config = MixerConfig::default();
    let mixer = AudioMixer::new(config);
    assert_eq!(mixer.config().sample_rate, 48000);
}

#[test]
fn test_mixer_default() {
    let mixer = AudioMixer::default();
    assert!(mixer.config().clipping_protection);
}

#[test]
fn test_mixer_clone() {
    let mixer = AudioMixer::default();
    let cloned = mixer.clone();
    assert_eq!(mixer.config().sample_rate, cloned.config().sample_rate);
}

// ============================================================================
// mix() tests
// ============================================================================

#[test]
fn test_mix_single_input() {
    let mixer = AudioMixer::default();
    let input = vec![0.5f32; 100];
    let result = mixer.mix(&[&input], &[1.0]).unwrap();
    assert_eq!(result.len(), 100);
    // tanh(0.5) ≈ 0.4621
    let expected = 0.5f32.tanh();
    for out in result.iter() {
        assert!((out - expected).abs() < 0.001);
    }
}

#[test]
fn test_mix_two_inputs_equal_volume() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.5f32; 100];
    let result = mixer.mix_two(&input1, 0.5, &input2, 0.5).unwrap();
    assert_eq!(result.len(), 100);
    let expected = 0.5f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.001);
    }
}

#[test]
fn test_mix_silent_input() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.0f32; 100];
    let input2 = vec![0.5f32; 100];
    let result = mixer.mix_two(&input1, 1.0, &input2, 0.0).unwrap();
    for sample in result.iter() {
        assert!(sample.abs() < 0.001);
    }
}

#[test]
fn test_mix_clipping_protection() {
    let mixer = AudioMixer::default();
    let input1 = vec![1.0f32; 100];
    let input2 = vec![1.0f32; 100];
    let result = mixer.mix_two(&input1, 1.0, input2.as_slice(), 1.0).unwrap();
    // tanh(2.0) ≈ 0.964, should be in (-1, 1)
    for sample in result.iter() {
        assert!(*sample < 1.0 && *sample > -1.0);
        assert!(*sample > 0.9);
    }
}

#[test]
fn test_mix_no_clipping_protection() {
    let config = MixerConfig {
        clipping_protection: false,
        ..Default::default()
    };
    let mixer = AudioMixer::new(config);
    let input1 = vec![1.0f32; 100];
    let input2 = vec![1.0f32; 100];
    let result = mixer.mix_two(&input1, 1.0, input2.as_slice(), 1.0).unwrap();
    for sample in result.iter() {
        assert!((sample - 2.0).abs() < 0.001);
    }
}

#[test]
fn test_mix_three_inputs() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.3f32; 100];
    let input2 = vec![0.3f32; 100];
    let input3 = vec![0.3f32; 100];
    let result = mixer
        .mix(&[&input1, &input2, &input3], &[1.0, 1.0, 1.0])
        .unwrap();
    assert_eq!(result.len(), 100);
    // 0.3 * 3 = 0.9, tanh(0.9) ≈ 0.716
    let expected = 0.9f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.01);
    }
}

#[test]
fn test_mix_volume_zero() {
    let mixer = AudioMixer::default();
    let input = vec![0.5f32; 100];
    let result = mixer.mix(&[&input], &[0.0]).unwrap();
    for sample in result.iter() {
        assert!(sample.abs() < 0.001);
    }
}

#[test]
fn test_mix_volume_half() {
    let mixer = AudioMixer::default();
    let input = vec![1.0f32; 100];
    let result = mixer.mix(&[&input], &[0.5]).unwrap();
    // 1.0 * 0.5 = 0.5, tanh(0.5) ≈ 0.462
    let expected = 0.5f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.01);
    }
}

// ============================================================================
// mix_two() tests
// ============================================================================

#[test]
fn test_mix_two_different_volumes() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.8f32; 100];
    let input2 = vec![0.2f32; 100];
    let result = mixer.mix_two(&input1, 0.5, &input2, 0.5).unwrap();
    // 0.8*0.5 + 0.2*0.5 = 0.5, tanh(0.5) ≈ 0.462
    let expected = 0.5f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.01);
    }
}

#[test]
fn test_mix_two_one_silent() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.0f32; 100];
    let result = mixer.mix_two(&input1, 1.0, &input2, 1.0).unwrap();
    // 0.5*1.0 + 0.0*1.0 = 0.5, tanh(0.5) ≈ 0.462
    let expected = 0.5f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.01);
    }
}

// ============================================================================
// mix_two_into() tests
// ============================================================================

#[test]
fn test_mix_two_into_basic() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.5f32; 100];
    let mut output = vec![0.0f32; 100];
    mixer
        .mix_two_into(&input1, 0.5, &input2, 0.5, &mut output)
        .unwrap();
    let expected = 0.5f32.tanh();
    for sample in output.iter() {
        assert!((sample - expected).abs() < 0.001);
    }
}

#[test]
fn test_mix_two_into_length_mismatch_inputs() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.5f32; 50];
    let mut output = vec![0.0f32; 100];
    let result = mixer.mix_two_into(&input1, 1.0, &input2, 1.0, &mut output);
    assert!(result.is_err());
}

#[test]
fn test_mix_two_into_output_too_small() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.5f32; 100];
    let mut output = vec![0.0f32; 50];
    let result = mixer.mix_two_into(&input1, 1.0, &input2, 1.0, &mut output);
    assert!(result.is_err());
}

// ============================================================================
// MixerError tests
// ============================================================================

#[test]
fn test_mixer_error_display() {
    let err = MixerError::FormatMismatch;
    assert!(err.to_string().contains("格式不匹配"));

    let err = MixerError::EmptyBuffers;
    assert!(err.to_string().contains("为空"));

    let err = MixerError::LengthMismatch {
        expected: 100,
        actual: 50,
    };
    assert!(err.to_string().contains("100"));
    assert!(err.to_string().contains("50"));

    let err = MixerError::VolumeCountMismatch {
        expected: 2,
        actual: 1,
    };
    assert!(err.to_string().contains("2"));
    assert!(err.to_string().contains("1"));
}

#[test]
fn test_mixer_error_debug() {
    let err = MixerError::EmptyBuffers;
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("EmptyBuffers"));
}

#[test]
fn test_mix_empty_inputs_error() {
    let mixer = AudioMixer::default();
    let result = mixer.mix(&[], &[]);
    assert!(result.is_err());
}

#[test]
fn test_mix_volume_mismatch_error() {
    let mixer = AudioMixer::default();
    let input = vec![0.5f32; 100];
    let result = mixer.mix(&[&input], &[1.0, 0.5]);
    assert!(result.is_err());
}

#[test]
fn test_mix_length_mismatch_error() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 100];
    let input2 = vec![0.5f32; 50];
    let result = mixer.mix_two(&input1, 1.0, &input2, 1.0);
    assert!(result.is_err());
}

// ============================================================================
// Soft clip (tanh) edge cases
// ============================================================================

#[test]
fn test_soft_clip_large_values() {
    let mixer = AudioMixer::default();
    let input1 = vec![10.0f32; 100];
    let input2 = vec![0.0f32; 100];
    let result = mixer.mix_two(&input1, 1.0, &input2, 1.0).unwrap();
    // tanh(10.0) ≈ 1.0 in f32, never exceeds 1.0
    for sample in result.iter() {
        assert!(*sample <= 1.0 && *sample >= -1.0);
        assert!(*sample > 0.99);
    }
}

#[test]
fn test_soft_clip_negative_values() {
    let mixer = AudioMixer::default();
    let input1 = vec![-1.0f32; 100];
    let input2 = vec![-1.0f32; 100];
    let result = mixer.mix_two(&input1, 1.0, input2.as_slice(), 1.0).unwrap();
    // tanh(-2.0) ≈ -0.964
    for sample in result.iter() {
        assert!(*sample > -1.0 && *sample < -0.9);
    }
}

#[test]
fn test_soft_clip_asymmetric() {
    let mixer = AudioMixer::default();
    // Positive large value
    let pos = vec![5.0f32; 10];
    let zero = vec![0.0f32; 10];
    let result_pos = mixer.mix_two(&pos, 1.0, &zero, 1.0).unwrap();
    // Negative large value
    let neg = vec![-5.0f32; 10];
    let result_neg = mixer.mix_two(&neg, 1.0, &zero, 1.0).unwrap();
    // tanh is symmetric: tanh(x) = -tanh(-x)
    for (p, n) in result_pos.iter().zip(result_neg.iter()) {
        assert!((p + n).abs() < 0.001);
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_mix_single_sample() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.5f32; 1];
    let input2 = vec![0.3f32; 1];
    let result = mixer.mix_two(&input1, 1.0, &input2, 1.0).unwrap();
    assert_eq!(result.len(), 1);
    // 0.5 + 0.3 = 0.8, tanh(0.8) ≈ 0.664
    let expected = 0.8f32.tanh();
    assert!((result[0] - expected).abs() < 0.01);
}

#[test]
fn test_mix_960_samples() {
    let mixer = AudioMixer::default();
    let input1 = vec![0.1f32; 960];
    let input2 = vec![0.2f32; 960];
    let result = mixer.mix_two(&input1, 1.0, &input2, 1.0).unwrap();
    assert_eq!(result.len(), 960);
}

#[test]
fn test_mix_many_inputs() {
    let mixer = AudioMixer::default();
    let inputs: Vec<Vec<f32>> = (0..10).map(|_| vec![0.1f32; 100]).collect();
    let refs: Vec<&[f32]> = inputs.iter().map(|v| v.as_slice()).collect();
    let volumes = vec![0.5f32; 10];
    let result = mixer.mix(&refs, &volumes).unwrap();
    assert_eq!(result.len(), 100);
    // 10 * 0.1 * 0.5 = 0.5, tanh(0.5) ≈ 0.462
    let expected = 0.5f32.tanh();
    for sample in result.iter() {
        assert!((sample - expected).abs() < 0.01);
    }
}

#[test]
fn test_mix_alternating_positive_negative() {
    let mixer = AudioMixer::default();
    let input: Vec<f32> = (0..100)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect();
    let result = mixer.mix(&[&input], &[1.0]).unwrap();
    // Should average to near 0
    let sum: f32 = result.iter().sum();
    assert!(sum.abs() < 1.0);
}

#[test]
fn test_mixer_config_44100_mono() {
    let config = MixerConfig {
        sample_rate: 44100,
        channels: 1,
        clipping_protection: true,
    };
    let mixer = AudioMixer::new(config);
    assert_eq!(mixer.config().sample_rate, 44100);
    assert_eq!(mixer.config().channels, 1);
}
