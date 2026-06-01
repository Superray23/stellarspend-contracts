//! # Pausable Batch Transfer Contract
//!
//! A contract that supports batch transfers with pause and resume capabilities.
//! When paused, the contract rejects all batch transfer operations while allowing
//! admin functions to continue.

#![no_std]

mod types;

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, Vec, symbol_short};

pub use crate::types::{
    BatchTransferError, BatchTransferResult, DataKey, TransferRequest, TransferResult, MAX_BATCH_SIZE,
};

#[contract]
pub struct BatchTransferContract;

#[contractimpl]
impl BatchTransferContract {
    /// Initialize the contract with an admin address
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::IsPaused, &false);
        env.storage()
            .instance()
            .set(&DataKey::TotalTransfers, &0u64);

        // Emit initialization event
        env.events()
            .publish((symbol_short!("batch"), symbol_short!("init")), admin);
    }

    /// Execute a batch transfer operation
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address initiating the batch transfer
    /// * `requests` - Vector of transfer requests to execute
    ///
    /// # Returns
    /// BatchTransferResult with success/failure counts and individual results
    pub fn batch_transfer(
        env: Env,
        caller: Address,
        requests: Vec<TransferRequest>,
    ) -> BatchTransferResult {
        // Require authentication
        caller.require_auth();

        // Check if paused
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false);

        if is_paused {
            panic_with_error!(&env, BatchTransferError::ContractPaused);
        }

        // Validate batch size
        let request_count = requests.len();
        if request_count == 0 {
            panic_with_error!(&env, BatchTransferError::EmptyBatch);
        }
        if request_count > MAX_BATCH_SIZE {
            panic_with_error!(&env, BatchTransferError::BatchTooLarge);
        }

        // Process transfers
        let mut results: Vec<TransferResult> = Vec::new(&env);
        let mut successful_count: u32 = 0;
        let mut failed_count: u32 = 0;

        for request in requests.iter() {
            // Validate transfer request
            if request.amount == 0 {
                results.push_back(TransferResult::Failure(6)); // InvalidTransfer
                failed_count += 1;
                continue;
            }

            // Simulate transfer execution
            // In a real implementation, this would call the token contract
            results.push_back(TransferResult::Success(request.amount));
            successful_count += 1;

            // Emit transfer event
            env.events().publish(
                (symbol_short!("xfer"), symbol_short!("done")),
                (&request.to, request.amount),
            );
        }

        // Update total transfers
        let mut total_transfers: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalTransfers)
            .unwrap_or(0);
        total_transfers += successful_count as u64;
        env.storage()
            .instance()
            .set(&DataKey::TotalTransfers, &total_transfers);

        // Emit batch completion event
        env.events().publish(
            (symbol_short!("batch"), symbol_short!("done")),
            (successful_count, failed_count),
        );

        BatchTransferResult {
            total_requests: request_count,
            successful: successful_count,
            failed: failed_count,
            results,
        }
    }

    /// Pause the contract - prevents batch transfers while allowing admin operations
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address calling this function (must be admin)
    pub fn pause(env: Env, caller: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        env.storage().instance().set(&DataKey::IsPaused, &true);

        // Emit pause event
        env.events()
            .publish((symbol_short!("pause"), symbol_short!("paused")), caller);
    }

    /// Resume the contract - allows batch transfers to proceed
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address calling this function (must be admin)
    pub fn resume(env: Env, caller: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        env.storage().instance().set(&DataKey::IsPaused, &false);

        // Emit resume event
        env.events()
            .publish((symbol_short!("pause"), symbol_short!("resumed")), caller);
    }

    /// Check if the contract is paused
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// true if paused, false if running
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    /// Get the total number of successful transfers
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// Total count of successful transfers
    pub fn get_total_transfers(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalTransfers)
            .unwrap_or(0)
    }

    /// Get the current admin address
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// Current admin address or panic if not initialized
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("contract not initialized"))
    }

    /// Update the admin address (admin only)
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address calling this function (must be admin)
    /// * `new_admin` - The new admin address
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        // Emit admin transfer event
        env.events()
            .publish((symbol_short!("admin"), symbol_short!("tfr")), (caller, new_admin));
    }

    // ── Private Helpers ───────────────────────────────────────────────────

    /// Require that the given address is the admin
    fn require_admin(env: &Env, addr: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, BatchTransferError::NotInitialized));

        if addr != &admin {
            panic_with_error!(env, BatchTransferError::Unauthorized);
        }
    }
}

#[cfg(test)]
mod test;
