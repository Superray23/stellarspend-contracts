use soroban_sdk::{contracterror, Env};

/// Error type for timestamp validation
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TimestampValidationError {
    /// Timestamp is too far in the future
    FutureTimestamp = 1,
    /// Timestamp is too far in the past
    PastTimestamp = 2,
}

/// The maximum allowed difference in seconds for future timestamps.
/// 300 seconds (5 minutes) threshold.
pub const MAX_FUTURE_THRESHOLD: u64 = 300;

/// The maximum allowed difference in seconds for past timestamps.
/// 600 seconds (10 minutes) threshold.
pub const MAX_PAST_THRESHOLD: u64 = 600;

/// Validates that a transaction timestamp falls within an acceptable window
/// compared to the current ledger timestamp.
///
/// Prevents replay attacks and inconsistent ordering.
pub fn validate_transaction_timestamp(
    env: &Env,
    tx_timestamp: u64,
) -> Result<(), TimestampValidationError> {
    let current_timestamp = env.ledger().timestamp();

    // Check future bounds
    if tx_timestamp > current_timestamp {
        let diff = tx_timestamp - current_timestamp;
        if diff > MAX_FUTURE_THRESHOLD {
            return Err(TimestampValidationError::FutureTimestamp);
        }
    }

    // Check past bounds
    if current_timestamp > tx_timestamp {
        let diff = current_timestamp - tx_timestamp;
        if diff > MAX_PAST_THRESHOLD {
            return Err(TimestampValidationError::PastTimestamp);
        }
    }

    Ok(())
}
