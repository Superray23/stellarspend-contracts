use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub struct Goal {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub target_amount: i128,
    pub saved_amount: i128,

    // NEW FIELD
    pub priority: u32,
}