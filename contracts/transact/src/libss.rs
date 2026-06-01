#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol, Error};

#[derive(Clone)]
#[contracttype]
pub struct Transaction {
    pub id: Symbol,
    pub amount: i128,
    pub sender: Symbol,
    pub receiver: Symbol,
}

#[contracttype]
pub enum DataKey {
    Transaction(Symbol),
}

#[derive(Debug)]
pub enum TransactionError {
    AlreadyExists = 1,
}

impl From<TransactionError> for Error {
    fn from(e: TransactionError) -> Self {
        Error::from_contract_error(e as u32)
    }
}

#[contract]
pub struct TransactionContract;

#[contractimpl]
impl TransactionContract {
    /// Add a new transaction
    pub fn add_transaction(env: Env, tx: Transaction) -> Result<(), Error> {
        let key = DataKey::Transaction(tx.id.clone());

        // 🔴 PRE-CHECK: prevent duplicate insert
        if env.storage().instance().has(&key) {
            return Err(TransactionError::AlreadyExists.into());
        }

        // ✅ store transaction if not exists
        env.storage().instance().set(&key, &tx);

        Ok(())
    }

    /// Optional helper: fetch transaction
    pub fn get_transaction(env: Env, id: Symbol) -> Option<Transaction> {
        let key = DataKey::Transaction(id);
        env.storage().instance().get(&key)
    }
}