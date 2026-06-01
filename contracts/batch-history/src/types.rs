use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionRecord {
    pub amount: i128,
    pub timestamp: u64,
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserHistory {
    pub user: Address,
    pub transactions: Vec<TransactionRecord>,
}
//pub struct BatchHistoryContract;
