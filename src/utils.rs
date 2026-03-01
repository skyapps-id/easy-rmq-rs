use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique trace ID
/// Format: {timestamp_hex}-{random_hex}
/// Example: 66d8f4a2b3c1d-7f8a9b2c
pub fn generate_trace_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let random: u64 = rand::random();

    format!("{:x}-{:x}", timestamp, random)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();

        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2);
        assert!(id1.len() > 10);
    }
}
