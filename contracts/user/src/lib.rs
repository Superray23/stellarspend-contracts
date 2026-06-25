//! # Deprecated — use the `users` crate instead
//!
//! This crate is a legacy stub that predates the full implementation in
//! `contracts/users`. It exists for historical reference only and **must not**
//! receive new features or be depended upon by other contracts.
//!
//! See `docs/user-vs-users-crates.md` for the authoritative decision record.

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Map};

// Storage key for users map
const USERS_KEY: &str = "USERS";

#[contract]
pub struct UsersContract;

#[contractimpl]
impl UsersContract {
    /// Register a user (helper for testing / completeness)
    pub fn register_user(env: Env, user: Address) {
        user.require_auth();

        let mut users: Map<Address, bool> =
            env.storage().persistent().get(&USERS_KEY).unwrap_or(Map::new(&env));

        // Prevent duplicate registration
        if users.contains_key(user.clone()) {
            panic!("User already registered");
        }

        users.set(user, true);
        env.storage().persistent().set(&USERS_KEY, &users);
    }

    /// 🔍 Check if a user is already registered
    pub fn user_exists(env: Env, user: Address) -> bool {
        let users: Map<Address, bool> =
            env.storage().persistent().get(&USERS_KEY).unwrap_or(Map::new(&env));

        users.contains_key(user)
    }
}