use soroban_sdk::{Env, Address};

use crate::storage::DataKey;

pub fn set_penalty_percent(env: &Env, percent: u32) {
    env.storage().instance().set(&DataKey::PenaltyPercent, &percent);
}

pub fn get_penalty_percent(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::PenaltyPercent)
        .unwrap_or(10)
}

pub fn set_treasury(env: &Env, treasury: Address) {
    env.storage().instance().set(&DataKey::Treasury, &treasury);
}

pub fn get_treasury(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Treasury)
        .expect("treasury not set")
}