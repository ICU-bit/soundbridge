use protocol::{Packet, PacketHeader, Protocol};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header_encode_decode() {
        let header = PacketHeader {
            sequence: 42,
            timestamp_ms: 1234567890,
            flags: 0x01,
            channels: 2,
            opus_length: 960,
        };

        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();

        assert_eq!(buf.len(), PacketHeader::SIZE);

        let (decoded, size) = PacketHeader::decode(&buf).unwrap();
        assert_eq!(size, PacketHeader::SIZE);
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.timestamp_ms, 1234567890);
        assert_eq!(decoded.flags, 0x01);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.opus_length, 960);
    }

    #[test]
    fn test_serialize_deserialize_audio() {
        let protocol = Protocol::new();

        let header = PacketHeader {
            sequence: 1,
            timestamp_ms: 1000,
            flags: 0x01,
            channels: 2,
            opus_length: 4,
        };

        let packet = Packet::Audio {
            header,
            data: vec![0x01, 0x02, 0x03, 0x04],
        };

        let serialized = protocol.serialize(&packet).unwrap();
        let deserialized = protocol.deserialize(&serialized).unwrap();

        match deserialized {
            Packet::Audio { header, data } => {
                assert_eq!(header.sequence, 1);
                assert_eq!(header.timestamp_ms, 1000);
                assert_eq!(header.flags, 0x01);
                assert_eq!(data, vec![0x01, 0x02, 0x03, 0x04]);
            }
            _ => panic!("Expected Audio packet"),
        }
    }

    #[test]
    fn test_invalid_data_too_short() {
        let buf = vec![0u8; 5];
        assert!(PacketHeader::decode(&buf).is_err());
    }
}
