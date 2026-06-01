#![no_std]

use soroban_sdk::{Env, Symbol, Vec, IntoVal};

/// Centralized event emitter helper
pub struct Events;

impl Events {
    /// Generic event emitter
    pub fn emit<T: IntoVal<Env, soroban_sdk::Val>>(
        env: &Env,
        topic: Symbol,
        data: T,
    ) {
        env.events().publish((topic,), data);
    }

    /// 🔹 User registered event
    pub fn user_registered(env: &Env, user: &soroban_sdk::Address) {
        env.events().publish(
            (Symbol::new(env, "user_registered"), user.clone()),
            (),
        );
    }

    /// 🔹 Budget initialized event
    pub fn budget_initialized(env: &Env, user: &soroban_sdk::Address, amount: i128) {
        env.events().publish(
            (Symbol::new(env, "budget_initialized"), user.clone()),
            amount,
        );
    }

    /// 🔹 Generic key-value style event
    pub fn key_value<T: IntoVal<Env, soroban_sdk::Val>>(
        env: &Env,
        key: Symbol,
        value: T,
    ) {
        env.events().publish((key,), value);
    }

    /// 🔹 Multi-topic event example (useful for indexing)
    pub fn emit_with_topics<T: IntoVal<Env, soroban_sdk::Val>>(
        env: &Env,
        topics: Vec<Symbol>,
        data: T,
    ) {
        env.events().publish(topics, data);
    }
}