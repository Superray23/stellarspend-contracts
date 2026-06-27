#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal,
};

use crate::{BudgetContract, BudgetContractClient};

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

fn setup_env() -> (Env, Address, BudgetContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let contract_id = env.register_contract(None, BudgetContract);
    let client = BudgetContractClient::new(&env, &contract_id);

    client.initialize(&owner);

    (env, owner, client)
}

// ─────────────────────────────────────────────
// 1. Ownership: authorized operations succeed
// ─────────────────────────────────────────────

#[test]
fn test_owner_can_update_owner() {
    let (env, owner, client) = setup_env();
    let new_owner = Address::generate(&env);

    // Should succeed without panicking
    client.update_owner(&owner, &new_owner);

    assert_eq!(client.get_owner(), new_owner);
}

#[test]
fn test_owner_can_add_contributor() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);

    client.add_contributor(&owner, &contributor);

    assert!(client.is_contributor(&contributor));
}

#[test]
fn test_owner_can_remove_contributor() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);

    client.add_contributor(&owner, &contributor);
    client.remove_contributor(&owner, &contributor);

    assert!(!client.is_contributor(&contributor));
}

#[test]
fn test_owner_can_transfer_ownership() {
    let (env, owner, client) = setup_env();
    let new_owner = Address::generate(&env);

    client.transfer_ownership(&owner, &new_owner);

    assert_eq!(client.get_owner(), new_owner);
}

// ─────────────────────────────────────────────
// 2. Ownership: unauthorized operations fail
// ─────────────────────────────────────────────

#[test]
#[should_panic(expected = "unauthorized")]
fn test_non_owner_cannot_update_owner() {
    let (env, _owner, client) = setup_env();
    let attacker = Address::generate(&env);
    let new_owner = Address::generate(&env);

    // Disable blanket auth mocking so require_auth() calls are enforced
    env.set_auths(&[]);

    client.update_owner(&attacker, &new_owner);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_non_owner_cannot_add_contributor() {
    let (env, _owner, client) = setup_env();
    let attacker = Address::generate(&env);
    let victim = Address::generate(&env);

    env.set_auths(&[]);

    client.add_contributor(&attacker, &victim);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_non_owner_cannot_remove_contributor() {
    let (env, owner, client) = setup_env();
    let attacker = Address::generate(&env);
    let contributor = Address::generate(&env);

    // Owner legitimately adds a contributor first
    client.add_contributor(&owner, &contributor);

    // Attacker tries to remove them
    env.set_auths(&[]);
    client.remove_contributor(&attacker, &contributor);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_contributor_cannot_transfer_ownership() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client.add_contributor(&owner, &contributor);

    env.set_auths(&[]);
    client.transfer_ownership(&contributor, &new_owner);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_anonymous_cannot_transfer_ownership() {
    let (env, _owner, client) = setup_env();
    let anonymous = Address::generate(&env);
    let new_owner = Address::generate(&env);

    env.set_auths(&[]);
    client.transfer_ownership(&anonymous, &new_owner);
}

// ─────────────────────────────────────────────
// 3. Contributor permissions
// ─────────────────────────────────────────────

#[test]
fn test_contributor_can_perform_allowed_actions() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);

    client.add_contributor(&owner, &contributor);

    // Contributors should be able to make contributions
    client.contribute(&contributor, &1_000_000_i128);

    assert!(client.get_total_contributions() >= 1_000_000_i128);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_non_contributor_cannot_contribute() {
    let (env, _owner, client) = setup_env();
    let outsider = Address::generate(&env);

    env.set_auths(&[]);
    client.contribute(&outsider, &1_000_000_i128);
}

#[test]
fn test_removed_contributor_loses_permissions() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);

    client.add_contributor(&owner, &contributor);
    assert!(client.is_contributor(&contributor));

    client.remove_contributor(&owner, &contributor);
    assert!(!client.is_contributor(&contributor));
}

// ─────────────────────────────────────────────
// 4. Ownership transfer edge cases
// ─────────────────────────────────────────────

#[test]
fn test_ownership_transfer_revokes_old_owner_privileges() {
    let (env, owner, client) = setup_env();
    let new_owner = Address::generate(&env);

    client.transfer_ownership(&owner, &new_owner);

    // Old owner should no longer be recognized as owner
    assert_ne!(client.get_owner(), owner);
    assert_eq!(client.get_owner(), new_owner);
}

#[test]
fn test_new_owner_gains_full_privileges_after_transfer() {
    let (env, owner, client) = setup_env();
    let new_owner = Address::generate(&env);
    let contributor = Address::generate(&env);

    client.transfer_ownership(&owner, &new_owner);

    // New owner should be able to add contributors
    client.add_contributor(&new_owner, &contributor);
    assert!(client.is_contributor(&contributor));
}

#[test]
#[should_panic(expected = "invalid_address")]
fn test_cannot_transfer_ownership_to_zero_address() {
    let (env, owner, client) = setup_env();

    // Passing the contract's own address or an invalid address should fail
    let invalid = client.address.clone();
    client.transfer_ownership(&owner, &invalid);
}

// ─────────────────────────────────────────────
// 5. Authorization event checks
// ─────────────────────────────────────────────

#[test]
fn test_update_owner_requires_current_owner_auth() {
    let (env, owner, client) = setup_env();
    let new_owner = Address::generate(&env);

    client.update_owner(&owner, &new_owner);

    // Verify that the owner's signature was required
    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &owner),
        "owner auth must be recorded"
    );
}

#[test]
fn test_add_contributor_requires_owner_auth() {
    let (env, owner, client) = setup_env();
    let contributor = Address::generate(&env);

    client.add_contributor(&owner, &contributor);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &owner),
        "owner auth must be recorded for add_contributor"
    );
}