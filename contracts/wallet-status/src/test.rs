use soroban_sdk::{Env, Address};
use crate::{set_wallet_status, get_wallet_status};
use crate::types::WalletStatus;

#[test]
fn test_wallet_status_active() {
    let env = Env::default();
    let wallet = Address::generate(&env);

    set_wallet_status(env.clone(), wallet.clone(), WalletStatus::Active);

    let status = get_wallet_status(env.clone(), wallet);

    assert_eq!(status, WalletStatus::Active);
}

#[test]
fn test_wallet_status_paused() {
    let env = Env::default();
    let wallet = Address::generate(&env);

    set_wallet_status(env.clone(), wallet.clone(), WalletStatus::Paused);

    let status = get_wallet_status(env.clone(), wallet);

    assert_eq!(status, WalletStatus::Paused);
}

#[test]
fn test_wallet_status_restricted() {
    let env = Env::default();
    let wallet = Address::generate(&env);

    set_wallet_status(env.clone(), wallet.clone(), WalletStatus::Restricted);

    let status = get_wallet_status(env.clone(), wallet);

    assert_eq!(status, WalletStatus::Restricted);
}