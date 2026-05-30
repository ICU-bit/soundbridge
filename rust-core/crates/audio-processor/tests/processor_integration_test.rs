//! Integration tests for audio-processor crate.
//!
//! Tests all public API types: ProcessorConfig, AudioProcessor,
//! GainProcessor, SilenceDetector, NoiseGate, AecProcessor, NsProcessor, AgcProcessor.

use audio_processor::aec::AecConfig;
use audio_processor::agc::AgcConfig;
use audio_processor::ns::NsConfig;
use audio_processor::*;

// ============================================================================
// ProcessorConfig tests
// ============================================================================

#[test]
fn test_processor_config_default() {
    let config = ProcessorConfig::default();
    assert_eq!(config.gain_db, 0.0);
    assert_eq!(config.silence_threshold_db, -60.0);
    assert_eq!(config.noise_gate_threshold_db, -50.0);
    assert_eq!(config.aec_tail_ms, 50);
    assert_eq!(config.ns_suppression_db, 12.0);
    assert_eq!(config.agc_target_dbfs, -3.0);
    assert_eq!(config.agc_max_gain_db, 30.0);
}

#[test]
fn test_processor_config_clone() {
    let config = ProcessorConfig::default();
    let cloned = config.clone();
    assert_eq!(config.gain_db, cloned.gain_db);
    assert_eq!(config.aec_tail_ms, cloned.aec_tail_ms);
}

#[test]
fn test_processor_config_debug() {
    let config = ProcessorConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("ProcessorConfig"));
}

// ============================================================================
// AudioProcessor tests
// ============================================================================

#[test]
fn test_audio_processor_new() {
    let config = ProcessorConfig::default();
    let processor = AudioProcessor::new(config);
    assert!(processor.is_ok());
}

#[test]
fn test_audio_processor_with_default_config() {
    let processor = AudioProcessor::with_default_config();
    assert!(processor.is_ok());
    let p = processor.unwrap();
    assert_eq!(p.config().gain_db, 0.0);
}

#[test]
fn test_audio_processor_process() {
    let mut processor = AudioProcessor::with_default_config().unwrap();
    let mut buffer = vec![0.5f32; 960];
    let result = processor.process(&mut buffer);
    assert!(result.is_ok());
    // Output should be finite
    for sample in &buffer {
        assert!(sample.is_finite());
    }
}

#[test]
fn test_audio_processor_process_with_aec() {
    let mut processor = AudioProcessor::with_default_config().unwrap();
    let mut buffer = vec![0.5f32; 960];
    let reference = vec![0.3f32; 960];
    let result = processor.process_with_aec(&mut buffer, &reference);
    assert!(result.is_ok());
    for sample in &buffer {
        assert!(sample.is_finite());
    }
}

#[test]
fn test_audio_processor_is_silence() {
    let processor = AudioProcessor::with_default_config().unwrap();
    let silence = vec![0.0f32; 100];
    assert!(processor.is_silence(&silence));

    let signal = vec![0.5f32; 100];
    assert!(!processor.is_silence(&signal));
}

#[test]
fn test_audio_processor_calculate_rms() {
    let processor = AudioProcessor::with_default_config().unwrap();
    let silence = vec![0.0f32; 100];
    assert_eq!(processor.calculate_rms(&silence), 0.0);

    let constant = vec![0.5f32; 100];
    let rms = processor.calculate_rms(&constant);
    assert!((rms - 0.5).abs() < 0.001);
}

#[test]
fn test_audio_processor_empty_buffer() {
    let processor = AudioProcessor::with_default_config().unwrap();
    let empty: Vec<f32> = vec![];
    assert!(processor.is_silence(&empty));
    assert_eq!(processor.calculate_rms(&empty), 0.0);
}

#[test]
fn test_audio_processor_960_frame() {
    let mut processor = AudioProcessor::with_default_config().unwrap();
    // 960 samples = 20ms @ 48kHz (standard frame)
    let mut buffer = vec![0.1f32; 960];
    assert!(processor.process(&mut buffer).is_ok());
}

// ============================================================================
// GainProcessor tests
// ============================================================================

#[test]
fn test_gain_processor_zero_db() {
    let gain = GainProcessor::new(0.0).unwrap();
    let mut buffer = vec![0.5f32; 100];
    gain.process(&mut buffer).unwrap();
    for sample in &buffer {
        assert!((sample - 0.5).abs() < 0.001);
    }
}

#[test]
fn test_gain_processor_plus_6db() {
    let gain = GainProcessor::new(6.0).unwrap();
    let mut buffer = vec![0.5f32; 100];
    gain.process(&mut buffer).unwrap();
    for sample in &buffer {
        assert!((sample - 1.0).abs() < 0.01);
    }
}

#[test]
fn test_gain_processor_minus_6db() {
    let gain = GainProcessor::new(-6.0).unwrap();
    let mut buffer = vec![1.0f32; 100];
    gain.process(&mut buffer).unwrap();
    for sample in &buffer {
        assert!((sample - 0.5).abs() < 0.01);
    }
}

#[test]
fn test_gain_processor_factor() {
    let gain = GainProcessor::new(0.0).unwrap();
    assert!((gain.gain_factor() - 1.0).abs() < 0.001);

    let gain_6db = GainProcessor::new(6.0).unwrap();
    assert!((gain_6db.gain_factor() - 2.0).abs() < 0.01);
}

// ============================================================================
// SilenceDetector tests
// ============================================================================

#[test]
fn test_silence_detector_creation() {
    let detector = SilenceDetector::new(-60.0).unwrap();
    assert!(detector.is_silence(&vec![0.0f32; 100]));
}

#[test]
fn test_silence_detector_threshold() {
    let detector = SilenceDetector::new(-60.0).unwrap();
    // Very quiet signal should be silence
    assert!(detector.is_silence(&vec![0.0001f32; 100]));
    // Loud signal should not be silence
    assert!(!detector.is_silence(&vec![0.5f32; 100]));
}

#[test]
fn test_silence_detector_rms() {
    let detector = SilenceDetector::new(-60.0).unwrap();
    assert_eq!(detector.calculate_rms(&vec![0.0f32; 100]), 0.0);
    let rms = detector.calculate_rms(&vec![0.5f32; 100]);
    assert!((rms - 0.5).abs() < 0.001);
}

#[test]
fn test_silence_detector_empty() {
    let detector = SilenceDetector::new(-60.0).unwrap();
    assert!(detector.is_silence(&vec![]));
    assert_eq!(detector.calculate_rms(&vec![]), 0.0);
}

// ============================================================================
// NoiseGate tests
// ============================================================================

#[test]
fn test_noise_gate_creation() {
    let gate = NoiseGate::new(-40.0).unwrap();
    // Threshold should be converted from dB
    assert!(gate.threshold() > 0.0);
    assert!(gate.threshold() < 1.0);
}

#[test]
fn test_noise_gate_below_threshold() {
    let gate = NoiseGate::new(-40.0).unwrap();
    // Signal below threshold should be zeroed
    let mut buffer = vec![0.005f32; 100]; // ~-46 dB
    gate.process(&mut buffer).unwrap();
    for sample in &buffer {
        assert!(sample.abs() < 0.001);
    }
}

#[test]
fn test_noise_gate_above_threshold() {
    let gate = NoiseGate::new(-40.0).unwrap();
    // Signal above threshold should be preserved
    let mut buffer = vec![0.1f32; 100]; // ~-20 dB
    gate.process(&mut buffer).unwrap();
    for sample in &buffer {
        assert!(sample.abs() > 0.01);
    }
}

// ============================================================================
// AecProcessor tests
// ============================================================================

#[test]
fn test_aec_config_default() {
    let config = AecConfig::default();
    assert_eq!(config.filter_length, 4800);
    assert_eq!(config.step_size, 0.1);
    assert_eq!(config.regularization, 0.1);
}

#[test]
fn test_aec_creation() {
    let aec = AecProcessor::with_default_config();
    assert_eq!(aec.config().filter_length, 4800);
}

#[test]
fn test_aec_process_basic() {
    let mut aec = AecProcessor::with_default_config();
    let speaker = vec![1.0f32; 100];
    let mut mic: Vec<f32> = speaker.iter().map(|&s| s * 0.5).collect();
    aec.process(&mut mic, &speaker).unwrap();
    for sample in &mic {
        assert!(sample.is_finite());
    }
}

#[test]
fn test_aec_no_reference() {
    let mut aec = AecProcessor::with_default_config();
    let mut mic = vec![0.5f32; 100];
    let reference = vec![0.0f32; 100];
    aec.process(&mut mic, &reference).unwrap();
    for sample in &mic {
        assert!((sample - 0.5).abs() < 0.1);
    }
}

#[test]
fn test_aec_buffer_mismatch() {
    let mut aec = AecProcessor::with_default_config();
    let mut buffer = vec![0.5f32; 100];
    let reference = vec![0.3f32; 50];
    assert!(aec.process(&mut buffer, &reference).is_err());
}

#[test]
fn test_aec_reset() {
    let mut aec = AecProcessor::with_default_config();
    let mut mic = vec![0.5f32; 100];
    let reference = vec![0.3f32; 100];
    aec.process(&mut mic, &reference).unwrap();
    aec.reset();
    // After reset, weights should be zero (internal state reset)
}

#[test]
fn test_aec_custom_config() {
    let aec = AecProcessor::new(AecConfig {
        filter_length: 100,
        step_size: 0.5,
        regularization: 1e-6,
    });
    assert_eq!(aec.config().filter_length, 100);
    assert_eq!(aec.config().step_size, 0.5);
}

// ============================================================================
// NsProcessor tests
// ============================================================================

#[test]
fn test_ns_config_default() {
    let config = NsConfig::default();
    assert_eq!(config.suppression_db, 12.0);
    assert_eq!(config.window_size, 480);
    assert_eq!(config.noise_update_factor, 0.98);
    assert_eq!(config.min_gain, 0.01);
}

#[test]
fn test_ns_creation() {
    let ns = NsProcessor::with_default_config();
    assert_eq!(ns.config().suppression_db, 12.0);
}

#[test]
fn test_ns_process_silence() {
    let mut ns = NsProcessor::with_default_config();
    let mut buffer = vec![0.001f32; 100];
    ns.process(&mut buffer).unwrap();
    let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms < 0.001);
}

#[test]
fn test_ns_process_signal() {
    let mut ns = NsProcessor::with_default_config();
    let mut buffer = vec![0.5f32; 100];
    ns.process(&mut buffer).unwrap();
    let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.1);
}

#[test]
fn test_ns_reset() {
    let mut ns = NsProcessor::with_default_config();
    let mut buffer = vec![0.5f32; 100];
    ns.process(&mut buffer).unwrap();
    ns.reset();
    // After reset, noise state should be cleared
}

#[test]
fn test_ns_empty_buffer() {
    let mut ns = NsProcessor::with_default_config();
    let mut buffer: Vec<f32> = vec![];
    assert!(ns.process(&mut buffer).is_ok());
}

#[test]
fn test_ns_custom_config() {
    let ns = NsProcessor::new(NsConfig {
        suppression_db: 20.0,
        window_size: 960,
        noise_update_factor: 0.95,
        min_gain: 0.001,
    });
    assert_eq!(ns.config().suppression_db, 20.0);
    assert_eq!(ns.config().window_size, 960);
}

// ============================================================================
// AgcProcessor tests
// ============================================================================

#[test]
fn test_agc_config_default() {
    let config = AgcConfig::default();
    assert_eq!(config.target_dbfs, -3.0);
    assert_eq!(config.max_gain_db, 30.0);
    assert_eq!(config.attack_ms, 10.0);
    assert_eq!(config.release_ms, 100.0);
    assert_eq!(config.sample_rate, 48000);
}

#[test]
fn test_agc_creation() {
    let agc = AgcProcessor::with_default_config();
    assert_eq!(agc.config().target_dbfs, -3.0);
    assert_eq!(agc.config().max_gain_db, 30.0);
}

#[test]
fn test_agc_process_quiet() {
    let mut agc = AgcProcessor::with_default_config();
    let mut buffer = vec![0.01f32; 1000];
    agc.process(&mut buffer).unwrap();
    let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01);
}

#[test]
fn test_agc_process_loud() {
    let mut agc = AgcProcessor::with_default_config();
    let mut buffer = vec![0.9f32; 1000];
    agc.process(&mut buffer).unwrap();
    let rms: f32 = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms < 0.9);
}

#[test]
fn test_agc_gain_db() {
    let agc = AgcProcessor::with_default_config();
    // Initial gain should be 0 dB (1.0 linear)
    assert!((agc.current_gain_db() - 0.0).abs() < 0.01);
}

#[test]
fn test_agc_reset() {
    let mut agc = AgcProcessor::with_default_config();
    let mut buffer = vec![0.01f32; 100];
    agc.process(&mut buffer).unwrap();
    agc.reset();
    assert!((agc.current_gain_db() - 0.0).abs() < 0.01);
}

#[test]
fn test_agc_empty_buffer() {
    let mut agc = AgcProcessor::with_default_config();
    let mut buffer: Vec<f32> = vec![];
    assert!(agc.process(&mut buffer).is_ok());
}

#[test]
fn test_agc_custom_config() {
    let agc = AgcProcessor::new(AgcConfig {
        target_dbfs: -6.0,
        max_gain_db: 20.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        sample_rate: 44100,
    });
    assert_eq!(agc.config().target_dbfs, -6.0);
    assert_eq!(agc.config().max_gain_db, 20.0);
    assert_eq!(agc.config().sample_rate, 44100);
}

// ============================================================================
// ProcessorError tests
// ============================================================================

#[test]
fn test_processor_error_display() {
    let err = ProcessorError::ProcessingFailed("test".to_string());
    assert!(err.to_string().contains("test"));

    let err = ProcessorError::ConfigError("bad config".to_string());
    assert!(err.to_string().contains("bad config"));

    let err = ProcessorError::BufferError("buffer err".to_string());
    assert!(err.to_string().contains("buffer err"));
}

#[test]
fn test_processor_error_debug() {
    let err = ProcessorError::ProcessingFailed("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("ProcessingFailed"));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_process_single_sample() {
    let mut processor = AudioProcessor::with_default_config().unwrap();
    let mut buffer = vec![0.5f32; 1];
    assert!(processor.process(&mut buffer).is_ok());
    assert!(buffer[0].is_finite());
}

#[test]
fn test_process_960_samples() {
    let mut processor = AudioProcessor::with_default_config().unwrap();
    let mut buffer = vec![0.1f32; 960];
    assert!(processor.process(&mut buffer).is_ok());
}

#[test]
fn test_rms_different_levels() {
    let processor = AudioProcessor::with_default_config().unwrap();
    let rms_0 = processor.calculate_rms(&vec![0.0f32; 100]);
    let rms_half = processor.calculate_rms(&vec![0.5f32; 100]);
    let rms_one = processor.calculate_rms(&vec![1.0f32; 100]);
    assert!(rms_0 < rms_half);
    assert!(rms_half < rms_one);
}

#[test]
fn test_gain_negative_12db() {
    let gain = GainProcessor::new(-12.0).unwrap();
    let mut buffer = vec![1.0f32; 100];
    gain.process(&mut buffer).unwrap();
    // -12dB ≈ 0.25
    for sample in &buffer {
        assert!((sample - 0.25).abs() < 0.01);
    }
}
