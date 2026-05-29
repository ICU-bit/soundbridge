//! Spike: WebRTC APM cross-compilation verification.
//! 
//! Goal: Verify that we can call WebRTC AudioProcessing C++ code from Rust
//! using the `webrtc-audio-processing` crate with the `bundled` feature.

use webrtc_audio_processing::Processor;

/// Create a Processor instance at 48kHz sample rate.
/// This proves that the WebRTC APM C++ code can be compiled and linked.
pub fn create_processor() -> Result<Processor, webrtc_audio_processing::Error> {
    Processor::new(48_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_processor() {
        let result = create_processor();
        assert!(result.is_ok(), "Failed to create Processor: {:?}", result.err());
        println!("Successfully created WebRTC AudioProcessing Processor at 48kHz");
    }

    #[test]
    fn test_processor_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Processor>();
    }
}
