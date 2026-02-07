/// Generate a new UUID v4 request ID for correlation.
pub fn new_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_request_id_format() {
        let id = new_request_id();
        // UUID v4 format: 8-4-4-4-12 hex chars
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn test_new_request_id_unique() {
        let id1 = new_request_id();
        let id2 = new_request_id();
        assert_ne!(id1, id2);
    }
}
