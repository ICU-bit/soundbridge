pub mod mixer;

pub use mixer::*;

pub mod mixer {
    use audio_core::{AudioBuffer, Result};

    pub struct AudioMixer {
        _private: (),
    }

    impl AudioMixer {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn mix(&mut self, _buffers: &[AudioBuffer]) -> Result<AudioBuffer> {
            unimplemented!()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn it_works() {
            let result = 2 + 2;
            assert_eq!(result, 4);
        }
    }
}
