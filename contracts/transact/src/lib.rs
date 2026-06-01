#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol, Vec, Map};

#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    pub amount: i128,
    pub sender: Symbol,
    pub receiver: Symbol,
    pub export: bool, // <-- flag used for filtering
}

const STORAGE_KEY: Symbol = Symbol::short("TXS");

#[contract]
pub struct TransactionContract;

#[contractimpl]
impl TransactionContract {
    /// Store a transaction (helper for completeness)
    pub fn add_transaction(env: Env, tx: Transaction) {
        let mut storage: Map<u64, Transaction> =
            env.storage().instance().get(&STORAGE_KEY).unwrap_or(Map::new(&env));

        storage.set(tx.id, tx);

        env.storage().instance().set(&STORAGE_KEY, &storage);
    }

    /// ✅ Core Requirement:
    /// Fetch transactions marked for export
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