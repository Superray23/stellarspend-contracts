#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

#[path = "../contracts/governance.rs"]
mod governance;

use governance::{GovernanceContract, GovernanceContractClient, Proposal};

fn setup_governance_contract() -> (Env, Address, GovernanceContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    // Set initial ledger timestamp to 1000 so we avoid underflow/overflow logic issues
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let contract_id = env.register(GovernanceContract, ());
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // Require 2 approvals
    client.initialize(&admin, &2);

    (env, admin, client)
}

#[test]
fn test_governance_initialization() {
    let (env, admin, client) = setup_governance_contract();

    // A query just to ensure it's initialized correctly
    let none = client.get_proposal(&1);
    assert!(none.is_none());
}

#[test]
fn test_proposal_creation() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400); // 1 day
    assert_eq!(prop_id, 1);

    let proposal = client.get_proposal(&prop_id).unwrap();
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.approvals, 0);
    assert_eq!(proposal.config_key, key);
    assert_eq!(proposal.config_value, val);
    assert_eq!(proposal.executed, false);
}

#[test]
fn test_voting_and_execution_success() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400);

    // Voter 1
    client.vote_proposal(&voter1, &prop_id);
    let prop_v1 = client.get_proposal(&prop_id).unwrap();
    assert_eq!(prop_v1.approvals, 1);

    // Voter 2
    client.vote_proposal(&voter2, &prop_id);
    let prop_v2 = client.get_proposal(&prop_id).unwrap();
    assert_eq!(prop_v2.approvals, 2);

    // Execute
    client.execute_proposal(&voter1, &prop_id); // voter1 executes it

    // Check execution
    let executed_prop = client.get_proposal(&prop_id).unwrap();
    assert_eq!(executed_prop.executed, true);

    let config_val = client.get_config(&key).unwrap();
    assert_eq!(config_val, val);
}

#[test]
#[should_panic]
fn test_double_voting_panics() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400);

    // Voter 1
    client.vote_proposal(&voter1, &prop_id);

    // Voter 1 votes again
    client.vote_proposal(&voter1, &prop_id);
}

#[test]
#[should_panic]
fn test_execution_without_enough_approvals_panics() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400);

    // Only 1 approval, needs 2
    client.vote_proposal(&voter1, &prop_id);

    client.execute_proposal(&proposer, &prop_id);
}

#[test]
#[should_panic]
fn test_voting_on_expired_proposal_panics() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let duration = 86400;
    let prop_id = client.create_proposal(&proposer, &key, &val, &duration);

    env.ledger().with_mut(|li| {
        li.timestamp += duration + 1;
    });

    client.vote_proposal(&voter1, &prop_id);
}

#[test]
#[should_panic]
fn test_execute_expired_proposal_panics() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let duration = 86400;
    let prop_id = client.create_proposal(&proposer, &key, &val, &duration);

    client.vote_proposal(&voter1, &prop_id);
    client.vote_proposal(&voter2, &prop_id);

    env.ledger().with_mut(|li| {
        li.timestamp += duration + 1;
    });

    client.execute_proposal(&voter1, &prop_id);
}

#[test]
#[should_panic]
fn test_execute_already_executed_proposal_panics() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let key = String::from_str(&env, "fee_rate");
    let val = String::from_str(&env, "500");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400);
    client.vote_proposal(&voter1, &prop_id);
    client.vote_proposal(&voter2, &prop_id);
    client.execute_proposal(&voter1, &prop_id);

    client.execute_proposal(&voter2, &prop_id);
}

#[test]
fn test_proposal_lifecycle_end_to_end() {
    let (env, admin, client) = setup_governance_contract();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let key = String::from_str(&env, "withdraw_limit");
    let val = String::from_str(&env, "10000");

    let prop_id = client.create_proposal(&proposer, &key, &val, &86400);
    assert!(!client.get_proposal(&prop_id).unwrap().executed);

    client.vote_proposal(&voter1, &prop_id);
    client.vote_proposal(&voter2, &prop_id);
    client.execute_proposal(&admin, &prop_id);

    assert!(client.get_proposal(&prop_id).unwrap().executed);
    assert_eq!(client.get_config(&key).unwrap(), val);
}
