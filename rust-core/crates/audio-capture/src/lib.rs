pub mod capture;

pub use capture::*;

pub mod capture {
    use audio_core::{AudioBuffer, AudioFormat, Result};

    pub struct AudioCapture {
        _private: (),
    }

    impl AudioCapture {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn start(&mut self) -> Result<()> {
            Ok(())
        }

        pub fn stop(&mut self) -> Result<()> {
            Ok(())
        }

        pub fn read(&mut self) -> Result<AudioBuffer> {
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
