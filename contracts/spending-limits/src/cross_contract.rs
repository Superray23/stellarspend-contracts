//! Whitelist storage keys shared with cross-contract spending authorization.

use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Whitelist(Address),
}
