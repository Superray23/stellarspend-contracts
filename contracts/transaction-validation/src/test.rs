#![cfg(test)]

use crate::validation::{
    validate_transaction_timestamp, TimestampValidationError, MAX_FUTURE_THRESHOLD,
    MAX_PAST_THRESHOLD,
};
use soroban_sdk::testutils::Ledger;
use soroban_sdk::Env;

#[test]
fn test_valid_timestamp_exact() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    assert_eq!(validate_transaction_timestamp(&env, 1000), Ok(()));
}

#[test]
fn test_valid_future_timestamp_within_bounds() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    // 100 seconds in the future is fine
    assert_eq!(validate_transaction_timestamp(&env, 1100), Ok(()));
}

#[test]
fn test_invalid_future_timestamp_beyond_bounds() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let too_far_future = 1000 + MAX_FUTURE_THRESHOLD + 1;
    assert_eq!(
        validate_transaction_timestamp(&env, too_far_future),
        Err(TimestampValidationError::FutureTimestamp)
    );
}

#[test]
fn test_valid_past_timestamp_within_bounds() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    // 100 seconds in the past is fine
    assert_eq!(validate_transaction_timestamp(&env, 900), Ok(()));
}

#[test]
fn test_invalid_past_timestamp_beyond_bounds() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let too_far_past = 1000 - MAX_PAST_THRESHOLD - 1;
    assert_eq!(
        validate_transaction_timestamp(&env, too_far_past),
        Err(TimestampValidationError::PastTimestamp)
    );
}
