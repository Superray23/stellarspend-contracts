use soroban_sdk::{contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendingUpdatedEvent {
    pub user: Address,
    pub category: Symbol,
    pub amount: i128,
    pub timestamp: u64,
}

pub fn emit_spending_updated(env: &Env, user: Address, category: Symbol, amount: i128) {
    let event = SpendingUpdatedEvent {
        user,
        category,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((Symbol::new(env, "spending_updated"),), event);
}
