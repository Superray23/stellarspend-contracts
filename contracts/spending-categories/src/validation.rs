//! Validation logic for spending category names.

use soroban_sdk::Symbol;

/// Validates that a category name is not empty and has reasonable length.
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(())` if invalid
pub fn validate_category_name(name: &Symbol) -> Result<(), ()> {
    // In Soroban, Symbols are always valid by construction,
    // but we verify the name is non-empty and within bounds.
    // A Symbol's length is the number of characters.
    // We check that it's not an empty symbol and within a reasonable max length.
    let len = name.len();
    if len == 0 || len > 32 {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol_short, Env};

    #[test]
    fn test_valid_category_name() {
        let env = Env::default();
        let name = symbol_short!("Food");
        assert!(validate_category_name(&name).is_ok());
    }

    #[test]
    fn test_valid_short_name() {
        let env = Env::default();
        let name = symbol_short!("A");
        assert!(validate_category_name(&name).is_ok());
    }

    #[test]
    fn test_valid_long_name() {
        let env = Env::default();
        // 32-char name
        let name = Symbol::new(&env, "abcdefghijklmnopqrstuvwxyz012345");
        assert!(validate_category_name(&name).is_ok());
    }
}
