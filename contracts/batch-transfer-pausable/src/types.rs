//! Type definitions for the pausable batch transfer contract

use soroban_sdk::{contracttype, Address, Vec};

/// Storage keys for the pausable batch transfer contract
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Admin address for this contract
    Admin,
    /// Pause/resume state of the contract
    IsPaused,
    /// Total number of batch transfers processed
    TotalTransfers,
}

/// Represents a single transfer request in a batch
#[contracttype]
#[derive(Clone, Debug)]
pub struct TransferRequest {
    /// Recipient of the transfer
    pub to: Address,
    /// Amount to transfer
    pub amount: u128,
}

/// Result of a single transfer operation
#[contracttype]
#[derive(Clone, Debug)]
pub enum TransferResult {
    /// Transfer succeeded with amount transferred
    Success(u128),
    /// Transfer failed with error code
    Failure(u32),
}

/// Result of a batch transfer operation
#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchTransferResult {
    /// Total number of transfer requests
    pub total_requests: u32,
    /// Number of successful transfers
    pub successful: u32,
    /// Number of failed transfers
    pub failed: u32,
    /// Individual results for each transfer
    pub results: Vec<TransferResult>,
}

/// Error codes for the pausable batch transfer contract
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BatchTransferError {
    /// Contract not initialized
    NotInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Contract is paused
    ContractPaused = 3,
    /// Batch is empty
    EmptyBatch = 4,
    /// Batch exceeds maximum size
    BatchTooLarge = 5,
    /// Invalid transfer request
    InvalidTransfer = 6,
}

impl From<BatchTransferError> for soroban_sdk::Error {
    fn from(e: BatchTransferError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}

/// Maximum batch size for transfers
pub const MAX_BATCH_SIZE: u32 = 100;
