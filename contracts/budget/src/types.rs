use soroban_sdk::{contracttype, Address};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Beneficiary {
    pub address: Address,
    pub percentage: u32, // percentage out of 100 (e.g. 50 for 50%)
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum BudgetRule {
    MaxAmount(i128),
    MinAmount(i128),
}
