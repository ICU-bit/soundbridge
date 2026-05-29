pub mod packet;

pub use packet::*;

pub mod packet {
    use audio_core::Result;

    pub enum Packet {
        Audio,
        Control,
    }

    pub struct Protocol {
        _private: (),
    }

    impl Protocol {
        pub fn new() -> Result<Self> {
            Ok(Self { _private: () })
        }

        pub fn serialize(&self, _packet: Packet) -> Result<Vec<u8>> {
            unimplemented!()
        }

        pub fn deserialize(&self, _data: &[u8]) -> Result<Packet> {
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
