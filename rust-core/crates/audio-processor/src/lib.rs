pub mod processor;

pub use processor::*;

pub mod processor {
    use audio_core::{AudioBuffer, Result};

    pub struct AudioProcessor {
        _private: (),
    }

    impl AudioProcessor {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn process(&mut self, _buffer: AudioBuffer) -> Result<AudioBuffer> {
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
