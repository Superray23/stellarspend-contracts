use crate::{
    FeeChangeType, FeeConfiguration, FeeContract, FeeContractClient, FeeHistoryEntry,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup_contract(env: &Env) -> (FeeContractClient, Address) {
    let admin = Address::generate(env);
    let contract = FeeContractClient::new(&env, env.register_contract(None, FeeContract {}));
    
    // Initialize with custom thresholds
    contract.initialize(
        &admin,
        &Some(100_000_000i128),   // Critical: 100 XLM
        &Some(500_000_000i128),   // High: 500 XLM
        &Some(1_000_000_000i128), // Medium: 1000 XLM
        &Some(5_000_000_000i128), // Low: 5000 XLM
    );
    
    (contract, admin)
}

#[test]
fn test_initialization_recorded_in_history() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Should have 1 history entry from initialization
    assert_eq!(contract.get_history_count(), 1);
    
    // Get the history entry
    let entry = contract.get_history_entry(&1);
    assert!(entry.is_some());
    
    let entry = entry.unwrap();
    assert_eq!(entry.entry_id, 1);
    assert_eq!(entry.changed_by, admin);
    assert_eq!(entry.change_type, FeeChangeType::Initialization);
    assert_eq!(entry.new_config.admin, admin);
    assert_eq!(entry.new_config.critical_threshold, 100_000_000);
    assert_eq!(entry.new_config.high_threshold, 500_000_000);
    assert_eq!(entry.new_config.medium_threshold, 1_000_000_000);
    assert_eq!(entry.new_config.low_threshold, 5_000_000_000);
    assert_eq!(entry.new_config.fee_pool, 0);
}

#[test]
fn test_update_thresholds_recorded_in_history() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Update thresholds
    contract.update_thresholds(
        &admin,
        &Some(200_000_000i128),   // New critical: 200 XLM
        &Some(600_000_000i128),   // New high: 600 XLM
        &Some(1_200_000_000i128), // New medium: 1200 XLM
        &Some(6_000_000_000i128), // New low: 6000 XLM
    );
    
    // Should have 2 history entries now
    assert_eq!(contract.get_history_count(), 2);
    
    // Get the second entry
    let entry = contract.get_history_entry(&2);
    assert!(entry.is_some());
    
    let entry = entry.unwrap();
    assert_eq!(entry.entry_id, 2);
    assert_eq!(entry.changed_by, admin);
    assert_eq!(entry.change_type, FeeChangeType::ThresholdUpdate);
    
    // Verify previous config
    assert_eq!(entry.previous_config.critical_threshold, 100_000_000);
    assert_eq!(entry.previous_config.high_threshold, 500_000_000);
    
    // Verify new config
    assert_eq!(entry.new_config.critical_threshold, 200_000_000);
    assert_eq!(entry.new_config.high_threshold, 600_000_000);
    assert_eq!(entry.new_config.medium_threshold, 1_200_000_000);
    assert_eq!(entry.new_config.low_threshold, 6_000_000_000);
}

#[test]
fn test_set_admin_recorded_in_history() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Set new admin
    let new_admin = Address::generate(&env);
    contract.set_admin(&admin, &new_admin);
    
    // Should have 2 history entries
    assert_eq!(contract.get_history_count(), 2);
    
    // Get the second entry
    let entry = contract.get_history_entry(&2);
    assert!(entry.is_some());
    
    let entry = entry.unwrap();
    assert_eq!(entry.entry_id, 2);
    assert_eq!(entry.changed_by, admin);
    assert_eq!(entry.change_type, FeeChangeType::AdminChange);
    
    // Verify admin changed
    assert_eq!(entry.previous_config.admin, admin);
    assert_eq!(entry.new_config.admin, new_admin);
}

#[test]
fn test_get_fee_history_returns_all_entries() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Make several changes
    let new_admin = Address::generate(&env);
    contract.set_admin(&admin, &new_admin);
    
    contract.update_thresholds(
        &new_admin,
        &Some(150_000_000i128),
        &Some(550_000_000i128),
        &Some(1_100_000_000i128),
        &Some(5_500_000_000i128),
    );
    
    // Get all history
    let history = contract.get_fee_history(&None, &None);
    
    // Should have 3 entries (init + admin change + threshold update)
    assert_eq!(history.len(), 3);
    
    // Verify order (newest first)
    assert_eq!(history.get(0).unwrap().entry_id, 3);
    assert_eq!(history.get(1).unwrap().entry_id, 2);
    assert_eq!(history.get(2).unwrap().entry_id, 1);
}

#[test]
fn test_get_fee_history_with_limit() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Make several changes
    contract.update_thresholds(
        &admin,
        &Some(150_000_000i128),
        &Some(550_000_000i128),
        &Some(1_100_000_000i128),
        &Some(5_500_000_000i128),
    );
    
    contract.update_thresholds(
        &admin,
        &Some(175_000_000i128),
        &Some(575_000_000i128),
        &Some(1_150_000_000i128),
        &Some(5_750_000_000i128),
    );
    
    // Get only last 2 entries
    let history = contract.get_fee_history(&None, &Some(2));
    
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().entry_id, 3);
    assert_eq!(history.get(1).unwrap().entry_id, 2);
}

#[test]
fn test_get_fee_history_with_start_id() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Make several changes
    contract.update_thresholds(&admin, &Some(150_000_000i128), &Some(550_000_000i128), &Some(1_100_000_000i128), &Some(5_500_000_000i128));
    contract.update_thresholds(&admin, &Some(175_000_000i128), &Some(575_000_000i128), &Some(1_150_000_000i128), &Some(5_750_000_000i128));
    contract.update_thresholds(&admin, &Some(200_000_000i128), &Some(600_000_000i128), &Some(1_200_000_000i128), &Some(6_000_000_000i128));
    
    // Get entries starting from ID 2
    let history = contract.get_fee_history(&Some(2), &None);
    
    // Should return entries 2 and 1 (going backwards from 2)
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().entry_id, 2);
    assert_eq!(history.get(1).unwrap().entry_id, 1);
}

#[test]
fn test_timestamps_are_recorded() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Get the history entry
    let entry = contract.get_history_entry(&1);
    assert!(entry.is_some());
    
    // Timestamp should be greater than 0
    let entry = entry.unwrap();
    assert!(entry.timestamp > 0);
}

#[test]
fn test_history_entry_contains_complete_config() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Update thresholds
    contract.update_thresholds(
        &admin,
        &Some(250_000_000i128),
        &Some(750_000_000i128),
        &Some(1_500_000_000i128),
        &Some(7_500_000_000i128),
    );
    
    // Get the history entry
    let entry = contract.get_history_entry(&2);
    assert!(entry.is_some());
    
    let entry = entry.unwrap();
    
    // Verify complete config is stored
    assert_eq!(entry.new_config.admin, admin);
    assert_eq!(entry.new_config.critical_threshold, 250_000_000);
    assert_eq!(entry.new_config.high_threshold, 750_000_000);
    assert_eq!(entry.new_config.medium_threshold, 1_500_000_000);
    assert_eq!(entry.new_config.low_threshold, 7_500_000_000);
    assert_eq!(entry.new_config.fee_pool, 0);
}

#[test]
fn test_empty_history_query() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract = FeeContractClient::new(&env, env.register_contract(None, FeeContract {}));
    
    // Try to get history before initialization (should fail as contract not initialized)
    // Instead, test with non-existent entry
    let entry = contract.get_history_entry(&999);
    assert!(entry.is_none());
}

#[test]
fn test_multiple_threshold_updates_tracked() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Make 5 threshold updates
    for i in 1..=5 {
        contract.update_thresholds(
            &admin,
            &Some(100_000_000 * (i as i128)),
            &Some(500_000_000 * (i as i128)),
            &Some(1_000_000_000 * (i as i128)),
            &Some(5_000_000_000 * (i as i128)),
        );
    }
    
    // Should have 6 entries (1 init + 5 updates)
    assert_eq!(contract.get_history_count(), 6);
        
    // Get all history
    let history = contract.get_fee_history(&None, &None);
    assert_eq!(history.len(), 6);
    
    // Verify each entry has correct type
    assert_eq!(history.get(5).unwrap().change_type, FeeChangeType::Initialization);
    for i in 0..5 {
        assert_eq!(history.get(i).unwrap().change_type, FeeChangeType::ThresholdUpdate);
    }
}

#[test]
fn test_history_tracks_admin_changes() {
    let env = Env::default();
    let (contract, admin) = setup_contract(&env);
    
    // Change admin multiple times
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    
    contract.set_admin(&admin, &admin2);
    contract.set_admin(&admin2, &admin3);
    
    // Should have 3 entries
    assert_eq!(contract.get_history_count(), 3);
    
    // Get history and verify admin changes
    let history = contract.get_fee_history(&None, &None);
    
    assert_eq!(history.get(0).unwrap().change_type, FeeChangeType::AdminChange);
    assert_eq!(history.get(0).unwrap().new_config.admin, admin3);
    
    assert_eq!(history.get(1).unwrap().change_type, FeeChangeType::AdminChange);
    assert_eq!(history.get(1).unwrap().new_config.admin, admin2);
}
