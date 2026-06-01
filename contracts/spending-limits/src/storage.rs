use soroban_sdk::{contracttype, Address, BytesN};

/// Storage key for budget records (uses BytesN<32> for optimized key size).
#[derive(Clone)]
#[contracttype]
pub enum BudgetStorageKey {
    Budget(BytesN<32>),
}