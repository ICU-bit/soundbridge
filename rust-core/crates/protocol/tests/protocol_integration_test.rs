//! Integration tests for protocol crate.
//!
//! Tests the public API from an external perspective, verifying
//! cross-component interactions and real-world usage patterns.

use protocol::*;

// ============================================================================
// PacketType tests
// ============================================================================

#[test]
fn test_packet_type_all_variants() {
    // Discovery
    assert_eq!(
        PacketType::from_u8(0x01).unwrap(),
        PacketType::DiscoverRequest
    );
    assert_eq!(
        PacketType::from_u8(0x02).unwrap(),
        PacketType::DiscoverResponse
    );

    // Handshake
    assert_eq!(
        PacketType::from_u8(0x10).unwrap(),
        PacketType::HandshakeRequest
    );
    assert_eq!(
        PacketType::from_u8(0x11).unwrap(),
        PacketType::HandshakeResponse
    );

    // Stream
    assert_eq!(PacketType::from_u8(0x20).unwrap(), PacketType::StartStream);
    assert_eq!(
        PacketType::from_u8(0x21).unwrap(),
        PacketType::StartStreamAck
    );

    // Audio
    assert_eq!(PacketType::from_u8(0x30).unwrap(), PacketType::AudioData);
    assert_eq!(PacketType::from_u8(0x31).unwrap(), PacketType::AudioRtcp);

    // Control
    assert_eq!(
        PacketType::from_u8(0x40).unwrap(),
        PacketType::ControlModeSwitch
    );
    assert_eq!(
        PacketType::from_u8(0x41).unwrap(),
        PacketType::ControlParamUpdate
    );

    // Heartbeat
    assert_eq!(PacketType::from_u8(0xF0).unwrap(), PacketType::Heartbeat);
    assert_eq!(PacketType::from_u8(0xF1).unwrap(), PacketType::HeartbeatAck);
}

#[test]
fn test_packet_type_invalid() {
    assert!(PacketType::from_u8(0x00).is_err());
    assert!(PacketType::from_u8(0x03).is_err());
    assert!(PacketType::from_u8(0xFF).is_err());
    assert!(PacketType::from_u8(0xFE).is_err());
}

#[test]
fn test_packet_type_clone_copy() {
    let pt = PacketType::AudioData;
    let cloned = pt.clone();
    let copied = pt;
    assert_eq!(pt, cloned);
    assert_eq!(pt, copied);
}

// ============================================================================
// PacketHeader tests
// ============================================================================

#[test]
fn test_packet_header_size_constant() {
    assert_eq!(PacketHeader::SIZE, 12);
}

#[test]
fn test_packet_header_encode_decode_roundtrip() {
    let header = PacketHeader {
        sequence: 12345,
        timestamp_ms: 987654321,
        flags: 0x01,
        channels: 2,
        opus_length: 960,
    };

    let mut buf = Vec::new();
    header.encode(&mut buf).unwrap();

    assert_eq!(buf.len(), PacketHeader::SIZE);

    let (decoded, size) = PacketHeader::decode(&buf).unwrap();
    assert_eq!(size, PacketHeader::SIZE);
    assert_eq!(decoded.sequence, 12345);
    assert_eq!(decoded.timestamp_ms, 987654321);
    assert_eq!(decoded.flags, 0x01);
    assert_eq!(decoded.channels, 2);
    assert_eq!(decoded.opus_length, 960);
}

#[test]
fn test_packet_header_big_endian() {
    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 1,
        flags: 0,
        channels: 1,
        opus_length: 1,
    };

    let mut buf = Vec::new();
    header.encode(&mut buf).unwrap();

    // sequence = 0x00000001 in big endian
    assert_eq!(buf[0], 0x00);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x00);
    assert_eq!(buf[3], 0x01);

    // timestamp = 0x00000001 in big endian
    assert_eq!(buf[4], 0x00);
    assert_eq!(buf[5], 0x00);
    assert_eq!(buf[6], 0x00);
    assert_eq!(buf[7], 0x01);

    // flags
    assert_eq!(buf[8], 0x00);

    // channels
    assert_eq!(buf[9], 0x01);

    // opus_length = 0x0001 in big endian
    assert_eq!(buf[10], 0x00);
    assert_eq!(buf[11], 0x01);
}

#[test]
fn test_packet_header_data_too_short() {
    // Too short
    let buf = vec![0u8; 5];
    assert!(PacketHeader::decode(&buf).is_err());

    // Exactly right
    let buf = vec![0u8; 12];
    assert!(PacketHeader::decode(&buf).is_ok());

    // Longer is OK (returns size consumed)
    let buf = vec![0u8; 20];
    let (_, size) = PacketHeader::decode(&buf).unwrap();
    assert_eq!(size, 12);
}

#[test]
fn test_packet_header_zero_values() {
    let header = PacketHeader {
        sequence: 0,
        timestamp_ms: 0,
        flags: 0,
        channels: 0,
        opus_length: 0,
    };

    let mut buf = Vec::new();
    header.encode(&mut buf).unwrap();

    let (decoded, _) = PacketHeader::decode(&buf).unwrap();
    assert_eq!(decoded.sequence, 0);
    assert_eq!(decoded.timestamp_ms, 0);
    assert_eq!(decoded.flags, 0);
    assert_eq!(decoded.channels, 0);
    assert_eq!(decoded.opus_length, 0);
}

#[test]
fn test_packet_header_max_values() {
    let header = PacketHeader {
        sequence: u32::MAX,
        timestamp_ms: u32::MAX,
        flags: u8::MAX,
        channels: u8::MAX,
        opus_length: u16::MAX,
    };

    let mut buf = Vec::new();
    header.encode(&mut buf).unwrap();

    let (decoded, _) = PacketHeader::decode(&buf).unwrap();
    assert_eq!(decoded.sequence, u32::MAX);
    assert_eq!(decoded.timestamp_ms, u32::MAX);
    assert_eq!(decoded.flags, u8::MAX);
    assert_eq!(decoded.channels, u8::MAX);
    assert_eq!(decoded.opus_length, u16::MAX);
}

// ============================================================================
// ControlMessageType tests
// ============================================================================

#[test]
fn test_control_message_type_all_variants() {
    assert_eq!(
        ControlMessageType::from_u8(0x01).unwrap(),
        ControlMessageType::Hello
    );
    assert_eq!(
        ControlMessageType::from_u8(0x21).unwrap(),
        ControlMessageType::HelloAck
    );
    assert_eq!(
        ControlMessageType::from_u8(0x02).unwrap(),
        ControlMessageType::Auth
    );
    assert_eq!(
        ControlMessageType::from_u8(0x22).unwrap(),
        ControlMessageType::AuthAck
    );
    assert_eq!(
        ControlMessageType::from_u8(0x03).unwrap(),
        ControlMessageType::StartAudio
    );
    assert_eq!(
        ControlMessageType::from_u8(0x04).unwrap(),
        ControlMessageType::StopAudio
    );
    assert_eq!(
        ControlMessageType::from_u8(0x05).unwrap(),
        ControlMessageType::ChangeMode
    );
    assert_eq!(
        ControlMessageType::from_u8(0x06).unwrap(),
        ControlMessageType::Volume
    );
    assert_eq!(
        ControlMessageType::from_u8(0x07).unwrap(),
        ControlMessageType::Status
    );
    assert_eq!(
        ControlMessageType::from_u8(0x27).unwrap(),
        ControlMessageType::StatusAck
    );
    assert_eq!(
        ControlMessageType::from_u8(0x08).unwrap(),
        ControlMessageType::Heartbeat
    );
    assert_eq!(
        ControlMessageType::from_u8(0x28).unwrap(),
        ControlMessageType::HeartbeatAck
    );
    assert_eq!(
        ControlMessageType::from_u8(0xFF).unwrap(),
        ControlMessageType::Error
    );
}

#[test]
fn test_control_message_type_invalid() {
    assert!(ControlMessageType::from_u8(0x00).is_err());
    assert!(ControlMessageType::from_u8(0x09).is_err());
    assert!(ControlMessageType::from_u8(0x20).is_err());
    assert!(ControlMessageType::from_u8(0xFE).is_err());
}

// ============================================================================
// ControlMessage tests
// ============================================================================

#[test]
fn test_control_message_encode_decode_roundtrip() {
    let msg = ControlMessage {
        message_type: ControlMessageType::Hello,
        payload: vec![0x01, 0x02, 0x03],
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();

    assert_eq!(buf.len(), 8); // length(4) + type(1) + payload(3)

    let (decoded, size) = ControlMessage::decode(&buf).unwrap();
    assert_eq!(size, 8);
    assert_eq!(decoded.message_type, ControlMessageType::Hello);
    assert_eq!(decoded.payload, vec![0x01, 0x02, 0x03]);
}

#[test]
fn test_control_message_empty_payload() {
    let msg = ControlMessage {
        message_type: ControlMessageType::Heartbeat,
        payload: vec![],
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();

    assert_eq!(buf.len(), 5); // length(4) + type(1)

    let (decoded, _) = ControlMessage::decode(&buf).unwrap();
    assert_eq!(decoded.message_type, ControlMessageType::Heartbeat);
    assert!(decoded.payload.is_empty());
}

#[test]
fn test_control_message_large_payload() {
    let payload = vec![0xAB; 1000];
    let msg = ControlMessage {
        message_type: ControlMessageType::StartAudio,
        payload: payload.clone(),
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();

    let (decoded, _) = ControlMessage::decode(&buf).unwrap();
    assert_eq!(decoded.message_type, ControlMessageType::StartAudio);
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_control_message_data_too_short() {
    // Too short for header
    let buf = vec![0u8; 3];
    assert!(ControlMessage::decode(&buf).is_err());

    // Header says 10 bytes but only 7 provided
    let mut buf = Vec::new();
    buf.extend_from_slice(&10u32.to_be_bytes()); // length = 10
    buf.push(0x01); // type
    buf.push(0x02); // payload byte 1
                    // Missing 3 more bytes
    assert!(ControlMessage::decode(&buf).is_err());
}

// ============================================================================
// Protocol serialize/deserialize tests
// ============================================================================

#[test]
fn test_protocol_audio_packet_roundtrip() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 42,
        timestamp_ms: 1234567890,
        flags: 0x01, // Audio flag
        channels: 2,
        opus_length: 4,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x01, 0x02, 0x03, 0x04],
    };

    let serialized = protocol.serialize(&packet).unwrap();
    assert_eq!(serialized.len(), PacketHeader::SIZE + 4);

    let deserialized = protocol.deserialize(&serialized).unwrap();
    match deserialized {
        Packet::Audio { header, data } => {
            assert_eq!(header.sequence, 42);
            assert_eq!(header.timestamp_ms, 1234567890);
            assert_eq!(header.flags, 0x01);
            assert_eq!(header.channels, 2);
            assert_eq!(header.opus_length, 4);
            assert_eq!(data, vec![0x01, 0x02, 0x03, 0x04]);
        }
        _ => panic!("Expected Audio packet"),
    }
}

#[test]
fn test_protocol_control_packet_roundtrip() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 100,
        timestamp_ms: 5000,
        flags: 0x00, // Control flag
        channels: 0,
        opus_length: 2,
    };

    let packet = Packet::Control {
        header,
        data: vec![0xAA, 0xBB],
    };

    let serialized = protocol.serialize(&packet).unwrap();
    let deserialized = protocol.deserialize(&serialized).unwrap();

    match deserialized {
        Packet::Control { header, data } => {
            assert_eq!(header.sequence, 100);
            assert_eq!(header.timestamp_ms, 5000);
            assert_eq!(data, vec![0xAA, 0xBB]);
        }
        _ => panic!("Expected Control packet"),
    }
}

#[test]
fn test_protocol_serialize_into() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 100,
        flags: 0x01,
        channels: 1,
        opus_length: 2,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x11, 0x22],
    };

    let mut buf = Vec::with_capacity(100);
    protocol.serialize_into(&packet, &mut buf).unwrap();

    assert_eq!(buf.len(), PacketHeader::SIZE + 2);

    // Verify by deserializing
    let deserialized = protocol.deserialize(&buf).unwrap();
    match deserialized {
        Packet::Audio { header, data } => {
            assert_eq!(header.sequence, 1);
            assert_eq!(data, vec![0x11, 0x22]);
        }
        _ => panic!("Expected Audio packet"),
    }
}

#[test]
fn test_protocol_serialize_audio_into() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 5,
        timestamp_ms: 500,
        flags: 0x01,
        channels: 1,
        opus_length: 3,
    };

    let data = vec![0xAA, 0xBB, 0xCC];
    let mut buf = Vec::new();

    protocol
        .serialize_audio_into(&header, &data, &mut buf)
        .unwrap();

    assert_eq!(buf.len(), PacketHeader::SIZE + 3);

    // Verify
    let deserialized = protocol.deserialize(&buf).unwrap();
    match deserialized {
        Packet::Audio { header, data } => {
            assert_eq!(header.sequence, 5);
            assert_eq!(data, vec![0xAA, 0xBB, 0xCC]);
        }
        _ => panic!("Expected Audio packet"),
    }
}

#[test]
fn test_protocol_deserialize_header() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 99,
        timestamp_ms: 999,
        flags: 0x01,
        channels: 1,
        opus_length: 2,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x11, 0x22],
    };

    let serialized = protocol.serialize(&packet).unwrap();

    let (decoded_header, payload, is_audio) = protocol.deserialize_header(&serialized).unwrap();
    assert_eq!(decoded_header.sequence, 99);
    assert_eq!(decoded_header.timestamp_ms, 999);
    assert!(is_audio);
    assert_eq!(payload, &[0x11, 0x22]);
}

#[test]
fn test_protocol_control_flag_detection() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 10,
        timestamp_ms: 1000,
        flags: 0x00, // Control
        channels: 0,
        opus_length: 1,
    };

    let packet = Packet::Control {
        header,
        data: vec![0xFF],
    };

    let serialized = protocol.serialize(&packet).unwrap();
    let (_, _, is_audio) = protocol.deserialize_header(&serialized).unwrap();
    assert!(!is_audio);
}

#[test]
fn test_protocol_default() {
    let protocol = Protocol::default();
    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 100,
        flags: 0x01,
        channels: 1,
        opus_length: 1,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x42],
    };

    let serialized = protocol.serialize(&packet).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ProtocolError tests
// ============================================================================

#[test]
fn test_protocol_error_display() {
    let err = ProtocolError::InvalidMagic;
    assert!(err.to_string().contains("魔术数"));

    let err = ProtocolError::InvalidVersion(2);
    assert!(err.to_string().contains("2"));

    let err = ProtocolError::InvalidPacketType(0xFF);
    assert!(err.to_string().contains("255"));

    let err = ProtocolError::DataTooShort {
        needed: 12,
        actual: 5,
    };
    assert!(err.to_string().contains("12"));
    assert!(err.to_string().contains("5"));

    let err = ProtocolError::SerializationError("test".to_string());
    assert!(err.to_string().contains("test"));

    let err = ProtocolError::DeserializationError("test".to_string());
    assert!(err.to_string().contains("test"));
}

// ============================================================================
// Protocol constants tests
// ============================================================================

#[test]
fn test_magic_constant() {
    assert_eq!(MAGIC, 0x53424447); // "SBDG"
}

#[test]
fn test_protocol_version() {
    assert_eq!(PROTOCOL_VERSION, 1);
}

// ============================================================================
// Packet enum tests
// ============================================================================

#[test]
fn test_packet_clone() {
    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 100,
        flags: 0x01,
        channels: 1,
        opus_length: 2,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x01, 0x02],
    };

    let cloned = packet.clone();
    match cloned {
        Packet::Audio { header, data } => {
            assert_eq!(header.sequence, 1);
            assert_eq!(data, vec![0x01, 0x02]);
        }
        _ => panic!("Expected Audio packet"),
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_empty_audio_payload() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 100,
        flags: 0x01,
        channels: 1,
        opus_length: 0,
    };

    let packet = Packet::Audio {
        header,
        data: vec![],
    };

    let serialized = protocol.serialize(&packet).unwrap();
    assert_eq!(serialized.len(), PacketHeader::SIZE);

    let deserialized = protocol.deserialize(&serialized).unwrap();
    match deserialized {
        Packet::Audio { data, .. } => {
            assert!(data.is_empty());
        }
        _ => panic!("Expected Audio packet"),
    }
}

#[test]
fn test_multiple_sequential_packets() {
    let protocol = Protocol::new();

    for i in 0..10 {
        let header = PacketHeader {
            sequence: i,
            timestamp_ms: i * 1000,
            flags: 0x01,
            channels: 1,
            opus_length: 2,
        };

        let packet = Packet::Audio {
            header,
            data: vec![0x01, 0x02],
        };

        let serialized = protocol.serialize(&packet).unwrap();
        let deserialized = protocol.deserialize(&serialized).unwrap();

        match deserialized {
            Packet::Audio { header, .. } => {
                assert_eq!(header.sequence, i);
                assert_eq!(header.timestamp_ms, i * 1000);
            }
            _ => panic!("Expected Audio packet"),
        }
    }
}

#[test]
fn test_serialize_into_reuses_buffer() {
    let protocol = Protocol::new();

    let header = PacketHeader {
        sequence: 1,
        timestamp_ms: 100,
        flags: 0x01,
        channels: 1,
        opus_length: 1,
    };

    let packet = Packet::Audio {
        header,
        data: vec![0x42],
    };

    let mut buf = Vec::new();
    protocol.serialize_into(&packet, &mut buf).unwrap();
    let first_len = buf.len();

    // Second call should reuse buffer (clear + extend)
    protocol.serialize_into(&packet, &mut buf).unwrap();
    assert_eq!(buf.len(), first_len);
}
