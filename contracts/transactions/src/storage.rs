use crate::TransactionError;
use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, String, Symbol, Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum TransactionStatus {
    Pending = 0,
    Completed = 1,
    Failed = 2,
    Cancelled = 3,
}

pub const MAX_TRANSACTIONS_PER_USER: u32 = 1000;
pub const MIN_TRANSACTION_AMOUNT: i128 = 1; // Minimum 1 unit
pub const MAX_TRANSACTION_AMOUNT: i128 = 1_000_000_000_000; // Maximum 1 trillion units

#[derive(Clone)]
#[contracttype]
pub struct Transaction {
    pub id: Symbol,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub note: String,
    pub memo: String,
    pub tags: Vec<String>,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub tx_type: Symbol,
    pub is_public: bool,
    pub metadata: Map<Symbol, String>,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Admin address
    Admin,
    /// Transaction by ID
    Transaction(Symbol),
    /// User's transaction list (user address -> vector of transaction IDs)
    UserTransactions(Address),
    /// Global list of all transaction IDs
    AllTransactions,
    /// Transaction counter for generating unique IDs
    TransactionCounter,
}

/// Create a new transaction
pub fn create_transaction(
    env: &Env,
    from: Address,
    to: Address,
    amount: i128,
    note: String,
    memo: String,
    tags: Vec<String>,
    tx_type: Symbol,
    is_public: bool,
    metadata: Map<Symbol, String>,
) -> Transaction {
    let tx_id = crate::utils::generate_transaction_id(env);

    let mut user_txs: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::UserTransactions(from.clone()))
        .unwrap_or_else(|| Vec::new(env));

    if user_txs.len() >= MAX_TRANSACTIONS_PER_USER {
        panic_with_error!(env, TransactionError::TransactionLimitReached);
    }

    let transaction = Transaction {
        id: tx_id.clone(),
        from: from.clone(),
        to,
        amount,
        note: note.clone(),
        memo: memo.clone(),
        tags: tags.clone(),
        timestamp: env.ledger().timestamp(),
        status: TransactionStatus::Completed,
        tx_type,
        is_public,
        metadata,
    };

    // Store the transaction
    env.storage()
        .persistent()
        .set(&DataKey::Transaction(tx_id.clone()), &transaction);

    // Add to global transaction list
    let mut all_txs: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllTransactions)
        .unwrap_or_else(|| Vec::new(env));
    all_txs.push_back(tx_id.clone());
    env.storage()
        .persistent()
        .set(&DataKey::AllTransactions, &all_txs);

    // Add to user's transaction list
    user_txs.push_back(tx_id.clone());
    env.storage()
        .persistent()
        .set(&DataKey::UserTransactions(from), &user_txs);

    transaction
}

/// Update the amount for a transaction (only transaction owner can update)
pub fn update_transaction_amount(env: &Env, id: Symbol, caller: Address, new_amount: i128) -> bool {
    let mut transaction: Transaction = match env
        .storage()
        .persistent()
        .get(&DataKey::Transaction(id.clone()))
    {
        Some(tx) => tx,
        None => return false,
    };

    if transaction.from != caller {
        return false;
    }

    if new_amount < MIN_TRANSACTION_AMOUNT || new_amount > MAX_TRANSACTION_AMOUNT {
        return false;
    }

    transaction.amount = new_amount;
    env.storage()
        .persistent()
        .set(&DataKey::Transaction(id), &transaction);

    true
}

/// Get a transaction by ID
pub fn get_transaction(env: &Env, id: Symbol) -> Option<Transaction> {
    env.storage().persistent().get(&DataKey::Transaction(id))
}

/// Update transaction note (only transaction owner can update)
pub fn update_transaction_note(env: &Env, id: Symbol, caller: Address, new_note: String) -> bool {
    let mut transaction: Transaction = match env
        .storage()
        .persistent()
        .get(&DataKey::Transaction(id.clone()))
    {
        Some(tx) => tx,
        None => return false,
    };

    // Verify caller is the transaction owner
    if transaction.from != caller {
        return false;
    }

    // Update the note
    transaction.note = new_note.clone();

    // Store updated transaction
    env.storage()
        .persistent()
        .set(&DataKey::Transaction(id), &transaction);

    true
}

/// Update transaction status (only admin or transaction owner can update)
pub fn update_transaction_status(
    env: &Env,
    id: Symbol,
    caller: Address,
    new_status: TransactionStatus,
) -> bool {
    let mut transaction: Transaction = match env
        .storage()
        .persistent()
        .get(&DataKey::Transaction(id.clone()))
    {
        Some(tx) => tx,
        None => return false,
    };

    // Verify caller is the transaction owner or admin
    if transaction.from != caller {
        // Check if caller is admin
        let admin: Option<Address> = env.storage().instance().get(&crate::DataKey::Admin);
        if admin.is_none() || admin.unwrap() != caller {
            return false;
        }
    }

    // Update the status
    transaction.status = new_status;

    // Store updated transaction
    env.storage()
        .persistent()
        .set(&DataKey::Transaction(id), &transaction);

    true
}

/// Get transaction timestamp
pub fn get_transaction_timestamp(env: &Env, id: Symbol) -> Option<u64> {
    get_transaction(env, id).map(|tx| tx.timestamp)
}

/// Get all transactions for a user
pub fn get_user_transactions(env: &Env, user: Address) -> Vec<Transaction> {
    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::UserTransactions(user.clone()))
        .unwrap_or_else(|| Vec::new(env));

    let mut transactions = Vec::new(env);
    for tx_id in tx_ids.iter() {
        if let Some(tx) = get_transaction(env, tx_id) {
            transactions.push_back(tx);
        }
    }

    transactions
}

/// Get user transactions filtered by `tx_type` (e.g. `income`, `expense`).
pub fn get_user_transactions_filtered(
    env: &Env,
    user: Address,
    tx_type: Symbol,
) -> Vec<Transaction> {
    let transactions = get_user_transactions(env, user);
    let mut filtered = Vec::new(env);

    for tx in transactions.iter() {
        if tx.tx_type == tx_type {
            filtered.push_back(tx);
        }
    }

    filtered
}

/// Clear all transactions for a user (only user can perform this)
pub fn clear_user_transactions(env: &Env, user: Address) -> bool {
    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::UserTransactions(user.clone()))
        .unwrap_or_else(|| Vec::new(env));

    // Get current all transactions
    let all_txs: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllTransactions)
        .unwrap_or_else(|| Vec::new(env));

    // Remove all transactions and filter out from all_txs
    let mut new_all_txs = Vec::new(env);
    for all_tx_id in all_txs.iter() {
        let mut keep = true;
        for user_tx_id in tx_ids.iter() {
            if all_tx_id == user_tx_id {
                // Remove the transaction
                env.storage()
                    .persistent()
                    .remove(&DataKey::Transaction(user_tx_id));
                keep = false;
                break;
            }
        }
        if keep {
            new_all_txs.push_back(all_tx_id);
        }
    }

    // Update the global list
    env.storage()
        .persistent()
        .set(&DataKey::AllTransactions, &new_all_txs);

    // Clear user's transaction list
    env.storage()
        .persistent()
        .remove(&DataKey::UserTransactions(user));

    true
}

/// Get the last (most recent) transaction for a user
pub fn get_last_transaction(env: &Env, user: Address) -> Option<Transaction> {
    let transactions = get_user_transactions(env, user);

    if transactions.is_empty() {
        return None;
    }

    // Find the transaction with the latest timestamp
    let mut latest_tx = None;
    let mut latest_timestamp = 0u64;

    for tx in transactions.iter() {
        if tx.timestamp > latest_timestamp {
            latest_timestamp = tx.timestamp;
            latest_tx = Some(tx.clone());
        }
    }

    latest_tx
}

/// Get the total number of transactions recorded in the contract
pub fn get_total_transactions_count(env: &Env) -> u64 {
    let all_txs: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllTransactions)
        .unwrap_or_else(|| Vec::new(env));
    all_txs.len() as u64
}

/// Check if a transaction exists
pub fn transaction_exists(env: &Env, id: Symbol) -> bool {
    env.storage().persistent().has(&DataKey::Transaction(id))
}

/// Check if a user is the owner of a transaction
pub fn is_transaction_owner(env: &Env, id: Symbol, user: Address) -> bool {
    if let Some(transaction) = get_transaction(env, id) {
        transaction.from == user
    } else {
        false
    }
}

/// Delete a transaction record and remove it from the user's transaction list.
pub fn delete_transaction(env: &Env, id: Symbol) -> bool {
    let transaction: Transaction = match env
        .storage()
        .persistent()
        .get(&DataKey::Transaction(id.clone()))
    {
        Some(tx) => tx,
        None => return false,
    };

    let owner = transaction.from.clone();

    env.storage()
        .persistent()
        .remove(&DataKey::Transaction(id.clone()));

    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::UserTransactions(owner.clone()))
        .unwrap_or_else(|| Vec::new(env));

    let mut remaining: Vec<Symbol> = Vec::new(env);
    for tx_id in tx_ids.iter() {
        if tx_id != id {
            remaining.push_back(tx_id.clone());
        }
    }

    if remaining.is_empty() {
        env.storage()
            .persistent()
            .remove(&DataKey::UserTransactions(owner));
    } else {
        env.storage()
            .persistent()
            .set(&DataKey::UserTransactions(owner), &remaining);
    }

    true
}

/// Get transaction memo
pub fn get_transaction_memo(env: &Env, id: Symbol) -> Option<String> {
    get_transaction(env, id).map(|tx| tx.memo)
}

/// Set metadata key-value pairs for a transaction (only owner can update).
pub fn set_transaction_metadata(
    env: &Env,
    id: Symbol,
    caller: Address,
    metadata: Map<Symbol, String>,
) -> bool {
    let mut transaction: Transaction = match env
        .storage()
        .persistent()
        .get(&DataKey::Transaction(id.clone()))
    {
        Some(tx) => tx,
        None => return false,
    };

    if transaction.from != caller {
        return false;
    }

    transaction.metadata = metadata;
    env.storage()
        .persistent()
        .set(&DataKey::Transaction(id), &transaction);

    true
}

/// Get metadata key-value pairs for a transaction.
pub fn get_transaction_metadata(env: &Env, id: Symbol) -> Option<Map<Symbol, String>> {
    get_transaction(env, id).map(|tx| tx.metadata)
}

/// Get all transactions in the contract
pub fn get_all_transactions(env: &Env) -> Vec<Transaction> {
    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllTransactions)
        .unwrap_or_else(|| Vec::new(env));

    let mut transactions = Vec::new(env);
    for tx_id in tx_ids.iter() {
        if let Some(tx) = get_transaction(env, tx_id) {
            transactions.push_back(tx);
        }
    }

    transactions
}

/// Get the total income from all transactions
pub fn get_total_income(env: &Env) -> i128 {
    let all_txs = get_all_transactions(env);
    let mut total: i128 = 0;
    let income_symbol = Symbol::new(env, "income");

    for tx in all_txs.iter() {
        if tx.tx_type == income_symbol {
            total += tx.amount;
        }
    }

    total
}

/// Get the total expense from all transactions
pub fn get_total_expense(env: &Env) -> i128 {
    let all_txs = get_all_transactions(env);
    let mut total: i128 = 0;
    let expense_symbol = Symbol::new(env, "expense");

    for tx in all_txs.iter() {
        if tx.tx_type == expense_symbol {
            total += tx.amount;
        }
    }

    total
}

/// Get a paginated subset of all transactions.
///
/// - `offset`: number of transactions to skip (0-based)
/// - `limit`:  maximum number of transactions to return (capped at 100)
pub fn get_transactions_paginated(env: &Env, offset: u32, limit: u32) -> Vec<Transaction> {
    let limit = limit.min(100);
    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllTransactions)
        .unwrap_or_else(|| Vec::new(env));

    let mut transactions = Vec::new(env);
    let total = tx_ids.len();

    if offset >= total || limit == 0 {
        return transactions;
    }

    let end = (offset + limit).min(total);

    for i in offset..end {
        if let Some(tx_id) = tx_ids.get(i) {
            if let Some(tx) = get_transaction(env, tx_id) {
                transactions.push_back(tx);
            }
        }
    }

    transactions
}

/// Check if a transaction is a duplicate (same from, to, amount, and memo)
/// within the user's recent history (last 5 transactions).
pub fn is_duplicate_transaction(
    env: &Env,
    from: Address,
    to: Address,
    amount: i128,
    memo: String,
) -> bool {
    let tx_ids: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::UserTransactions(from))
        .unwrap_or_else(|| Vec::new(env));

    if tx_ids.is_empty() {
        return false;
    }

    // Check only the most recent 5 transactions for duplicates
    let start = if tx_ids.len() > 5 {
        tx_ids.len() - 5
    } else {
        0
    };

    for i in start..tx_ids.len() {
        if let Some(tx_id) = tx_ids.get(i) {
            if let Some(tx) = get_transaction(env, tx_id) {
                if tx.to == to && tx.amount == amount && tx.memo == memo {
                    return true;
                }
            }
        }
    }

    false
}
