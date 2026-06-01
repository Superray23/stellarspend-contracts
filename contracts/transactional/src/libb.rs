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

    /// Add transaction (no duplicates)
    pub fn add_transaction(env: Env, tx: Transaction) -> Result<(), Error> {
        let key = DataKey::Transaction(tx.id.clone());

        if env.storage().instance().has(&key) {
            return Err(TransactionError::AlreadyExists.into());
        }

        env.storage().instance().set(&key, &tx);

        let mut counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);

        counter += 1;

        env.storage()
            .instance()
            .set(&DataKey::TxSeq(counter), &tx.id);

        if counter == 1 {
            env.storage()
                .instance()
                .set(&DataKey::FirstTx, &tx.id);
        }

        env.storage().instance().set(&DataKey::Counter, &counter);

        Ok(())
    }

    /// Get earliest transaction
    pub fn get_earliest_transaction(env: Env) -> Option<Transaction> {
        let first_id: Option<Symbol> =
            env.storage().instance().get(&DataKey::FirstTx);

        match first_id {
            Some(id) => env.storage().instance().get(&DataKey::Transaction(id)),
            None => None,
        }
    }

    /// 📊 Calculate average transaction amount
    pub fn get_average_transaction_amount(env: Env) -> i128 {
        let counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);

        if counter == 0 {
            return 0;
        }

        let mut sum: i128 = 0;

        let mut i: u32 = 1;
        while i <= counter {
            let tx_id: Option<Symbol> =
                env.storage().instance().get(&DataKey::TxSeq(i));

            if let Some(id) = tx_id {
                if let Some(tx) =
                    env.storage().instance().get::<DataKey, Transaction>(&DataKey::Transaction(id))
                {
                    sum += tx.amount;
                }
            }

            i += 1;
        }

        sum / (counter as i128)
    }

    /// Optional fetch by id
    pub fn get_transaction(env: Env, id: Symbol) -> Option<Transaction> {
        let key = DataKey::Transaction(id);
        env.storage().instance().get(&key)
    }
}