#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol, Map};

const STORAGE_KEY: Symbol = Symbol::short("BUDGETS");

#[contract]
pub struct BudgetContract;

#[contractimpl]
impl BudgetContract {
    /// Set or update a user's budget
    pub fn update_budget(env: Env, user: Symbol, amount: i128) {
        let mut budgets: Map<Symbol, i128> =
            env.storage()
                .instance()
                .get(&STORAGE_KEY)
                .unwrap_or(Map::new(&env));

        // Update budget for user (overwrite or insert)
        budgets.set(user, amount);

        env.storage().instance().set(&STORAGE_KEY, &budgets);
    }

    /// Optional helper: get budget
    pub fn get_budget(env: Env, user: Symbol) -> i128 {
        let budgets: Map<Symbol, i128> =
            env.storage()
                .instance()
                .get(&STORAGE_KEY)
                .unwrap_or(Map::new(&env));

        budgets.get(user).unwrap_or(0)
    }
}