use crate::errors::SharedError;

/// Validates that a batch size is within the configured bounds.
///
/// This helper is intentionally generic so each contract can pass its own
/// configured `max_batch_size` constant and map failures to contract-specific
/// error codes.
pub fn validate_batch_size(batch_size: u32, max_batch_size: u32) -> Result<(), SharedError> {
    if batch_size == 0 || batch_size > max_batch_size {
        return Err(SharedError::InvalidLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_batch_size_valid() {
        assert!(validate_batch_size(1, 100).is_ok());
        assert!(validate_batch_size(100, 100).is_ok());
    }

    #[test]
    fn test_validate_batch_size_zero_rejected() {
        assert_eq!(validate_batch_size(0, 100), Err(SharedError::InvalidLength));
    }

    #[test]
    fn test_validate_batch_size_exceeds_max() {
        assert_eq!(validate_batch_size(101, 100), Err(SharedError::InvalidLength));
    }

    #[test]
    fn test_validate_batch_size_configurable_limit() {
        assert!(validate_batch_size(10, 20).is_ok());
        assert_eq!(validate_batch_size(21, 20), Err(SharedError::InvalidLength));
    }
}
