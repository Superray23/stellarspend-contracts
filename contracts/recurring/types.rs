use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecurringExpense {
    pub id: u64,
    pub owner: Address,
    pub amount: i128,
    pub interval_days: u32,
    pub next_due: u64,
    pub active: bool,
}