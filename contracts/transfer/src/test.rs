#![cfg(test)]

extern crate std;

use crate::{TransferContract, TransferContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use std::string::String as StdString;

fn setup_test() -> (Env, TransferContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(TransferContract, ());
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

    // Result should be Ok with a reference ID
    let ref_id = client.execute_transfer(&from, &to, &amount, &clean_desc);
    assert!(ref_id.len() > 0);
    // Reference IDs should start with "TXN-"
    let mut ref_id_bytes = std::vec![0u8; ref_id.len() as usize];
    ref_id.copy_into_slice(&mut ref_id_bytes);
    let ref_id_str = StdString::from_utf8(ref_id_bytes).unwrap_or_default();
    assert!(ref_id_str.starts_with("TXN-"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #10)")] // SharedError::InvalidInput = 10
fn test_invalid_characters_rejected() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    // Description containing invalid characters (e.g., emojis or unsupported symbols)
    let invalid_desc = String::from_str(&env, "Payment 🎉");

    // This should panic with the SharedError::InvalidInput error code (10)
    client.execute_transfer(&from, &to, &amount, &invalid_desc);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #10)")]
fn test_html_tags_rejected() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    let html_desc = String::from_str(&env, "<script>alert('xss')</script>");
    client.execute_transfer(&from, &to, &amount, &html_desc);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_empty_description_rejected() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount = 100;

    let empty_desc = String::from_str(&env, "");

    client.execute_transfer(&from, &to, &amount, &empty_desc);
}

#[test]
fn test_transfer_generates_unique_reference_ids() {
    let (env, client) = setup_test();
    let from = Address::generate(&env);
    let to1 = Address::generate(&env);
    let to2 = Address::generate(&env);
    let amount = 100;

    let desc = String::from_str(&env, "Payment");

    // Execute two transfers from the same sender
    let ref_id_1 = client.execute_transfer(&from, &to1, &amount, &desc);
    let ref_id_2 = client.execute_transfer(&from, &to2, &amount, &desc);

    // Reference IDs should be different for different transactions
    assert_ne!(ref_id_1, ref_id_2);
}
