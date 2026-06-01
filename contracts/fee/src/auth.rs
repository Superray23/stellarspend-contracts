use crate::storage::{has_admin, read_admin, read_locked};
use crate::FeeContractError;
use shared::auth::require_admin_with_error;
use soroban_sdk::{panic_with_error, Address, Env};

/// Validates that the given address is the contract admin and has authorized the call.
///
/// Panics with `NotInitialized` if the admin is not set.
/// Panics with `Unauthorized` if the address is not the admin.
/// Panics if the address has not authorized the invocation.
pub fn require_admin(env: &Env, address: &Address) {
    if !has_admin(env) {
        panic_with_error!(env, FeeContractError::NotInitialized);
    }

    let admin = read_admin(env);
    require_admin_with_error(env, address, &admin, FeeContractError::Unauthorized);
}

/// Validates that the contract is not locked.
///
/// Panics with `Locked` if the contract is currently locked.
pub fn require_unlocked(env: &Env) {
    if read_locked(env) {
        panic_with_error!(env, FeeContractError::Locked);
    }
}
