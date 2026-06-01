use soroban_sdk::string::ToString;
use soroban_sdk::{Env, String, Symbol};

/// Generate a unique transaction ID
pub fn generate_transaction_id(env: &Env) -> Symbol {
    // Use a persistent counter for unique IDs
    let mut counter: u64 = env
        .storage()
        .persistent()
        .get(&crate::storage::DataKey::TransactionCounter)
        .unwrap_or(0);

    counter += 1;

    // Create ID string
    let id_str = String::from_str(env, &counter.to_string());

    // Update counter
    env.storage()
        .persistent()
        .set(&crate::storage::DataKey::TransactionCounter, &counter);

    Symbol::new(env, id_str.as_str())
}
