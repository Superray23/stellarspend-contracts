#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Ledger, LedgerInfo},
    Env, Symbol,
};
use std::format;

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });
    env
}

fn deploy_contract(env: &Env) -> ActivityFeedContractClient {
    let contract_id = env.register(ActivityFeedContract, ());
    ActivityFeedContractClient::new(env, &contract_id)
}

#[test]
fn test_record_event_increments_total_and_returns_sequence() {
    let env = setup_env();
    let client = deploy_contract(&env);

    let seq1 = client.record_event(&symbol_short!("deposit"));
    assert_eq!(seq1, 1);
    assert_eq!(client.total_events(), 1);

    let seq2 = client.record_event(&symbol_short!("withdraw"));
    assert_eq!(seq2, 2);
    assert_eq!(client.total_events(), 2);
}

#[test]
fn test_get_feed_returns_newest_first() {
    let env = setup_env();
    let client = deploy_contract(&env);

    client.record_event(&symbol_short!("a"));
    client.record_event(&symbol_short!("b"));
    client.record_event(&symbol_short!("c"));

    let feed = client.get_feed(&1, &10);
    assert_eq!(feed.len(), 3);
    assert_eq!(feed.get(0).unwrap().sequence, 3);
    assert_eq!(feed.get(1).unwrap().sequence, 2);
    assert_eq!(feed.get(2).unwrap().sequence, 1);
    assert_eq!(feed.get(0).unwrap().event_type, symbol_short!("c"));
    assert_eq!(feed.get(2).unwrap().event_type, symbol_short!("a"));
}

#[test]
fn test_get_feed_pagination() {
    let env = setup_env();
    let client = deploy_contract(&env);

    for i in 1..=55 {
        let event_type = Symbol::new(&env, &format!("event{}", i));
        client.record_event(&event_type);
    }

    let page1 = client.get_feed(&1, &20);
    assert_eq!(page1.len(), 20);
    assert_eq!(page1.get(0).unwrap().sequence, 55);
    assert_eq!(page1.get(19).unwrap().sequence, 36);

    let page2 = client.get_feed(&2, &20);
    assert_eq!(page2.len(), 20);
    assert_eq!(page2.get(0).unwrap().sequence, 35);
    assert_eq!(page2.get(19).unwrap().sequence, 16);

    let page3 = client.get_feed(&3, &20);
    assert_eq!(page3.len(), 15);
    assert_eq!(page3.get(0).unwrap().sequence, 15);
    assert_eq!(page3.get(14).unwrap().sequence, 1);
}

#[test]
fn test_get_feed_defaults_page_and_size() {
    let env = setup_env();
    let client = deploy_contract(&env);

    for i in 1..=5 {
        let event_type = Symbol::new(&env, &format!("event{}", i));
        client.record_event(&event_type);
    }

    let page_zero = client.get_feed(&0, &0);
    assert_eq!(page_zero.len(), 5);
    assert_eq!(page_zero.get(0).unwrap().sequence, 5);
}
