// Validation helpers for shared budget operations.

/// Validates an amount.
pub fn validate_amount(amount: i128) -> Result<(), &'static str> {
    if amount <= 0 {
        return Err("invalid_amount");
    }
    Ok(())
}

/// Validates a percentage value (0-100).
pub fn validate_percentage(percentage: u32) -> Result<(), &'static str> {
    if percentage > 100 {
        return Err("invalid_percentage");
    }
    Ok(())
}
