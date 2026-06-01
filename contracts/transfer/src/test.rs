#![cfg(test)]

use crate::{TransferContract, TransferContractClient};
use shared::errors::SharedError;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup_test() -> (Env, TransferContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, TransferContract);
    let client = TransferContractClient::new(&env, &contract_id);
    (env, client)
}

#[test]
fn test_clean_description_passes() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    // Clean description remains unchanged and execution passes
    let clean_desc = String::from_str(&env, "Payment for dinner.");

    // Result should be Ok(())
    client.execute_transfer(&from, &to, &amount, &clean_desc);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")] // SharedError::InvalidInput = 3
fn test_invalid_characters_rejected() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    // Description containing invalid characters (e.g., emojis or unsupported symbols)
    let invalid_desc = String::from_str(&env, "Payment 🎉");

    // This should panic with the SharedError::InvalidInput error code (3)
    client.execute_transfer(&from, &to, &amount, &invalid_desc);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_html_tags_rejected() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    let html_desc = String::from_str(&env, "<script>alert('xss')</script>");
    client.execute_transfer(&from, &to, &amount, &html_desc);
}

#[test]
fn test_empty_description_passes() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    let empty_desc = String::from_str(&env, "");

    // Empty strings shouldn't fail
    client.execute_transfer(&from, &to, &amount, &empty_desc);
}
