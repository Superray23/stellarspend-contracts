//! Validation logic for transaction memo length.

use soroban_sdk::String;

/// Maximum memo text length in bytes (256 bytes to prevent oversized payloads).
pub const MAX_MEMO_TEXT_LENGTH: u32 = 256;

/// Maximum memo reference length in bytes.
pub const MAX_MEMO_REFERENCE_LENGTH: u32 = 64;

/// Maximum total memo size in bytes.
pub const MAX_TOTAL_MEMO_SIZE: u32 = 320;

/// Validates that a memo text string is within length bounds.
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(())` if empty or exceeds maximum length
pub fn validate_memo_text_length(text: &String) -> Result<(), ()> {
    let len = text.len() as u32;
    if len == 0 || len > MAX_MEMO_TEXT_LENGTH {
        return Err(());
    }
    Ok(())
}

/// Validates that a memo reference string is within length bounds.
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(())` if exceeds maximum reference length
pub fn validate_memo_reference_length(reference: &String) -> Result<(), ()> {
    let len = reference.len() as u32;
    if len > MAX_MEMO_REFERENCE_LENGTH {
        return Err(());
    }
    Ok(())
}

/// Validates total memo payload size (text + reference + type name).
///
/// # Returns
/// * `Ok(())` if total size is within bounds
/// * `Err(())` if exceeds maximum total size
pub fn validate_total_memo_size(text_len: u32, reference_len: u32, type_len: u32) -> Result<(), ()> {
    let total = text_len + reference_len + type_len;
    if total > MAX_TOTAL_MEMO_SIZE {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String};

    #[test]
    fn test_valid_memo_text_length() {
        let env = Env::default();
        let text = String::from_str(&env, "Payment for services");
        assert!(validate_memo_text_length(&text).is_ok());
    }

    #[test]
    fn test_memo_text_at_max_length() {
        let env = Env::default();
        let text = String::from_str(&env, &"a".repeat(MAX_MEMO_TEXT_LENGTH as usize));
        assert!(validate_memo_text_length(&text).is_ok());
    }

    #[test]
    fn test_memo_text_over_max_length() {
        let env = Env::default();
        let text = String::from_str(&env, &"a".repeat((MAX_MEMO_TEXT_LENGTH + 1) as usize));
        assert!(validate_memo_text_length(&text).is_err());
    }

    #[test]
    fn test_empty_memo_text_rejected() {
        let env = Env::default();
        let text = String::from_str(&env, "");
        assert!(validate_memo_text_length(&text).is_err());
    }

    #[test]
    fn test_valid_reference_length() {
        let env = Env::default();
        let reference = String::from_str(&env, "REF-12345");
        assert!(validate_memo_reference_length(&reference).is_ok());
    }

    #[test]
    fn test_reference_over_max_length() {
        let env = Env::default();
        let reference = String::from_str(&env, &"r".repeat((MAX_MEMO_REFERENCE_LENGTH + 1) as usize));
        assert!(validate_memo_reference_length(&reference).is_err());
    }

    #[test]
    fn test_reference_at_max_length() {
        let env = Env::default();
        let reference = String::from_str(&env, &"r".repeat(MAX_MEMO_REFERENCE_LENGTH as usize));
        assert!(validate_memo_reference_length(&reference).is_ok());
    }

    #[test]
    fn test_total_memo_size_within_bounds() {
        assert!(validate_total_memo_size(200, 50, 20).is_ok());
    }

    #[test]
    fn test_total_memo_size_at_boundary() {
        assert!(validate_total_memo_size(256, 64, 0).is_ok());
    }

    #[test]
    fn test_total_memo_size_exceeds_maximum() {
        assert!(validate_total_memo_size(300, 50, 20).is_err());
    }
}
