#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{UsersContract, UsersContractClient};

fn setup() -> (Env, UsersContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UsersContract);
    let client = UsersContractClient::new(&env, &contract_id);
    (env, client)
}

// --- user_exists ---

#[test]
fn user_exists_returns_false_when_not_registered() {
    let (env, client) = setup();
    let user = Address::generate(&env);
    assert!(!client.user_exists(&user));
}

#[test]
fn user_exists_returns_true_after_registration() {
    let (env, client) = setup();
    let user = Address::generate(&env);
    client.register_user(&user);
    assert!(client.user_exists(&user));
}

// --- register_user ---

#[test]
fn register_user_succeeds_for_new_user() {
    let (env, client) = setup();
    let user = Address::generate(&env);
    client.register_user(&user);
    assert!(client.user_exists(&user));
}

#[test]
fn register_user_multiple_distinct_users() {
    let (env, client) = setup();
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    client.register_user(&user_a);
    client.register_user(&user_b);
    assert!(client.user_exists(&user_a));
    assert!(client.user_exists(&user_b));
}

#[test]
#[should_panic(expected = "User already registered")]
fn register_user_panics_on_duplicate() {
    let (env, client) = setup();
    let user = Address::generate(&env);
    client.register_user(&user);
    client.register_user(&user); // second call must panic
}
