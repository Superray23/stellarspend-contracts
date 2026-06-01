#![no_std]

pub mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Env};
use validation::{validate_transaction_timestamp, TimestampValidationError};

#[contract]
pub struct TransactionValidationContract;

#[contractimpl]
impl TransactionValidationContract {
    /// Validates a transaction payload and its timestamp.
    /// Acts as an entry point for transaction processing flows.
    pub fn process_transaction(
        env: Env,
        tx_timestamp: u64,
    ) -> Result<(), TimestampValidationError> {
        validate_transaction_timestamp(&env, tx_timestamp)
    }
}
