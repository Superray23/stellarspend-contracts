// src/contract.rs

use soroban_sdk::{contract, contractimpl, Address, Env};
use crate::admin::{get_admin, is_initialized, require_admin, set_admin};
use crate::types::Error;

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    /// Set the admin address. May only be called once.
    ///
    /// Subsequent calls panic with `Error::AlreadyInitialized` so the admin
    /// cannot be silently overwritten by a replay or a misconfigured deploy
    /// script.
    ///
    /// No authentication is required for the first call — the deployer is
    /// trusted to supply the correct address at deploy time. After this point
    /// every privileged action requires the admin to sign.
    pub fn initialize(env: Env, admin: Address) {
        if is_initialized(&env) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        set_admin(&env, &admin);
    }

    /// Transfer admin rights to a new address.
    ///
    /// Requires the current admin's signature. The new admin does not need
    /// to sign here — they accept implicitly. If you need explicit acceptance
    /// (two-step transfer), store a `PendingAdmin` key and add a `accept_admin`
    /// entry point.
    pub fn transfer_admin(env: Env, current_admin: Address, new_admin: Address) {
        require_admin(&env, &current_admin);
        set_admin(&env, &new_admin);
    }

    /// Return the stored admin address. No authentication required.
    pub fn get_admin(env: Env) -> Address {
        get_admin(&env)
    }

    /// Return whether the contract has been initialized.
    pub fn is_initialized(env: Env) -> bool {
        is_initialized(&env)
    }
}