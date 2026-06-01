#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol, Vec, Map, Env as SorobanEnv};

#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    pub amount: i128,
    pub sender: Symbol,
    pub receiver: Symbol,
    pub export: bool,
    pub updated_at: u64, // ✅ new field
}

const STORAGE_KEY: Symbol = Symbol::short("TXS");

#[contract]
pub struct TransactionContract;

#[contractimpl]
impl TransactionContract {
    /// Add a new transaction
    pub fn add_transaction(env: Env, tx: Transaction) {
        let mut storage: Map<u64, Transaction> =
            env.storage().instance().get(&STORAGE_KEY).unwrap_or(Map::new(&env));

        storage.set(tx.id, tx);

        env.storage().instance().set(&STORAGE_KEY, &storage);
    }

    /// Update an existing transaction
    pub fn update_transaction(
        env: Env,
        id: u64,
        amount: Option<i128>,
        sender: Option<Symbol>,
        receiver: Option<Symbol>,
        export: Option<bool>,
    ) {
        let mut storage: Map<u64, Transaction> =
            env.storage().instance().get(&STORAGE_KEY).unwrap_or(Map::new(&env));

        if let Some(mut tx) = storage.get(id).clone() {
            // apply updates if provided
            if let Some(a) = amount {
                tx.amount = a;
            }
            if let Some(s) = sender {
                tx.sender = s;
            }
            if let Some(r) = receiver {
                tx.receiver = r;
            }
            if let Some(e) = export {
                tx.export = e;
            }

            // ✅ REQUIRED: update timestamp on every edit
            tx.updated_at = env.ledger().timestamp();

            storage.set(id, tx);
            env.storage().instance().set(&STORAGE_KEY, &storage);
        }
    }

    /// Fetch transactions marked for export (from previous task)
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