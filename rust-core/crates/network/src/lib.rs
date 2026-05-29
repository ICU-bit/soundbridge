pub mod transport;

pub use transport::*;

pub mod transport {
    use audio_core::Result;

    pub struct NetworkTransport {
        _private: (),
    }

    impl NetworkTransport {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub async fn connect(&mut self, _addr: &str) -> Result<()> {
            unimplemented!()
        }

        pub async fn send(&mut self, _data: &[u8]) -> Result<()> {
            unimplemented!()
        }

        pub async fn receive(&mut self) -> Result<Vec<u8>> {
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
