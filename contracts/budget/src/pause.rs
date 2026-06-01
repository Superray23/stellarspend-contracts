use soroban_sdk::{Env};

use crate::storage::DataKey;

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn require_not_paused(env: &Env) {
    if is_paused(env) {
        panic!("Contract is paused");
    }
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage()
        .instance()
        .set(&DataKey::Paused, &paused);
}