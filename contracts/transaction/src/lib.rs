#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol, Vec, Map};

#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    pub amount: i128,
    pub sender: Symbol,
    pub receiver: Symbol,
    pub export: bool,
    pub updated_at: u64,
}

const STORAGE_KEY: Symbol = Symbol::short("TXS");
const USER_COUNT_KEY: Symbol = Symbol::short("TX_COUNT");
const MAX_TX_PER_USER: u32 = 5; // ✅ limit rule

#[contract]
pub struct TransactionContract;

#[contractimpl]
impl TransactionContract {

    /// Add transaction with per-user limit enforcement
    pub fn add_transaction(env: Env, tx: Transaction) {
        let mut storage: Map<u64, Transaction> =
            env.storage().instance().get(&STORAGE_KEY).unwrap_or(Map::new(&env));

        // Track per-user transaction count
        let mut counts: Map<Symbol, u32> =
            env.storage().instance().get(&USER_COUNT_KEY).unwrap_or(Map::new(&env));

        let mut current_count = counts.get(tx.sender.clone()).unwrap_or(0);

        // ❌ Enforce limit
        if current_count >= MAX_TX_PER_USER {
            panic!("Transaction limit exceeded for user");
        }

        // ✅ Increment count
        current_count += 1;
        counts.set(tx.sender.clone(), current_count);

        // Store transaction
        storage.set(tx.id, tx);

        env.storage().instance().set(&STORAGE_KEY, &storage);
        env.storage().instance().set(&USER_COUNT_KEY, &counts);
    }

    /// Optional helper: get export transactions (from previous issue)
    pub fn get_export_transactions(env: Env) -> Vec<Transaction> {
        let storage: Map<u64, Transaction> =
            env.storage().instance().get(&STORAGE_KEY).unwrap_or(Map::new(&env));

        let mut result: Vec<Transaction> = Vec::new(&env);

        for (_, tx) in storage.iter() {
            if tx.export {
                result.push_back(tx);
            }
        }

        result
    }
}