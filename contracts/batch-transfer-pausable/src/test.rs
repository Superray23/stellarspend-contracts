//! Integration tests for the pausable batch transfer contract

#![cfg(test)]

use crate::{
    BatchTransferContract, BatchTransferContractClient, BatchTransferError, TransferRequest,
    TransferResult,
};
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger},
    Address, Env, Vec,
};

/// Setup test environment with contract deployed and initialized
fn setup_test_env() -> (Env, Address, BatchTransferContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.sequence_number = 12345;
    });

    // Deploy contract
    let contract_id = env.register(BatchTransferContract, ());
    let client = BatchTransferContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, admin, client)
}

/// Helper to create a transfer request
fn create_transfer_request(env: &Env, to: Address, amount: u128) -> TransferRequest {
    TransferRequest { to, amount }
}

// ─── Initialization Tests ─────────────────────────────────────────────────────

#[test]
fn test_initialize_contract() {
    let (env, admin, client) = setup_test_env();

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.is_paused(), false);
    assert_eq!(client.get_total_transfers(), 0);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_cannot_initialize_twice() {
    let (env, admin, client) = setup_test_env();

    let new_admin = Address::generate(&env);
    client.initialize(&new_admin);
}

// ─── Pause/Resume Tests ───────────────────────────────────────────────────────

#[test]
fn test_pause_contract() {
    let (_env, admin, client) = setup_test_env();

    assert_eq!(client.is_paused(), false);

    // Pause the contract
    client.pause(&admin);

    assert_eq!(client.is_paused(), true);
}

#[test]
fn test_resume_contract() {
    let (_env, admin, client) = setup_test_env();

    // Pause first
    client.pause(&admin);
    assert_eq!(client.is_paused(), true);

    // Resume
    client.resume(&admin);
    assert_eq!(client.is_paused(), false);
}

#[test]
fn test_multiple_pause_resume_cycles() {
    let (_env, admin, client) = setup_test_env();

    assert_eq!(client.is_paused(), false);

    // Cycle 1
    client.pause(&admin);
    assert_eq!(client.is_paused(), true);
    client.resume(&admin);
    assert_eq!(client.is_paused(), false);

    // Cycle 2
    client.pause(&admin);
    assert_eq!(client.is_paused(), true);
    client.resume(&admin);
    assert_eq!(client.is_paused(), false);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_pause() {
    let (env, _admin, client) = setup_test_env();

    let non_admin = Address::generate(&env);

    // Non-admin tries to pause
    client.pause(&non_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_resume() {
    let (env, admin, client) = setup_test_env();

    let non_admin = Address::generate(&env);

    // Pause as admin
    client.pause(&admin);

    // Non-admin tries to resume
    client.resume(&non_admin);
}

// ─── Batch Transfer When Paused Tests ────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_transfer_rejected_when_paused() {
    let (env, admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Pause the contract
    client.pause(&admin);

    // Try to execute batch transfer - should fail
    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient, 1000u128));

    client.batch_transfer(&caller, &requests);
}

#[test]
fn test_batch_transfer_succeeds_when_not_paused() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Transfer while not paused
    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient.clone(), 1000u128));

    let result = client.batch_transfer(&caller, &requests);

    assert_eq!(result.total_requests, 1);
    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);
}

#[test]
fn test_pause_after_batch_transfer() {
    let (env, admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient = Address::generate(&env);

    // First transfer succeeds
    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient.clone(), 1000u128));
    let result1 = client.batch_transfer(&caller, &requests);
    assert_eq!(result1.successful, 1);

    // Pause
    client.pause(&admin);

    // Second transfer fails
    let mut requests2: Vec<TransferRequest> = Vec::new(&env);
    requests2.push_back(create_transfer_request(&env, recipient.clone(), 2000u128));
    
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.batch_transfer(&caller, &requests2);
    }));
    assert!(result.is_err());

    // Resume
    client.resume(&admin);

    // Third transfer succeeds
    let mut requests3: Vec<TransferRequest> = Vec::new(&env);
    requests3.push_back(create_transfer_request(&env, recipient.clone(), 3000u128));
    let result3 = client.batch_transfer(&caller, &requests3);
    assert_eq!(result3.successful, 1);
}

// ─── Batch Transfer Tests ─────────────────────────────────────────────────────

#[test]
fn test_batch_transfer_single_recipient() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient, 1000u128));

    let result = client.batch_transfer(&caller, &requests);

    assert_eq!(result.total_requests, 1);
    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);
    assert_eq!(client.get_total_transfers(), 1);
}

#[test]
fn test_batch_transfer_multiple_recipients() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient1, 1000u128));
    requests.push_back(create_transfer_request(&env, recipient2, 2000u128));
    requests.push_back(create_transfer_request(&env, recipient3, 3000u128));

    let result = client.batch_transfer(&caller, &requests);

    assert_eq!(result.total_requests, 3);
    assert_eq!(result.successful, 3);
    assert_eq!(result.failed, 0);
    assert_eq!(client.get_total_transfers(), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_batch_transfer_empty_batch() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);

    let requests: Vec<TransferRequest> = Vec::new(&env);
    client.batch_transfer(&caller, &requests);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_batch_transfer_exceeds_max_size() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);

    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    for i in 0..=100 {
        let recipient = Address::generate(&env);
        requests.push_back(create_transfer_request(&env, recipient, 1000u128 + i as u128));
    }

    // 101 requests exceeds MAX_BATCH_SIZE of 100
    client.batch_transfer(&caller, &requests);
}

#[test]
fn test_batch_transfer_invalid_amount_skipped() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient1, 1000u128));
    requests.push_back(create_transfer_request(&env, recipient2, 0u128)); // Invalid
    requests.push_back(create_transfer_request(&env, recipient3, 3000u128));

    let result = client.batch_transfer(&caller, &requests);

    assert_eq!(result.total_requests, 3);
    assert_eq!(result.successful, 2);
    assert_eq!(result.failed, 1);
}

// ─── Admin Function Tests ─────────────────────────────────────────────────────

#[test]
fn test_set_admin() {
    let (_env, admin, client) = setup_test_env();
    let new_admin = Address::generate(&_env);

    // Change admin
    client.set_admin(&admin, &new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_set_admin() {
    let (env, _admin, client) = setup_test_env();

    let non_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // Non-admin tries to set admin
    client.set_admin(&non_admin, &new_admin);
}

// ─── Event Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_pause_event_emitted() {
    let (env, admin, client) = setup_test_env();

    client.pause(&admin);

    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_resume_event_emitted() {
    let (env, admin, client) = setup_test_env();

    client.pause(&admin);
    client.resume(&admin);

    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_batch_transfer_event_emitted() {
    let (env, _admin, client) = setup_test_env();

    let caller = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut requests: Vec<TransferRequest> = Vec::new(&env);
    requests.push_back(create_transfer_request(&env, recipient, 1000u128));

    client.batch_transfer(&caller, &requests);

    let events = env.events().all();
    assert!(events.len() > 0);
}
