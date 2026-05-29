pub mod codec;

pub use codec::*;

pub mod codec {
    use audio_core::{AudioBuffer, Result};

    pub struct AudioCodec {
        _private: (),
    }

    impl AudioCodec {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn encode(&mut self, _buffer: AudioBuffer) -> Result<Vec<u8>> {
            unimplemented!()
        }

        pub fn decode(&mut self, _data: &[u8]) -> Result<AudioBuffer> {
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
