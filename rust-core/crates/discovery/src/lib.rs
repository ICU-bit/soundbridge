pub mod discovery;

pub use discovery::*;

pub mod discovery {
    use audio_core::Result;

    pub struct DeviceDiscovery {
        _private: (),
    }

    impl DeviceDiscovery {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub async fn start(&mut self) -> Result<()> {
            unimplemented!()
        }

        pub async fn stop(&mut self) -> Result<()> {
            unimplemented!()
        }

        pub async fn discover(&mut self) -> Result<Vec<String>> {
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
