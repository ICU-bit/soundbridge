use audio_processor::{AudioProcessor, ProcessorConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let config = ProcessorConfig::default();
        let processor = AudioProcessor::new(config).unwrap();
        let _config = processor.config();
    }

    #[test]
    fn test_process_basic() {
        let mut processor = AudioProcessor::with_default_config().unwrap();
        let mut buffer = vec![0.5f32; 100];
        processor.process(&mut buffer).unwrap();
        for sample in &buffer {
            assert!(sample.abs() > 0.0, "Output should not be silent");
        }
    }

    #[test]
    fn test_silence_detection() {
        let processor = AudioProcessor::with_default_config().unwrap();
        let silence = vec![0.0f32; 100];
        assert!(processor.is_silence(&silence));

        let signal = vec![0.5f32; 100];
        assert!(!processor.is_silence(&signal));
    }

    #[test]
    fn test_rms_calculation() {
        let processor = AudioProcessor::with_default_config().unwrap();

        let silence = vec![0.0f32; 100];
        assert_eq!(processor.calculate_rms(&silence), 0.0);

        let constant = vec![0.5f32; 100];
        let rms = processor.calculate_rms(&constant);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_buffer() {
        let processor = AudioProcessor::with_default_config().unwrap();
        let empty: Vec<f32> = vec![];
        assert!(processor.is_silence(&empty));
        assert_eq!(processor.calculate_rms(&empty), 0.0);
    }
}
