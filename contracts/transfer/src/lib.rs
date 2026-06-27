#![no_std]

use shared::errors::SharedError;
use shared::sanitizer::sanitize_description;
use shared::utils::generate_transaction_reference_id;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};

#[contract]
pub struct TransferContract;

#[contractimpl]
impl TransferContract {
    /// Executes a transfer and records its description after sanitization.
    /// Generates a unique transaction reference ID for tracking and reconciliation.
    /// Reverts if the description contains malformed or unsupported characters.
    pub fn execute_transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
        description: String,
    ) -> Result<String, SharedError> {
        from.require_auth();

        // 1. Sanitize the transfer description
        // Rejects invalid characters to prevent malformed text storage.
        sanitize_description(&description)?;

        // 2. Generate a unique transaction reference ID
        let ref_id_counter_key = Symbol::new(&env, "transfer_ref_counter");
        let reference_id = generate_transaction_reference_id(&env, &from, &ref_id_counter_key);

        // 3. Perform the actual transfer logic here (mocked for this package)
        // In a real scenario, this would call the token contract:
        // token::Client::new(&env, &token_address).transfer(&from, &to, &amount);

        // 4. Emit an event containing the reference ID, clean description, and transfer details
        let topics = (
            symbol_short!("transfer"),
            symbol_short!("executed"),
            reference_id.clone(),
        );
        env.events()
            .publish(topics, (from, to, amount, description));

        Ok(reference_id)
    }
}

#[cfg(test)]
mod test;
