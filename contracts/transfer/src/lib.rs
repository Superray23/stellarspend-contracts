#![no_std]

use shared::errors::SharedError;
use shared::sanitizer::sanitize_description;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};

#[contract]
pub struct TransferContract;

#[contractimpl]
impl TransferContract {
    /// Executes a transfer and records its description after sanitization.
    /// Reverts if the description contains malformed or unsupported characters.
    pub fn execute_transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
        description: String,
    ) -> Result<(), SharedError> {
        from.require_auth();

        // 1. Sanitize the transfer description
        // Rejects invalid characters to prevent malformed text storage.
        sanitize_description(&env, &description)?;

        // 2. Perform the actual transfer logic here (mocked for this package)
        // In a real scenario, this would call the token contract:
        // token::Client::new(&env, &token_address).transfer(&from, &to, &amount);

        // 3. Emit an event containing the clean description
        let topics = (symbol_short!("transfer"), symbol_short!("executed"));
        env.events()
            .publish(topics, (from, to, amount, description));

        Ok(())
    }
}

#[cfg(test)]
mod test;
