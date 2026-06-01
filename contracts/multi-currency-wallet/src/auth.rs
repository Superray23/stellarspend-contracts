#![no_std]

use soroban_sdk::{Address, Env};

/// Centralized auth/ownership helper
pub struct Auth;

impl Auth {
    /// 🔐 Ensure the caller is the owner of the resource
    pub fn require_owner(owner: &Address) {
        owner.require_auth();
    }

    /// 🔐 Ensure the caller matches the expected address
    pub fn require_auth(address: &Address) {
        address.require_auth();
    }

    /// 🔐 Validate ownership explicitly (useful for comparisons)
    pub fn assert_owner(owner: &Address, caller: &Address) {
        if owner != caller {
            panic!("Unauthorized: caller is not the owner");
        }
        caller.require_auth();
    }

    /// 🔐 Optional helper: require both auth + equality in one call
    pub fn require_owner_match(owner: &Address, caller: &Address) {
        caller.require_auth();

        if owner != caller {
            panic!("Unauthorized: owner mismatch");
        }
    }
}