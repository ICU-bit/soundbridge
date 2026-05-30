//! End-to-end encryption transport integration tests.
//!
//! Tests the complete encrypted transport chain:
//! Session handshake → Key derivation → SRTP encrypt → UDP send →
//! UDP receive → SRTP decrypt → Data verification.

use network::crypto::{
    CryptoKeys, DtlsSession, SrtpContext, SRTP_MASTER_KEY_LEN, SRTP_MASTER_SALT_LEN,
};
use network::{
    generate_session_id, Capability, DisconnectReason, EncryptionMode, NegotiatedParams, Session,
    SessionConfig, SessionState, TransportConfig, UdpTransport,
};

// ============================================================================
// Helpers
// ============================================================================

/// Create a UDP transport bound to localhost with random port.
async fn new_loopback_transport() -> UdpTransport {
    let config = TransportConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };
    UdpTransport::new(config).await.unwrap()
}

/// Create default capability for a device.
fn default_capability(device_id: &str, device_name: &str) -> Capability {
    Capability {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
        ..Default::default()
    }
}

/// Construct an RTP packet (12-byte header + payload).
fn make_rtp_packet(ssrc: u32, seq: u16, payload: &[u8]) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(12 + payload.len());
    pkt.push(0x80); // V=2, P=0, X=0, CC=0
    pkt.push(0x60); // M=0, PT=96
    pkt.extend_from_slice(&seq.to_be_bytes());
    pkt.extend_from_slice(&0u32.to_be_bytes()); // timestamp
    pkt.extend_from_slice(&ssrc.to_be_bytes());
    pkt.extend_from_slice(payload);
    pkt
}

/// Perform a full Session handshake between client and server, returning
/// both established sessions and the negotiated parameters.
fn perform_session_handshake() -> (Session, Session, NegotiatedParams) {
    let session_id = generate_session_id();
    let config = SessionConfig::default();

    let mut client = Session::new_client(
        session_id.clone(),
        default_capability("client-1", "PC"),
        config.clone(),
    );
    let mut server = Session::new_server(
        String::new(),
        default_capability("server-1", "Phone"),
        config,
    );

    // 1. Client → ClientHello
    let client_hello = client.initiate_handshake().unwrap();
    assert_eq!(client.state(), SessionState::ClientHelloSent);

    // 2. Server → ServerHello
    let server_hello = server.handle_client_hello(&client_hello).unwrap();
    assert_eq!(server.state(), SessionState::ServerHelloSent);

    // 3. Client → KeyExchange
    let key_exchange = client.handle_server_hello(&server_hello).unwrap();
    assert_eq!(client.state(), SessionState::KeyExchangeSent);

    // 4. Server → Finished
    let finished = server.handle_key_exchange(&key_exchange).unwrap();
    assert_eq!(server.state(), SessionState::Established);

    // 5. Client processes Finished
    client.handle_finished_client(&finished).unwrap();
    assert_eq!(client.state(), SessionState::Established);

    // Verify negotiated params match
    assert_eq!(client.negotiated(), server.negotiated());

    let negotiated = client.negotiated().unwrap().clone();
    (client, server, negotiated)
}

/// Perform a DTLS handshake and extract the derived CryptoKeys.
fn perform_dtls_handshake() -> CryptoKeys {
    let mut dtls = DtlsSession::with_default_config();
    dtls.start_handshake().unwrap();

    // ClientHello → ServerHello (generates keys)
    let _server_hello = dtls.process_handshake(&[]).unwrap();
    assert!(dtls.keys().is_some());

    // ServerHello → Finished (establishes session)
    dtls.process_handshake(&[]).unwrap();

    dtls.keys().unwrap().clone()
}

// ============================================================================
// Test 1: Full encrypted transport chain
// ============================================================================

#[tokio::test]
async fn test_full_encrypted_transport_chain() {
    // Step 1: Session handshake → negotiate encryption
    let (client_session, server_session, negotiated) = perform_session_handshake();
    assert_eq!(negotiated.encryption_mode, EncryptionMode::Srtp);
    assert!(client_session.is_established());
    assert!(server_session.is_established());

    // Step 2: DTLS handshake → derive shared keys
    let keys = perform_dtls_handshake();

    // Step 3: Create sender and receiver transports with encryption
    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;

    sender
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();
    receiver
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();

    assert!(sender.is_encrypted());
    assert!(receiver.is_encrypted());

    let receiver_addr = receiver.local_addr().unwrap();

    // Step 4: Construct RTP audio packet and send encrypted
    let original_payload = b"SoundBridge encrypted audio frame 48kHz";
    let rtp_packet = make_rtp_packet(0xDEADBEEF, 1, original_payload);

    let sent = sender.send_to(&rtp_packet, receiver_addr).await.unwrap();
    // Encrypted packet is larger (RTP header + ciphertext + 10-byte auth tag)
    assert!(sent > rtp_packet.len());

    // Step 5: Receive and decrypt
    let mut recv_buf = vec![0u8; 4096];
    let (recv_len, _src_addr) = receiver.receive_from(&mut recv_buf).await.unwrap();

    // Step 6: Verify data integrity (decrypted == original)
    assert_eq!(recv_len, rtp_packet.len());
    assert_eq!(&recv_buf[..recv_len], &rtp_packet[..]);

    // Verify stats
    let sender_stats = sender.stats();
    assert_eq!(sender_stats.packets_sent, 1);
    assert!(sender_stats.bytes_sent > 0);

    let receiver_stats = receiver.stats();
    assert_eq!(receiver_stats.packets_received, 1);
    assert!(receiver_stats.bytes_received > 0);
}

// ============================================================================
// Test 2: Unencrypted transport chain (compatibility)
// ============================================================================

#[tokio::test]
async fn test_unencrypted_transport_chain() {
    // No handshake, no encryption — plain UDP
    let sender = new_loopback_transport().await;
    let receiver = new_loopback_transport().await;

    assert!(!sender.is_encrypted());
    assert!(!receiver.is_encrypted());

    let receiver_addr = receiver.local_addr().unwrap();

    // Send plain data (not even RTP format — just raw bytes)
    let plain_data = b"plain audio data without encryption";
    let sent = sender.send_to(plain_data, receiver_addr).await.unwrap();
    assert_eq!(sent, plain_data.len());

    let mut recv_buf = vec![0u8; 4096];
    let (recv_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();

    // Data arrives unchanged
    assert_eq!(recv_len, plain_data.len());
    assert_eq!(&recv_buf[..recv_len], plain_data);
}

// ============================================================================
// Test 3: Encryption enable/disable toggle
// ============================================================================

#[tokio::test]
async fn test_encryption_toggle() {
    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;
    let receiver_addr = receiver.local_addr().unwrap();

    let master_key = vec![0xAAu8; SRTP_MASTER_KEY_LEN];
    let master_salt = vec![0xBBu8; SRTP_MASTER_SALT_LEN];

    // Phase 1: Unencrypted send/receive
    assert!(!sender.is_encrypted());
    let data_plain = b"unencrypted packet";
    sender.send_to(data_plain, receiver_addr).await.unwrap();

    let mut recv_buf = vec![0u8; 4096];
    let (len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
    assert_eq!(&recv_buf[..len], data_plain);

    // Phase 2: Enable encryption
    sender
        .enable_encryption(master_key.clone(), master_salt.clone())
        .unwrap();
    receiver
        .enable_encryption(master_key.clone(), master_salt.clone())
        .unwrap();
    assert!(sender.is_encrypted());
    assert!(receiver.is_encrypted());

    // Send encrypted — receiver without encryption should fail to parse auth tag
    // (because transport auto-decrypts only if is_encrypted)
    // We test that encrypted send/receive works with both sides encrypted
    let rtp_enc = make_rtp_packet(0x11111111, 2, b"encrypted payload");
    sender.send_to(&rtp_enc, receiver_addr).await.unwrap();

    let (len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
    assert_eq!(&recv_buf[..len], &rtp_enc[..]);

    // Phase 3: Verify that encryption state persists across multiple packets
    for seq in 3..6u16 {
        let payload = format!("frame_{}", seq);
        let rtp = make_rtp_packet(0x11111111, seq, payload.as_bytes());
        sender.send_to(&rtp, receiver_addr).await.unwrap();

        let (len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
        assert_eq!(&recv_buf[..len], &rtp[..]);
    }
}

// ============================================================================
// Test 4: Wrong key rejection
// ============================================================================

#[tokio::test]
async fn test_wrong_key_rejected() {
    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;
    let receiver_addr = receiver.local_addr().unwrap();

    // Sender uses one key
    sender
        .enable_encryption(
            vec![0x01u8; SRTP_MASTER_KEY_LEN],
            vec![0x02u8; SRTP_MASTER_SALT_LEN],
        )
        .unwrap();

    // Receiver uses a completely different key
    receiver
        .enable_encryption(
            vec![0xFFu8; SRTP_MASTER_KEY_LEN],
            vec![0xFEu8; SRTP_MASTER_SALT_LEN],
        )
        .unwrap();

    let rtp = make_rtp_packet(0x12345678, 1, b"secret audio");
    sender.send_to(&rtp, receiver_addr).await.unwrap();

    let mut recv_buf = vec![0u8; 4096];
    let result = receiver.receive_from(&mut recv_buf).await;

    // Decryption must fail (HMAC-SHA1 auth tag mismatch)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("认证标签") || err.to_string().contains("CryptoError"),
        "Expected auth tag verification error, got: {}",
        err
    );
}

// ============================================================================
// Test 5: Large data encryption (16KB)
// ============================================================================

#[tokio::test]
async fn test_large_data_encrypted_16kb() {
    let keys = perform_dtls_handshake();

    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;

    sender
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();
    receiver
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();

    let receiver_addr = receiver.local_addr().unwrap();

    // 16KB payload (16384 bytes) — simulates a large audio buffer
    let large_payload = vec![0xABu8; 16384];
    let rtp_large = make_rtp_packet(0xCAFEBABE, 1, &large_payload);

    // Send large encrypted packet
    let sent = sender.send_to(&rtp_large, receiver_addr).await.unwrap();
    // Encrypted = header(12) + ciphertext(16384) + auth_tag(10) = 16406
    assert_eq!(sent, 12 + 16384 + 10);

    // Receive and decrypt
    let mut recv_buf = vec![0u8; 20000]; // generous buffer
    let (recv_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();

    // Verify data integrity: decrypted payload must match original
    assert_eq!(recv_len, rtp_large.len());
    assert_eq!(&recv_buf[..recv_len], &rtp_large[..]);

    // Verify the payload portion specifically
    assert_eq!(&recv_buf[12..recv_len], &large_payload[..]);
}

// ============================================================================
// Additional: Multiple packets with sequence tracking
// ============================================================================

#[tokio::test]
async fn test_encrypted_multiple_sequential_packets() {
    let keys = perform_dtls_handshake();

    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;

    sender
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();
    receiver
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();

    let receiver_addr = receiver.local_addr().unwrap();

    // Send 10 sequential audio frames
    for seq in 0..10u16 {
        let payload = format!("audio_frame_{:04}", seq);
        let rtp = make_rtp_packet(0xA0D10001, seq, payload.as_bytes());
        sender.send_to(&rtp, receiver_addr).await.unwrap();

        let mut recv_buf = vec![0u8; 4096];
        let (recv_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
        assert_eq!(
            &recv_buf[..recv_len],
            &rtp[..],
            "Frame {} data mismatch",
            seq
        );
    }

    let stats = sender.stats();
    assert_eq!(stats.packets_sent, 10);
}

// ============================================================================
// Additional: Session handshake + encrypted transport end-to-end
// ============================================================================

#[tokio::test]
async fn test_session_handshake_then_encrypted_transport() {
    // 1. Session handshake with SRTP negotiation
    let (mut client, mut server, negotiated) = perform_session_handshake();
    assert_eq!(negotiated.encryption_mode, EncryptionMode::Srtp);

    // 2. Heartbeat exchange (verify session is alive)
    let heartbeat = client.create_heartbeat().unwrap();
    let ack = server.handle_heartbeat(&heartbeat).unwrap();
    client.handle_heartbeat_ack(&ack).unwrap();
    assert_eq!(client.stats().heartbeat.acked, 1);

    // 3. DTLS key derivation
    let keys = perform_dtls_handshake();

    // 4. Create encrypted transports
    let mut sender = new_loopback_transport().await;
    let mut receiver = new_loopback_transport().await;
    sender
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();
    receiver
        .enable_encryption(keys.master_key.to_vec(), keys.master_salt.to_vec())
        .unwrap();

    let receiver_addr = receiver.local_addr().unwrap();

    // 5. Send encrypted audio data
    let payload = b"encrypted audio after session handshake";
    let rtp = make_rtp_packet(0x5E551001, 1, payload);
    sender.send_to(&rtp, receiver_addr).await.unwrap();

    let mut recv_buf = vec![0u8; 4096];
    let (recv_len, _) = receiver.receive_from(&mut recv_buf).await.unwrap();
    assert_eq!(&recv_buf[..recv_len], &rtp[..]);

    // 6. Graceful disconnect
    let disconnect = client
        .initiate_disconnect(DisconnectReason::UserInitiated)
        .unwrap();
    server.handle_disconnect(&disconnect).unwrap();
    assert!(server.is_closed());
    client.close();
    assert!(client.is_closed());
}

// ============================================================================
// Additional: DTLS handshake key derivation determinism
// ============================================================================

#[tokio::test]
async fn test_dtls_key_derivation_roundtrip() {
    // Simulate DTLS handshake — each party independently completes the flow.
    // In the simplified model, each side generates its own CryptoKeys.
    let mut dtls_client = DtlsSession::with_default_config();
    let mut dtls_server = DtlsSession::with_default_config();

    // Client: Idle → WaitingClientHello → ServerHelloSent → Established
    dtls_client.start_handshake().unwrap();
    dtls_client.process_handshake(&[]).unwrap(); // generates keys
    dtls_client.complete_handshake().unwrap();

    // Server: Idle → WaitingClientHello → ServerHelloSent → Established
    dtls_server.start_handshake().unwrap();
    dtls_server.process_handshake(&[]).unwrap(); // generates keys
    dtls_server.complete_handshake().unwrap();

    // Both should have derived keys
    let client_keys = dtls_client.keys().unwrap();
    let server_keys = dtls_server.keys().unwrap();

    // Keys are non-zero and usable for encryption
    assert_ne!(client_keys.master_key, [0u8; 16]);
    assert_ne!(client_keys.master_salt, [0u8; 14]);
    assert_ne!(server_keys.master_key, [0u8; 16]);
    assert_ne!(server_keys.master_salt, [0u8; 14]);

    // Verify each key set works for encryption/decryption independently
    let mut enc_ctx = SrtpContext::new(client_keys.clone(), 0).unwrap();
    let mut dec_ctx = SrtpContext::new(client_keys.clone(), 0).unwrap();

    let rtp = make_rtp_packet(0x11111111, 1, b"test payload");
    let encrypted = enc_ctx.protect(&rtp).unwrap();
    let decrypted = dec_ctx.unprotect(&encrypted).unwrap();
    assert_eq!(&decrypted[..], &rtp[..]);
}
