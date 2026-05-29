use network::{JitterBuffer, JitterBufferConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_buffer_creation() {
        let config = JitterBufferConfig::default();
        let _jb = JitterBuffer::new(config);
    }

    #[test]
    fn test_jitter_buffer_push_pop() {
        let mut jb = JitterBuffer::with_default_config();

        jb.push(1, vec![1.0f32; 100]);
        jb.push(2, vec![2.0f32; 100]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);
    }

    #[test]
    fn test_jitter_buffer_empty() {
        let mut jb = JitterBuffer::with_default_config();
        assert!(jb.pop().is_none());
    }

    #[test]
    fn test_jitter_buffer_stats() {
        let mut jb = JitterBuffer::with_default_config();

        jb.push(1, vec![1.0f32; 100]);
        jb.push(2, vec![2.0f32; 100]);

        assert_eq!(jb.len(), 2);
        assert!(!jb.is_empty());
    }
}
