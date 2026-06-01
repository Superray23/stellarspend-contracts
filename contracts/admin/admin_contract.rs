// src/admin.rs

use soroban_sdk::{panic_with_error, Address, Env};
use crate::storage::{bump_instance, DataKey};
use crate::types::Error;

/// Persist the admin address.
///
/// Private to this module — external code must go through `AdminContract::initialize`,
/// which enforces the one-time-only invariant.
pub(crate) fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    bump_instance(env);
}

/// Load the admin address and refresh the instance TTL.
///
/// Panics with `Error::NotInitialized` if the contract has never been initialized,
/// giving callers a typed error instead of a host trap.
pub fn get_admin(env: &Env) -> Address {
    let admin = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    bump_instance(env);
    admin
}

/// Return `true` when an admin has been set.
///
/// Cheaper than `get_admin` when you only need existence, not the address —
/// avoids a full deserialise.
pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Assert that `caller` is the stored admin.
///
/// Combines the storage read, the Soroban auth check, and the TTL bump in one
/// call so every admin-gated function reads identically:
///
/// ```rust
/// pub fn protected_action(env: Env, caller: Address) {
///     require_admin(&env, &caller);
///     // ... rest of logic
/// }
/// ```
pub fn require_admin(env: &Env, caller: &Address) {
    let admin = get_admin(env); // panics with NotInitialized if unset
    if &admin != caller {
        panic_with_error!(env, Error::Unauthorized);
    }
    caller.require_auth();
}