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
    TxSeq(u32),
    FirstTx,
    Counter,
}

#[derive(Debug)]
pub enum TransactionError {
    AlreadyExists = 1,
    NotFound = 2,
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
    /// Add a transaction (no duplicates)
    pub fn add_transaction(env: Env, tx: Transaction) -> Result<(), Error> {
        let key = DataKey::Transaction(tx.id.clone());

        // prevent duplicates
        if env.storage().instance().has(&key) {
            return Err(TransactionError::AlreadyExists.into());
        }

        // store transaction
        env.storage().instance().set(&key, &tx);

        // get counter
        let mut counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);

        counter += 1;

        // store sequence mapping
        env.storage()
            .instance()
            .set(&DataKey::TxSeq(counter), &tx.id);

        // set first transaction if this is first insert
        if counter == 1 {
            env.storage()
                .instance()
                .set(&DataKey::FirstTx, &tx.id);
        }

        // update counter
        env.storage().instance().set(&DataKey::Counter, &counter);

        Ok(())
    }

    /// 🟢 Fetch earliest transaction (FIRST ENTRY)
    pub fn get_earliest_transaction(env: Env) -> Option<Transaction> {
        let first_id: Option<Symbol> = env
            .storage()
            .instance()
            .get(&DataKey::FirstTx);

        match first_id {
            Some(id) => {
                let key = DataKey::Transaction(id);
                env.storage().instance().get(&key)
            }
            None => None,
        }
    }

    /// Optional: fetch by id
    pub fn get_transaction(env: Env, id: Symbol) -> Option<Transaction> {
        let key = DataKey::Transaction(id);
        env.storage().instance().get(&key)
    }
}