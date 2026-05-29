pub mod playback;

pub use playback::*;

pub mod playback {
    use audio_core::{AudioBuffer, Result};

    pub struct AudioPlayback {
        _private: (),
    }

    impl AudioPlayback {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn start(&mut self) -> Result<()> {
            Ok(())
        }

        pub fn stop(&mut self) -> Result<()> {
            Ok(())
        }

        pub fn write(&mut self, _buffer: AudioBuffer) -> Result<()> {
            Ok(())
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
