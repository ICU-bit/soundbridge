use audio_mixer::{AudioMixer, MixerConfig};

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
    }

    #[test]
    fn test_mix_two_inputs() {
        let mixer = AudioMixer::default();
        let input1 = vec![0.5f32; 100];
        let input2 = vec![0.3f32; 100];
        let result = mixer.mix_two(&input1, 0.7, &input2, 0.5).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_mix_empty_inputs() {
        let mixer = AudioMixer::default();
        let result = mixer.mix(&[], &[]);
        assert!(result.is_err());
    }
}
