#![cfg(test)]

use crate::fee::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    Address, Env, IntoVal, Symbol, TryFromVal, Vec,
};

// =============================================================================
// Test Setup
// =============================================================================

fn setup_contract() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    (env, admin, contract_id)
}

// =============================================================================
// PriorityLevel Tests
// =============================================================================

#[test]
fn test_priority_level_from_u32() {
    assert_eq!(PriorityLevel::from_u32(0), Some(PriorityLevel::Low));
    assert_eq!(PriorityLevel::from_u32(1), Some(PriorityLevel::Medium));
    assert_eq!(PriorityLevel::from_u32(2), Some(PriorityLevel::High));
    assert_eq!(PriorityLevel::from_u32(3), Some(PriorityLevel::Urgent));
    assert_eq!(PriorityLevel::from_u32(4), None);
    assert_eq!(PriorityLevel::from_u32(100), None);
}

#[test]
fn test_priority_level_to_u32() {
    assert_eq!(PriorityLevel::Low.to_u32(), 0);
    assert_eq!(PriorityLevel::Medium.to_u32(), 1);
    assert_eq!(PriorityLevel::High.to_u32(), 2);
    assert_eq!(PriorityLevel::Urgent.to_u32(), 3);
}

#[test]
fn test_priority_level_ordering() {
    assert!(PriorityLevel::Low < PriorityLevel::Medium);
    assert!(PriorityLevel::Medium < PriorityLevel::High);
    assert!(PriorityLevel::High < PriorityLevel::Urgent);
    assert!(PriorityLevel::Low < PriorityLevel::Urgent);
}

#[test]
fn test_priority_level_default() {
    assert_eq!(PriorityLevel::default(), PriorityLevel::Medium);
}

// =============================================================================
// PriorityFeeConfig Tests
// =============================================================================

#[test]
fn test_priority_fee_config_default() {
    let config = PriorityFeeConfig::default();

    // Default values should be ascending
    assert_eq!(config.low_multiplier_bps, 8000);
    assert_eq!(config.medium_multiplier_bps, 10000);
    assert_eq!(config.high_multiplier_bps, 15000);
    assert_eq!(config.urgent_multiplier_bps, 20000);
}

#[test]
fn test_priority_fee_config_is_valid() {
    // Valid: ascending order
    let valid_config = PriorityFeeConfig {
        low_multiplier_bps: 5000,
        medium_multiplier_bps: 10000,
        high_multiplier_bps: 15000,
        urgent_multiplier_bps: 20000,
    };
    assert!(valid_config.is_valid());

    // Valid: equal values allowed
    let equal_config = PriorityFeeConfig {
        low_multiplier_bps: 10000,
        medium_multiplier_bps: 10000,
        high_multiplier_bps: 10000,
        urgent_multiplier_bps: 10000,
    };
    assert!(equal_config.is_valid());
}

#[test]
fn test_priority_fee_config_is_invalid() {
    // Invalid: descending order
    let invalid_config = PriorityFeeConfig {
        low_multiplier_bps: 20000,
        medium_multiplier_bps: 15000,
        high_multiplier_bps: 10000,
        urgent_multiplier_bps: 5000,
    };
    assert!(!invalid_config.is_valid());

    // Invalid: high > urgent
    let invalid_config2 = PriorityFeeConfig {
        low_multiplier_bps: 8000,
        medium_multiplier_bps: 10000,
        high_multiplier_bps: 20000,
        urgent_multiplier_bps: 15000,
    };
    assert!(!invalid_config2.is_valid());
}

#[test]
fn test_priority_fee_config_get_multiplier() {
    let config = PriorityFeeConfig::default();

    assert_eq!(config.get_multiplier_bps(PriorityLevel::Low), 8000);
    assert_eq!(config.get_multiplier_bps(PriorityLevel::Medium), 10000);
    assert_eq!(config.get_multiplier_bps(PriorityLevel::High), 15000);
    assert_eq!(config.get_multiplier_bps(PriorityLevel::Urgent), 20000);
}

// =============================================================================
// Priority Fee Calculation Tests
// =============================================================================

#[test]
fn test_calculate_priority_fee_rate() {
    let config = PriorityFeeConfig::default();
    let base_rate = 1000u32; // 10%

    // Low: 1000 * 8000 / 10000 = 800 (8%)
    assert_eq!(
        calculate_priority_fee_rate(base_rate, PriorityLevel::Low, &config),
        800
    );

    // Medium: 1000 * 10000 / 10000 = 1000 (10%)
    assert_eq!(
        calculate_priority_fee_rate(base_rate, PriorityLevel::Medium, &config),
        1000
    );

    // High: 1000 * 15000 / 10000 = 1500 (15%)
    assert_eq!(
        calculate_priority_fee_rate(base_rate, PriorityLevel::High, &config),
        1500
    );

    // Urgent: 1000 * 20000 / 10000 = 2000 (20%)
    assert_eq!(
        calculate_priority_fee_rate(base_rate, PriorityLevel::Urgent, &config),
        2000
    );
}

#[test]
fn test_calculate_fee_with_priority() {
    let env = Env::default();
    let priority_config = PriorityFeeConfig::default();

    let config = FeeConfig {
        default_fee_rate: 500, // 5%
        windows: Vec::new(&env),
        priority_config,
    };

    let amount = 10_000i128;

    // Low: 5% * 0.8 = 4% => 10000 * 0.04 = 400
    let low_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Low);
    assert_eq!(low_fee, 400);

    // Medium: 5% * 1.0 = 5% => 10000 * 0.05 = 500
    let medium_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Medium);
    assert_eq!(medium_fee, 500);

    // High: 5% * 1.5 = 7.5% => 10000 * 0.075 = 750
    let high_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::High);
    assert_eq!(high_fee, 750);

    // Urgent: 5% * 2.0 = 10% => 10000 * 0.10 = 1000
    let urgent_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Urgent);
    assert_eq!(urgent_fee, 1000);
}

#[test]
fn test_priority_fees_scale_correctly() {
    let env = Env::default();
    let priority_config = PriorityFeeConfig::default();

    let config = FeeConfig {
        default_fee_rate: 1000, // 10%
        windows: Vec::new(&env),
        priority_config,
    };

    // Test that higher priority always results in higher fees
    let amount = 100_000i128;

    let low_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Low);
    let medium_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Medium);
    let high_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::High);
    let urgent_fee = calculate_fee_with_priority(&env, amount, &config, PriorityLevel::Urgent);

    // Verify ascending order
    assert!(low_fee < medium_fee);
    assert!(medium_fee < high_fee);
    assert!(high_fee < urgent_fee);

    // Verify specific values
    assert_eq!(low_fee, 8_000); // 10% * 0.8 = 8%
    assert_eq!(medium_fee, 10_000); // 10% * 1.0 = 10%
    assert_eq!(high_fee, 15_000); // 10% * 1.5 = 15%
    assert_eq!(urgent_fee, 20_000); // 10% * 2.0 = 20%
}

// =============================================================================
// Contract Tests
// =============================================================================

#[test]
fn test_contract_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);

    let config = client.get_fee_config();
    assert_eq!(config.default_fee_rate, 500);

    let priority_config = client.get_priority_config();

    // Initialize with 5% fee rate
    FeeContract::initialize(env.clone(), admin.clone(), 500);

    let config = FeeContract::get_fee_config(env.clone());
    assert_eq!(config.default_fee_rate, 500);

    let priority_config = FeeContract::get_priority_config(env.clone());
    assert_eq!(priority_config.low_multiplier_bps, 8000);
    assert_eq!(priority_config.medium_multiplier_bps, 10000);
    assert_eq!(priority_config.high_multiplier_bps, 15000);
    assert_eq!(priority_config.urgent_multiplier_bps, 20000);

    let events = env.events().all();
    let event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("init"))
        .expect("initialize event should be emitted");
    assert_eq!(event.topics.2, admin.clone());
    assert_eq!(event.topics.3, 500i128);
}

#[test]
fn test_set_priority_multipliers() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);
    client.set_priority_multipliers(&admin, &5000u32, &10000u32, &20000u32, &30000u32);

    let config = client.get_priority_config();

    FeeContract::initialize(env.clone(), admin.clone(), 500);

    // Set custom priority multipliers
    FeeContract::set_priority_multipliers(
        env.clone(),
        admin.clone(),
        5000,  // Low: 0.5x
        10000, // Medium: 1.0x
        20000, // High: 2.0x
        30000, // Urgent: 3.0x
    );

    let config = FeeContract::get_priority_config(env.clone());
    assert_eq!(config.low_multiplier_bps, 5000);
    assert_eq!(config.medium_multiplier_bps, 10000);
    assert_eq!(config.high_multiplier_bps, 20000);
    assert_eq!(config.urgent_multiplier_bps, 30000);
}

#[test]
#[should_panic]
fn test_set_invalid_priority_multipliers_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);
    // Descending order is invalid
    client.set_priority_multipliers(&admin, &30000u32, &20000u32, &10000u32, &5000u32);

    FeeContract::initialize(env.clone(), admin.clone(), 500);

    // Try to set invalid multipliers (descending order)
    FeeContract::set_priority_multipliers(
        env.clone(),
        admin.clone(),
        30000, // Low: 3.0x (higher than urgent!)
        20000, // Medium: 2.0x
        10000, // High: 1.0x
        5000,  // Urgent: 0.5x
    );
}

#[test]
fn test_get_priority_multiplier() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);

    assert_eq!(client.get_priority_multiplier(&PriorityLevel::Low), 8000);

    FeeContract::initialize(env.clone(), admin.clone(), 500);

    assert_eq!(
        FeeContract::get_priority_multiplier(env.clone(), PriorityLevel::Low),
        8000
    );
    assert_eq!(
        client.get_priority_multiplier(&PriorityLevel::Medium),
        10000
    );
    assert_eq!(client.get_priority_multiplier(&PriorityLevel::High), 15000);
    assert_eq!(
        client.get_priority_multiplier(&PriorityLevel::Urgent),
        20000
    );
}

#[test]
fn test_calculate_fee_with_priority_contract() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32); // 10% base rate
    let amount = 10_000i128;

    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::Low),
        800
    );
    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::Medium),
        1000
    );
    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::High),
        1500
    );
    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::Urgent),
        2000
    );

    FeeContract::initialize(env.clone(), admin.clone(), 1000); // 10% base rate

    let amount = 10_000i128;

    // Low: 10% * 0.8 = 8% => 800
    let low_fee = FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::Low);
    assert_eq!(low_fee, 800);

    // Medium: 10% * 1.0 = 10% => 1000
    let medium_fee =
        FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::Medium);
    assert_eq!(medium_fee, 1000);

    // High: 10% * 1.5 = 15% => 1500
    let high_fee =
        FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::High);
    assert_eq!(high_fee, 1500);

    // Urgent: 10% * 2.0 = 20% => 2000
    let urgent_fee =
        FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::Urgent);
    assert_eq!(urgent_fee, 2000);
}

#[test]
fn test_deduct_fee_with_priority() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32);
    let amount = 10_000i128;
    let (net, fee) = client.deduct_fee_with_priority(&payer, &amount, &PriorityLevel::High);

    assert_eq!(fee, 1500);
    assert_eq!(net, 8500);
    assert_eq!(client.get_total_collected(), 1500);
    assert_eq!(client.get_user_fees_accrued(&payer), 1500);

    FeeContract::initialize(env.clone(), admin.clone(), 1000); // 10% base rate

    let amount = 10_000i128;

    // Deduct with High priority (15% fee)
    let (net, fee) = FeeContract::deduct_fee_with_priority(
        env.clone(),
        payer.clone(),
        amount,
        PriorityLevel::High,
    );

    assert_eq!(fee, 1500);
    assert_eq!(net, 8500);
    assert_eq!(FeeContract::get_total_collected(env.clone()), 1500);
    assert_eq!(FeeContract::get_user_fees_accrued(env.clone(), payer.clone()), 1500);

    let events = env.events().all();
    let event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("deduct"))
        .expect("deduction event should be emitted");
    assert_eq!(event.topics.2, payer.clone());
    assert_eq!(event.topics.3, amount);
}

#[test]
fn test_priority_fee_with_bounds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32);
    client.set_fee_bounds(&admin, &500i128, &2000i128);

    // Low: 10%*0.8 on 5000 = 400 < min 500 -> clamped to 500
    assert_eq!(
        client.calculate_fee_with_priority(&5000i128, &PriorityLevel::Low),
        500
    );
    // Urgent: 10%*2.0 on 20000 = 4000 > max 2000 -> clamped to 2000
    assert_eq!(
        client.calculate_fee_with_priority(&20000i128, &PriorityLevel::Urgent),
        2000
    );

    FeeContract::initialize(env.clone(), admin.clone(), 1000);

    // Set fee bounds
    FeeContract::set_fee_bounds(env.clone(), admin.clone(), 500, 2000);

    // Low priority would calculate to 400 (below min)
    // Should be clamped to min 500
    let low_fee = FeeContract::calculate_fee_with_priority(env.clone(), 5000, PriorityLevel::Low);
    assert_eq!(low_fee, 500);

    // Urgent priority would calculate to 4000 (above max)
    // Should be clamped to max 2000
    let urgent_fee =
        FeeContract::calculate_fee_with_priority(env.clone(), 20000, PriorityLevel::Urgent);
    assert_eq!(urgent_fee, 2000);
}

#[test]
fn test_priority_fee_events() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32);
    client.set_priority_multipliers(&admin, &5000u32, &10000u32, &15000u32, &20000u32);

    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.1.get(0).and_then(|v| Symbol::try_from_val(&env, &v).ok()) == Some(symbol_short!("fee"))
            && e.1.get(1).and_then(|v| Symbol::try_from_val(&env, &v).ok())
                == Some(symbol_short!("pri_cfg"))
    }));

    FeeContract::initialize(env.clone(), admin.clone(), 1000);

    // Set priority multipliers
    FeeContract::set_priority_multipliers(env.clone(), admin.clone(), 5000, 10000, 15000, 20000);

    // Check event was emitted
    let events = env.events().all();
    assert!(events.iter().any(|e| e.topics.0 == symbol_short!("fee") 
        && e.topics.1 == symbol_short!("pri_cfg")));
    let event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("pri_cfg"))
        .expect("priority config event should be emitted");
    assert_eq!(event.topics.2, admin.clone());
    assert_eq!(event.topics.3, 10000i128);
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_zero_amount_fee() {
    let env = Env::default();
    let priority_config = PriorityFeeConfig::default();

    let config = FeeConfig {
        default_fee_rate: 1000,
        windows: Vec::new(&env),
        priority_config,
    };
    assert_eq!(
        calculate_fee_with_priority(&env, 0, &config, PriorityLevel::Urgent),
        0
    );
    assert_eq!(
        calculate_fee_with_priority(&env, -1000, &config, PriorityLevel::Urgent),
        0
    );

    // Zero amount should return 0 fee
    let fee = calculate_fee_with_priority(&env, 0, &config, PriorityLevel::Urgent);
    assert_eq!(fee, 0);

    // Negative amount should return 0 fee
    let fee = calculate_fee_with_priority(&env, -1000, &config, PriorityLevel::Urgent);
    assert_eq!(fee, 0);
}

#[test]
fn test_large_amount_with_priority() {
    let env = Env::default();
    let priority_config = PriorityFeeConfig::default();

    let config = FeeConfig {
        default_fee_rate: 100,
        windows: Vec::new(&env),
        priority_config,
    };
    let large_amount = 1_000_000_000_000i128;

    let large_amount = 1_000_000_000_000i128;

    // Urgent: 1% * 2.0 = 2% => 20_000_000_000
    assert_eq!(
        calculate_fee_with_priority(&env, large_amount, &config, PriorityLevel::Urgent),
        20_000_000_000
    );
}

#[test]
fn test_custom_priority_multipliers() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32);
    client.set_priority_multipliers(&admin, &2500u32, &10000u32, &25000u32, &50000u32);

    let amount = 10_000i128;
    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::Low),
        250
    );
    assert_eq!(
        client.calculate_fee_with_priority(&amount, &PriorityLevel::Urgent),
        5000
    );

    FeeContract::initialize(env.clone(), admin.clone(), 1000);

    // Set custom multipliers with larger spread
    FeeContract::set_priority_multipliers(
        env.clone(),
        admin.clone(),
        2500,  // Low: 0.25x (75% discount)
        10000, // Medium: 1.0x
        25000, // High: 2.5x (150% premium)
        50000, // Urgent: 5.0x (400% premium)
    );

    let amount = 10_000i128;

    // Low: 10% * 0.25 = 2.5% => 250
    let low_fee = FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::Low);
    assert_eq!(low_fee, 250);

    // Urgent: 10% * 5.0 = 50% => 5000
    let urgent_fee =
        FeeContract::calculate_fee_with_priority(env.clone(), amount, PriorityLevel::Urgent);
    assert_eq!(urgent_fee, 5000);
}

#[test]
fn test_multiple_priority_transactions() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1000u32);

    let (_, low_fee) = client.deduct_fee_with_priority(&payer, &10_000i128, &PriorityLevel::Low);
    let (_, med_fee) = client.deduct_fee_with_priority(&payer, &10_000i128, &PriorityLevel::Medium);
    let (_, high_fee) = client.deduct_fee_with_priority(&payer, &10_000i128, &PriorityLevel::High);
    let (_, urgent_fee) =
        client.deduct_fee_with_priority(&payer, &10_000i128, &PriorityLevel::Urgent);

    assert_eq!(low_fee, 800);
    assert_eq!(med_fee, 1000);
    assert_eq!(high_fee, 1500);
    assert_eq!(urgent_fee, 2000);
    assert_eq!(client.get_total_collected(), 5300);
    assert_eq!(client.get_user_fees_accrued(&payer), 5300);

    FeeContract::initialize(env.clone(), admin.clone(), 1000);

    // Execute transactions with different priorities
    let (_, low_fee) = FeeContract::deduct_fee_with_priority(
        env.clone(),
        payer.clone(),
        10_000,
        PriorityLevel::Low,
    );
    assert_eq!(low_fee, 800);

    let (_, medium_fee) = FeeContract::deduct_fee_with_priority(
        env.clone(),
        payer.clone(),
        10_000,
        PriorityLevel::Medium,
    );
    assert_eq!(medium_fee, 1000);

    let (_, high_fee) = FeeContract::deduct_fee_with_priority(
        env.clone(),
        payer.clone(),
        10_000,
        PriorityLevel::High,
    );
    assert_eq!(high_fee, 1500);

    let (_, urgent_fee) = FeeContract::deduct_fee_with_priority(
        env.clone(),
        payer.clone(),
        10_000,
        PriorityLevel::Urgent,
    );
    assert_eq!(urgent_fee, 2000);

    // Total collected should be sum of all fees
    assert_eq!(FeeContract::get_total_collected(env.clone()), 5300);

    // User fees accrued should match
    assert_eq!(
        FeeContract::get_user_fees_accrued(env.clone(), payer.clone()),
        5300
    );
}

// =============================================================================
// Asset-aware Fee Tests
// =============================================================================

#[test]
fn test_set_and_get_asset_fee_config() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);
    client.set_asset_fee_config(&admin, &asset, &200u32, &0i128, &0i128);
    FeeContract::set_asset_fee_config(
        env.clone(),
        admin.clone(),
        asset.clone(),
        200, // 2% fee rate
        0,   // no min fee
        0,   // no max fee
    );

    let config = client.get_asset_fee_config(&asset);
    assert_eq!(config.fee_rate, 200);
    assert_eq!(config.min_fee, 0);
    assert_eq!(config.max_fee, 0);
    assert_eq!(config.asset, asset);

    let events = env.events().all();
    let event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("ast_cfg"))
        .expect("asset config event should be emitted");
    assert_eq!(event.topics.2, admin.clone());
    assert_eq!(event.topics.3, 200i128);
}

#[test]
fn test_calculate_asset_fee_uses_asset_rate() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &200u32, &0i128, &0i128); // 2%

    assert_eq!(
        client.calculate_asset_fee(&asset, &10_000i128, &PriorityLevel::Medium),
        200
    );
    // Medium priority (1.0x multiplier): 2% of 10000 = 200
    let fee =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), 10_000, PriorityLevel::Medium);
    assert_eq!(fee, 200);
}

#[test]
fn test_calculate_asset_fee_falls_back_to_default() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let unconfigured_asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32); // 1% default
    assert_eq!(
        client.calculate_asset_fee(&unconfigured_asset, &10_000i128, &PriorityLevel::Medium),
        100
    );
}

#[test]
fn test_calculate_asset_fee_with_priority() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32);
    client.set_asset_fee_config(&admin, &asset, &100u32, &0i128, &0i128); // 1%
    let amount = 10_000i128;

    assert_eq!(
        client.calculate_asset_fee(&asset, &amount, &PriorityLevel::Low),
        80
    );
    assert_eq!(
        client.calculate_asset_fee(&asset, &amount, &PriorityLevel::Medium),
        100
    );
    assert_eq!(
        client.calculate_asset_fee(&asset, &amount, &PriorityLevel::High),
        150
    );
    assert_eq!(
        client.calculate_asset_fee(&asset, &amount, &PriorityLevel::Urgent),
        200
    );
    // Low: 1% * 0.8 = 0.8% => 80
    let low_fee =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), amount, PriorityLevel::Low);
    assert_eq!(low_fee, 80);

    // Medium: 1% * 1.0 = 1% => 100
    let medium_fee =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), amount, PriorityLevel::Medium);
    assert_eq!(medium_fee, 100);

    // High: 1% * 1.5 = 1.5% => 150
    let high_fee =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), amount, PriorityLevel::High);
    assert_eq!(high_fee, 150);

    // Urgent: 1% * 2.0 = 2% => 200
    let urgent_fee =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), amount, PriorityLevel::Urgent);
    assert_eq!(urgent_fee, 200);
}

#[test]
fn test_asset_fee_min_max_bounds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &50u32, &100i128, &500i128);

    // 0.5% of 1000 = 5 < min 100 -> clamped to 100
    assert_eq!(
        client.calculate_asset_fee(&asset, &1_000i128, &PriorityLevel::Medium),
        100
    );
    // 0.5% of 1_000_000 = 5000 > max 500 -> clamped to 500
    assert_eq!(
        client.calculate_asset_fee(&asset, &1_000_000i128, &PriorityLevel::Medium),
        500
    );
    // 0.5% of 1000 = 5, below min of 100 -> clamped to 100
    let fee_low =
        FeeContract::calculate_asset_fee(env.clone(), asset.clone(), 1_000, PriorityLevel::Medium);
    assert_eq!(fee_low, 100);

    // 0.5% of 1_000_000 = 5000, above max of 500 -> clamped to 500
    let fee_high = FeeContract::calculate_asset_fee(
        env.clone(),
        asset.clone(),
        1_000_000,
        PriorityLevel::Medium,
    );
    assert_eq!(fee_high, 500);
}

#[test]
fn test_deduct_asset_fee_tracks_balances_independently() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let xlm_asset = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &xlm_asset, &100u32, &0i128, &0i128); // 1%
    client.set_asset_fee_config(&admin, &usdc_asset, &200u32, &0i128, &0i128); // 2%

    let (xlm_net, xlm_fee) =
        client.deduct_asset_fee(&payer, &xlm_asset, &10_000i128, &PriorityLevel::Medium);
    let (usdc_net, usdc_fee) =
        client.deduct_asset_fee(&payer, &usdc_asset, &10_000i128, &PriorityLevel::Medium);

    assert_eq!(xlm_fee, 100);
    assert_eq!(xlm_net, 9_900);
    assert_eq!(usdc_fee, 200);
    assert_eq!(usdc_net, 9_800);
    assert_eq!(client.get_asset_fees_collected(&xlm_asset), 100);
    assert_eq!(client.get_asset_fees_collected(&usdc_asset), 200);
    assert_eq!(client.get_user_asset_fees_accrued(&payer, &xlm_asset), 100);
    assert_eq!(client.get_user_asset_fees_accrued(&payer, &usdc_asset), 200);
    assert_eq!(client.get_total_collected(), 300);
    // Deduct XLM fee: 1% of 10000 = 100
    let (xlm_net, xlm_fee) = FeeContract::deduct_asset_fee(
        env.clone(),
        payer.clone(),
        xlm_asset.clone(),
        10_000,
        PriorityLevel::Medium,
    );
    assert_eq!(xlm_fee, 100);
    assert_eq!(xlm_net, 9_900);

    // Deduct USDC fee: 2% of 10000 = 200
    let (usdc_net, usdc_fee) = FeeContract::deduct_asset_fee(
        env.clone(),
        payer.clone(),
        usdc_asset.clone(),
        10_000,
        PriorityLevel::Medium,
    );
    assert_eq!(usdc_fee, 200);
    assert_eq!(usdc_net, 9_800);

    // Per-asset balances tracked independently
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), xlm_asset.clone()),
        100
    );
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), usdc_asset.clone()),
        200
    );

    // Per-user per-asset fees tracked independently
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer.clone(), xlm_asset.clone()),
        100
    );
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer.clone(), usdc_asset.clone()),
        200
    );

    // Global total includes both
    assert_eq!(FeeContract::get_total_collected(env.clone()), 300);

    let events = env.events().all();
    let event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("ast_ded"))
        .expect("asset deduction event should be emitted");
    assert_eq!(event.topics.2, payer.clone());
    assert_eq!(event.topics.3, 10_000i128);
}

#[test]
fn test_multiple_users_per_asset_tracked_independently() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer_a = Address::generate(&env);
    let payer_b = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &100u32, &0i128, &0i128);

    client.deduct_asset_fee(&payer_a, &asset, &10_000i128, &PriorityLevel::Medium);
    client.deduct_asset_fee(&payer_b, &asset, &20_000i128, &PriorityLevel::Medium);

    assert_eq!(client.get_user_asset_fees_accrued(&payer_a, &asset), 100);
    assert_eq!(client.get_user_asset_fees_accrued(&payer_b, &asset), 200);
    assert_eq!(client.get_asset_fees_collected(&asset), 300);
    FeeContract::deduct_asset_fee(
        env.clone(),
        payer_a.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    );
    FeeContract::deduct_asset_fee(
        env.clone(),
        payer_b.clone(),
        asset.clone(),
        20_000,
        PriorityLevel::Medium,
    );

    // Each user's accrued fees tracked separately
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer_a.clone(), asset.clone()),
        100
    );
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer_b.clone(), asset.clone()),
        200
    );

    // Asset total is the sum
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), asset.clone()),
        300
    );
}

#[test]
#[should_panic]
fn test_set_asset_fee_config_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&non_admin, &asset, &200u32, &0i128, &0i128);
}

#[test]
#[should_panic]
fn test_set_asset_fee_config_invalid_rate() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &10_001u32, &0i128, &0i128);
}

// =============================================================================
// Batch Fee Tests
// =============================================================================

fn make_tx(
    payer: Address,
    asset: Address,
    amount: i128,
    priority: PriorityLevel,
) -> FeeTransaction {
    FeeTransaction {
        payer,
        asset,
        amount,
        priority,
    }
}

#[test]
fn test_calculate_batch_fees_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &200u32, &0i128, &0i128); // 2%

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    ));
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        5_000,
        PriorityLevel::High,
    ));

    let result = client.calculate_batch_fees(&txs);

    assert_eq!(result.results.get(0).unwrap().fee, 200);
    assert_eq!(result.results.get(0).unwrap().net_amount, 9_800);
    assert_eq!(result.results.get(1).unwrap().fee, 150);
    assert_eq!(result.results.get(1).unwrap().net_amount, 4_850);
    assert_eq!(result.total_fees, 350);

    // read-only: state must be unchanged
    assert_eq!(client.get_total_collected(), 0);
    assert_eq!(client.get_asset_fees_collected(&asset), 0);
    // simulate is read-only — global total must still be zero
    assert_eq!(FeeContract::get_total_collected(env.clone()), 0);
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), asset.clone()),
        0
    );
}

#[test]
fn test_deduct_batch_fees_aggregates_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer_a = Address::generate(&env);
    let payer_b = Address::generate(&env);
    let xlm = Address::generate(&env);
    let usdc = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &xlm, &100u32, &0i128, &0i128);
    client.set_asset_fee_config(&admin, &usdc, &200u32, &0i128, &0i128);

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    // payer_a pays 1% on 10_000 XLM at medium priority  -> fee 100
    txs.push_back(make_tx(
        payer_a.clone(),
        xlm.clone(),
        10_000,
        PriorityLevel::Medium,
    )); // fee 100
    // payer_b pays 2% on 5_000 USDC at medium priority  -> fee 100
    txs.push_back(make_tx(
        payer_b.clone(),
        usdc.clone(),
        5_000,
        PriorityLevel::Medium,
    )); // fee 100
    // payer_a pays 1% * 2.0 (urgent) on 10_000 XLM     -> fee 200
    txs.push_back(make_tx(
        payer_a.clone(),
        xlm.clone(),
        10_000,
        PriorityLevel::Urgent,
    )); // fee 200

    let result = client.deduct_batch_fees(&txs);

    assert_eq!(result.results.get(0).unwrap().fee, 100);
    assert_eq!(result.results.get(1).unwrap().fee, 100);
    assert_eq!(result.results.get(2).unwrap().fee, 200);
    assert_eq!(result.total_fees, 400);
    assert_eq!(client.get_asset_fees_collected(&xlm), 300);
    assert_eq!(client.get_asset_fees_collected(&usdc), 100);
    assert_eq!(client.get_user_asset_fees_accrued(&payer_a, &xlm), 300);
    assert_eq!(client.get_user_asset_fees_accrued(&payer_b, &usdc), 100);
    assert_eq!(client.get_total_collected(), 400);
}

#[test]
fn test_deduct_batch_fees_single_transaction() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &200u32); // 2% default
    // Per-asset balances tracked independently
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), xlm.clone()),
        300
    );
    assert_eq!(
        FeeContract::get_asset_fees_collected(env.clone(), usdc.clone()),
        100
    );

    // Per-user per-asset
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer_a.clone(), xlm.clone()),
        300
    );
    assert_eq!(
        FeeContract::get_user_asset_fees_accrued(env.clone(), payer_b.clone(), usdc.clone()),
        100
    );

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(FeeTransaction {
        payer: payer.clone(),
        asset: asset.clone(),
        amount: 5_000,
        priority: PriorityLevel::Medium,
    });

    let result = client.deduct_batch_fees(&txs);

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results.get(0).unwrap().fee, 100);
    assert_eq!(result.results.get(0).unwrap().net_amount, 4_900);
    assert_eq!(result.total_fees, 100);
    assert_eq!(client.get_total_collected(), 100);
}

// =============================================================================
// Issue #208 — Fee Fallback Mechanism Tests
// =============================================================================

#[test]
fn test_deduct_fee_with_fallback_success_path() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);

    let result = client.deduct_fee_with_fallback(&payer, &10_000i128, &PriorityLevel::Medium);

    assert_eq!(result.status, FeeOperationStatus::Success);
    assert_eq!(result.fee_charged, 100);
    assert_eq!(result.net_amount, 9_900);
    assert_eq!(client.get_total_collected(), 100);
    assert_eq!(client.get_user_fees_accrued(&payer), 100);
}

#[test]
fn test_deduct_fee_with_fallback_taken_when_fee_exceeds_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &10_000u32); // 100% fee rate
    client.set_fee_bounds(&admin, &5i128, &999_999i128);

    // fee == amount -> Success, net = 0
    let r1 = client.deduct_fee_with_fallback(&payer, &50i128, &PriorityLevel::Medium);
    assert_eq!(r1.status, FeeOperationStatus::Success);
    assert_eq!(r1.fee_charged, 50);
    assert_eq!(r1.net_amount, 0);

    // 100% of 3 = 3, but min_fee clamps to 5 > 3 -> Fallback; fee capped at amount (3)
    let r2 = client.deduct_fee_with_fallback(&payer, &3i128, &PriorityLevel::Medium);
    assert_eq!(r2.status, FeeOperationStatus::FallbackUsed);
    assert_eq!(r2.fee_charged, 3);
    assert_eq!(r2.net_amount, 0);
}

#[test]
fn test_deduct_fee_with_fallback_urgency_multiplier() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &500u32); // 5% base
                                        // Urgent: 5% * 2.0 = 10% of 20_000 = 2_000
    let result = client.deduct_fee_with_fallback(&payer, &20_000i128, &PriorityLevel::Urgent);

    assert_eq!(result.status, FeeOperationStatus::Success);
    assert_eq!(result.fee_charged, 2_000);
    assert_eq!(result.net_amount, 18_000);
}

#[test]
fn test_deduct_asset_fee_with_fallback_success_path() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &200u32, &0i128, &0i128); // 2%

    let result =
        client.deduct_asset_fee_with_fallback(&payer, &asset, &5_000i128, &PriorityLevel::Medium);

    assert_eq!(result.status, FeeOperationStatus::Success);
    assert_eq!(result.fee_charged, 100);
    assert_eq!(result.net_amount, 4_900);
    assert_eq!(client.get_asset_fees_collected(&asset), 100);
    assert_eq!(client.get_user_asset_fees_accrued(&payer, &asset), 100);
}

#[test]
fn test_deduct_asset_fee_with_fallback_no_asset_config_uses_default() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env); // NOT configured
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);
    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    ));

    client.initialize(&admin, &100u32);

    let result =
        client.deduct_asset_fee_with_fallback(&payer, &asset, &10_000i128, &PriorityLevel::Medium);

    assert_eq!(result.status, FeeOperationStatus::FallbackUsed);
    assert_eq!(result.fee_charged, 100);
    assert_eq!(result.net_amount, 9_900);
    assert_eq!(client.get_total_collected(), 100);
}

#[test]
fn test_deduct_asset_fee_with_fallback_asset_fee_too_large() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &0u32, &500i128, &0i128); // min_fee=500

    let result =
        client.deduct_asset_fee_with_fallback(&payer, &asset, &50i128, &PriorityLevel::Medium);

    // min_fee 500 > amount 50 -> fallback to default 1% of 50 = 0, net = 50
    assert_eq!(result.status, FeeOperationStatus::FallbackUsed);
    assert_eq!(result.net_amount, 50);
}

#[test]
fn test_deduct_batch_fees_updates_user_global_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &100u32, &0i128, &0i128);

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Low,
    )); // 0.8% = 80
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    )); // 1.0% = 100
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::High,
    )); // 1.5% = 150

    let result = client.deduct_batch_fees(&txs);
    assert_eq!(result.total_fees, 330);
    assert_eq!(client.get_user_fees_accrued(&payer), 330);

    // Per-user global balance reflects all three
    assert_eq!(
        FeeContract::get_user_fees_accrued(env.clone(), payer.clone()),
        330
    );
}

#[test]
fn test_deduct_batch_fees_emits_batch_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &asset, &100u32, &0i128, &0i128);

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    ));
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        10_000,
        PriorityLevel::Medium,
    ));
    client.deduct_batch_fees(&txs);

    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.1.get(0).and_then(|v| Symbol::try_from_val(&env, &v).ok()) == Some(symbol_short!("fee"))
            && e.1.get(1).and_then(|v| Symbol::try_from_val(&env, &v).ok())
                == Some(symbol_short!("batch"))
    }));

    FeeContract::deduct_batch_fees(env.clone(), txs);

    let events = env.events().all();
    assert!(events.iter().any(|e| {
        e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("batch")
    }));
    assert!(events.iter().any(|e| {
        e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("bat_itm")
    }));

    let item_event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("bat_itm"))
        .expect("batch item event should be emitted");
    assert_eq!(item_event.topics.2, payer.clone());
    assert_eq!(item_event.topics.3, 10_000i128);

    let summary_event = events
        .iter()
        .rev()
        .find(|e| e.topics.0 == symbol_short!("fee") && e.topics.1 == symbol_short!("batch"))
        .expect("batch summary event should be emitted");
    assert_eq!(summary_event.topics.2, payer.clone());
    assert_eq!(summary_event.topics.3, 200i128);
}

#[test]
#[should_panic]
fn test_deduct_batch_fees_rejects_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let asset = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    txs.push_back(make_tx(
        payer.clone(),
        asset.clone(),
        0,
        PriorityLevel::Medium,
    ));
    client.deduct_batch_fees(&txs);

    FeeContract::deduct_batch_fees(env.clone(), txs);
}

#[test]
fn test_calculate_batch_fees_mixed_assets_and_priorities() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let xlm = Address::generate(&env);
    let usdc = Address::generate(&env);
    let unconfigured = Address::generate(&env);
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);

    client.initialize(&admin, &100u32);
    client.set_asset_fee_config(&admin, &xlm, &50u32, &0i128, &0i128); // 0.5%
    client.set_asset_fee_config(&admin, &usdc, &300u32, &0i128, &0i128); // 3%

    let mut txs: Vec<FeeTransaction> = Vec::new(&env);
    // XLM  0.5% * 1.0 (medium) of 20000 = 100
    txs.push_back(make_tx(
        payer.clone(),
        xlm.clone(),
        20_000,
        PriorityLevel::Medium,
    )); // 0.5%*1.0=100
    // USDC 3% * 2.0 (urgent) of 10000 = 600
    txs.push_back(make_tx(
        payer.clone(),
        usdc.clone(),
        10_000,
        PriorityLevel::Urgent,
    )); // 3%*2.0=600
    // unconfigured falls back to 1% default, low priority 0.8 = 0.8% of 5000 = 40
    txs.push_back(make_tx(
        payer.clone(),
        unconfigured.clone(),
        5_000,
        PriorityLevel::Low,
    )); // 1%*0.8=40

    let result = FeeContract::calculate_batch_fees(env.clone(), txs);

    assert_eq!(result.results.get(0).unwrap().fee, 100);
    assert_eq!(result.results.get(1).unwrap().fee, 600);
    assert_eq!(result.results.get(2).unwrap().fee, 40);
    assert_eq!(result.total_fees, 740);
}
