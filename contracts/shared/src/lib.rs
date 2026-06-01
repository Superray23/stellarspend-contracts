#![no_std]

extern crate alloc;

use soroban_sdk::{contracttype, Address, Env, String};

pub mod assets;
pub mod auth;
pub mod errors;
pub mod sanitizer;
pub mod utils;
pub mod validation;

pub use errors::SharedError;

pub const SHARED_VERSION: &str = "0.1.0";

pub fn get_version(env: Env) -> String {
    String::from_str(&env, SHARED_VERSION)
}

/// Health check function that returns true if the contract is active.
pub fn health_check() -> bool {
    true
}

#[contracttype]
#[derive(Clone)]
pub enum SharedDataKey {
    Admin,
}

/// Returns the current contract owner/admin address.
pub fn get_contract_owner(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&SharedDataKey::Admin)
        .expect("Contract owner not initialized")
}

/// Updates the contract owner/admin address.
/// Only the current owner can perform this action.
pub fn update_contract_owner(env: &Env, new_owner: Address) {
    let current_owner: Address = get_contract_owner(env);
    current_owner.require_auth();
    env.storage()
        .instance()
        .set(&SharedDataKey::Admin, &new_owner);
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod tests {
    use super::get_version;
    use soroban_sdk::{Env, String};

    #[test]
    fn returns_shared_version() {
        let env = Env::default();
        let version = get_version(env);
        assert_eq!(version, String::from_str(&Env::default(), "0.1.0"));
    }
}
