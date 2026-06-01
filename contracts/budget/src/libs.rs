#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Map};

// Storage key
const BUDGETS_KEY: &str = "BUDGETS";

// Default budget value (adjust as needed)
const DEFAULT_BUDGET: i128 = 1_000;

#[contract]
pub struct BudgetContract;

#[contractimpl]
impl BudgetContract {
    /// Register a user with a default budget
    pub fn register_user(env: Env, user: Address) {
        user.require_auth();

        let mut budgets: Map<Address, i128> =
            env.storage().persistent().get(&BUDGETS_KEY).unwrap_or(Map::new(&env));

        // Prevent overwriting existing user budget
        if budgets.contains_key(user.clone()) {
            panic!("User already initialized");
        }

        // ✅ Set default budget
        budgets.set(user, DEFAULT_BUDGET);

        env.storage().persistent().set(&BUDGETS_KEY, &budgets);
    }

    /// Get user budget
    pub fn get_budget(env: Env, user: Address) -> i128 {
        let budgets: Map<Address, i128> =
            env.storage().persistent().get(&BUDGETS_KEY).unwrap_or(Map::new(&env));

        budgets.get(user).unwrap_or(0)
    }
}