//! 参数均衡器测试

use audio_processor::eq::*;

#[test]
fn test_biquad_filter_peaking() {
    use std::f32::consts::PI;

    let mut filter = BiquadFilter::peaking(48000.0, 1000.0, 6.0, 1.0);
    // Generate a 1kHz sine wave to exercise the peaking filter at its center frequency
    let input: Vec<f32> = (0..960)
        .map(|i| (2.0 * PI * 1000.0 * i as f32 / 48000.0).sin() * 0.5)
        .collect();
    let mut output = vec![0.0; 960];

    filter.process_buffer(&input, &mut output);

    // Compute RMS of input and output (skip first 100 samples for filter settling)
    let input_rms: f32 =
        input[100..].iter().map(|x| x * x).sum::<f32>().sqrt() / (input.len() - 100) as f32;
    let output_rms: f32 =
        output[100..].iter().map(|x| x * x).sum::<f32>().sqrt() / (output.len() - 100) as f32;

    // +6dB peaking at 1kHz should amplify the 1kHz sine wave
    assert!(
        output_rms > input_rms,
        "Expected output RMS ({}) > input RMS ({})",
        output_rms,
        input_rms
    );
}

#[test]
fn test_biquad_filter_reset() {
    let mut filter = BiquadFilter::peaking(48000.0, 1000.0, 6.0, 1.0);
    // Process some samples to build state
    for _ in 0..50 {
        filter.process(0.5);
    }

    filter.reset();

    // After reset, processing a zero sample should produce near-zero output
    let out = filter.process(0.0);
    assert!(
        out.abs() < 1e-6,
        "Expected near-zero after reset, got {}",
        out
    );
}

#[test]
fn test_parametric_eq_presets() {
    let mut eq = ParametricEq::new(48000);

    for preset in [
        EqPreset::Flat,
        EqPreset::Gaming,
        EqPreset::Music,
        EqPreset::Voice,
        EqPreset::Bass,
        EqPreset::Treble,
    ] {
        eq.set_preset(preset);
        eq.reset();

        let input = vec![0.5; 960];
        let mut output = vec![0.0; 960];
        eq.process(&input, &mut output);

        // Output should be valid (finite)
        for (i, &s) in output.iter().enumerate() {
            assert!(
                s.is_finite(),
                "Non-finite output at index {} for {:?}",
                i,
                preset
            );
        }
    }
}

#[test]
fn test_parametric_eq_gaming_changes_signal() {
    let mut eq = ParametricEq::new(48000);
    eq.set_preset(EqPreset::Gaming);

    let input = vec![0.5; 960];
    let mut output = vec![0.0; 960];

    eq.process(&input, &mut output);

    // With Gaming preset applied, output should differ from flat input
    assert!(
        output.iter().any(|&x| (x - 0.5).abs() > 1e-6),
        "Expected EQ to modify signal"
    );
}

#[test]
fn test_eq_disabled_passthrough() {
    let mut eq = ParametricEq::new(48000);
    eq.set_enabled(false);

    let input = vec![0.5; 100];
    let mut output = vec![0.0; 100];

    eq.process(&input, &mut output);

    assert_eq!(input, output, "Disabled EQ should pass through unchanged");
}

#[test]
fn test_eq_enabled_default() {
    let eq = ParametricEq::new(48000);
    assert!(eq.is_enabled(), "EQ should be enabled by default");
}

#[test]
fn test_eq_set_individual_band() {
    let mut eq = ParametricEq::new(48000);
    // Set a large boost on band 0 (31 Hz)
    eq.set_band(0, 12.0, 1.0);

    let input = vec![0.5; 960];
    let mut output = vec![0.0; 960];
    eq.process(&input, &mut output);

    // Should produce valid output
    for &s in &output {
        assert!(s.is_finite());
    }
}

#[test]
fn test_eq_reset() {
    let mut eq = ParametricEq::new(48000);
    eq.set_preset(EqPreset::Bass);

    let input = vec![0.5; 100];
    let mut output = vec![0.0; 100];
    eq.process(&input, &mut output);

    eq.reset();

    // After reset, processing should still work correctly
    let mut output2 = vec![0.0; 100];
    eq.process(&input, &mut output2);
    for &s in &output2 {
        assert!(s.is_finite());
    }
}

#[test]
fn test_flat_preset_identity() {
    let mut eq = ParametricEq::new(48000);
    eq.set_preset(EqPreset::Flat);

    let input = vec![0.5; 960];
    let mut output = vec![0.0; 960];
    eq.process(&input, &mut output);

    // Flat preset should be very close to identity (0 dB gain)
    for (i, (&inp, &out)) in input.iter().zip(output.iter()).enumerate() {
        assert!(
            (inp - out).abs() < 0.01,
            "Flat preset should be near-identity at index {}: in={}, out={}",
            i,
            inp,
            out
        );
    }
}
