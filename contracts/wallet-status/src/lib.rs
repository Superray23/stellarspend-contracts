use soroban_sdk::{Env, Address};

mod types;
use types::WalletStatus;

#[contracttype]
pub enum DataKey {
    WalletStatus(Address),
}

// Get wallet status
pub fn get_wallet_status(env: Env, wallet: Address) -> WalletStatus {
    env.storage()
        .instance()
        .get(&DataKey::WalletStatus(wallet))
        .unwrap_or(WalletStatus::Active)
}

// Optional setter (if needed internally/admin use)
pub fn set_wallet_status(env: Env, wallet: Address, status: WalletStatus) {
    env.storage()
        .instance()
        .set(&DataKey::WalletStatus(wallet), &status);
}