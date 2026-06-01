#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    TotalPenalties,
    TotalFees,
    TotalRewards,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TreasuryError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
}

impl From<TreasuryError> for soroban_sdk::Error {
    fn from(value: TreasuryError) -> Self {
        soroban_sdk::Error::from_contract_error(value as u32)
    }
}

pub struct TreasuryEvents;
impl TreasuryEvents {
    pub fn penalty_received(env: &Env, amount: i128) {
        let topics = (symbol_short!("treasury"), symbol_short!("penalty"));
        env.events().publish(topics, (amount, env.ledger().timestamp()));
    }

    pub fn fee_received(env: &Env, amount: i128) {
        let topics = (symbol_short!("treasury"), symbol_short!("fee"));
        env.events().publish(topics, (amount, env.ledger().timestamp()));
    }

    pub fn reward_received(env: &Env, amount: i128) {
        let topics = (symbol_short!("treasury"), symbol_short!("reward"));
        env.events().publish(topics, (amount, env.ledger().timestamp()));
    }
}

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, TreasuryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalPenalties, &0i128);
        env.storage().instance().set(&DataKey::TotalFees, &0i128);
        env.storage().instance().set(&DataKey::TotalRewards, &0i128);
    }

    pub fn credit_penalty(env: Env, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, TreasuryError::InvalidAmount);
        }
        let mut total: i128 = env.storage().instance().get(&DataKey::TotalPenalties).unwrap_or(0);
        total = total.checked_add(amount).unwrap_or_else(|| panic_with_error!(&env, TreasuryError::InvalidAmount));
        env.storage().instance().set(&DataKey::TotalPenalties, &total);
        TreasuryEvents::penalty_received(&env, amount);
    }

    pub fn credit_fee(env: Env, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, TreasuryError::InvalidAmount);
        }
        let mut total: i128 = env.storage().instance().get(&DataKey::TotalFees).unwrap_or(0);
        total = total.checked_add(amount).unwrap_or_else(|| panic_with_error!(&env, TreasuryError::InvalidAmount));
        env.storage().instance().set(&DataKey::TotalFees, &total);
        TreasuryEvents::fee_received(&env, amount);
    }

    pub fn credit_reward(env: Env, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, TreasuryError::InvalidAmount);
        }
        let mut total: i128 = env.storage().instance().get(&DataKey::TotalRewards).unwrap_or(0);
        total = total.checked_add(amount).unwrap_or_else(|| panic_with_error!(&env, TreasuryError::InvalidAmount));
        env.storage().instance().set(&DataKey::TotalRewards, &total);
        TreasuryEvents::reward_received(&env, amount);
    }

    pub fn get_total_penalties(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalPenalties).unwrap_or(0)
    }

    pub fn get_total_fees(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalFees).unwrap_or(0)
    }

    pub fn get_total_rewards(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalRewards).unwrap_or(0)
    }

    pub fn get_total_reserve(env: Env) -> i128 {
        let p: i128 = env.storage().instance().get(&DataKey::TotalPenalties).unwrap_or(0);
        let f: i128 = env.storage().instance().get(&DataKey::TotalFees).unwrap_or(0);
        let r: i128 = env.storage().instance().get(&DataKey::TotalRewards).unwrap_or(0);
        p.checked_add(f).unwrap_or(0).checked_add(r).unwrap_or(0)
    }
}
