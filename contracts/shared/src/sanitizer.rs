extern crate alloc;

use crate::errors::SharedError;
use soroban_sdk::String;

/// Maximum allowed description length.
pub const MAX_DESCRIPTION_LENGTH: u32 = 256;

/// Validates a user-provided description.
///
/// Rules:
/// - Must not exceed `MAX_DESCRIPTION_LENGTH`.
/// - Must contain only printable ASCII characters.
/// - Allows:
///   - letters (A-Z, a-z)
///   - numbers (0-9)
///   - spaces
///   - punctuation: . , - _ ! ? '
/// - Rejects:
///   - control characters
///   - HTML/XML delimiters
///   - emojis and non-ASCII characters
pub fn sanitize_description(description: &String) -> Result<(), SharedError> {
    let len = description.len();

    if len == 0 {
        return Err(SharedError::MissingRequiredField);
    }

    if len > MAX_DESCRIPTION_LENGTH {
        return Err(SharedError::InvalidLength);
    }

    let mut bytes = [0u8; MAX_DESCRIPTION_LENGTH as usize];
    description.copy_into_slice(&mut bytes[..len as usize]);

    for byte in bytes.iter().take(len as usize) {
        if !is_allowed_ascii(*byte) {
            return Err(SharedError::InvalidInput);
        }
    }

    Ok(())
}

/// Returns true if a byte is an allowed description character.
#[inline]
fn is_allowed_ascii(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b' '
            | b'.'
            | b','
            | b'-'
            | b'_'
            | b'!'
            | b'?'
            | b'\''
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String};

    fn make_string(env: &Env, value: &str) -> String {
        String::from_str(env, value)
    }

    #[test]
    fn accepts_valid_descriptions() {
        let env = Env::default();

        let valid_cases = [
            "Valid transfer description",
            "Payment for service!",
            "Invoice_123-A.",
            "Are you sure?",
            "It's a test",
            "Order 123, completed.",
        ];

        for text in valid_cases {
            let value = make_string(&env, text);
            assert_eq!(sanitize_description(&value), Ok(()));
        }
    }

    #[test]
    fn rejects_invalid_characters() {
        let env = Env::default();

        let invalid_cases = [
            "Invalid\nnewline",
            "Invalid\ttab",
            "HTML <script>alert(1)</script>",
            "Symbols like @ or #",
            "Emoji 🔥",
            "€100 payment",
        ];

        for text in invalid_cases {
            let value = make_string(&env, text);

            assert_eq!(
                sanitize_description(&value),
                Err(SharedError::InvalidInput)
            );
        }
    }

    #[test]
    fn rejects_empty_description() {
        let env = Env::default();

        let value = make_string(&env, "");

        assert_eq!(
            sanitize_description(&value),
            Err(SharedError::MissingRequiredField)
        );
    }

    #[test]
    fn accepts_max_length_boundary() {
        let env = Env::default();

        let text = "A".repeat(MAX_DESCRIPTION_LENGTH as usize);
        let value = make_string(&env, &text);

        assert_eq!(sanitize_description(&value), Ok(()));
    }

    #[test]
    fn rejects_too_long_description() {
        let env = Env::default();

        let text = "A".repeat((MAX_DESCRIPTION_LENGTH + 1) as usize);
        let value = make_string(&env, &text);

        assert_eq!(
            sanitize_description(&value),
            Err(SharedError::InvalidLength)
        );
    }

    #[test]
    fn rejects_non_ascii_characters() {
        let env = Env::default();

        let value = make_string(&env, "Café");

        assert_eq!(
            sanitize_description(&value),
            Err(SharedError::InvalidInput)
        );
    }
}