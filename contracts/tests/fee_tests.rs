use soroban_sdk::{testutils::Address as _, Address, Env};
use stellarspend_contracts::fee::{FeeConfig, FeeWindow, FeeContract, FeeContractClient};


#[test]
fn test_default_fee() {
    let env = Env::default();
    let config = FeeConfig {
        default_fee_rate: 100, // 1%
        windows: vec![],
    };
    let fee = calculate_fee(&env, 1000, &config);
    assert_eq!(fee, 10);
}

#[test]
fn test_promotional_window() {
    let env = Env::default();
    let now = env.ledger().timestamp();
    let config = FeeConfig {
        default_fee_rate: 100,
        windows: vec![FeeWindow { start: now - 10, end: now + 10, fee_rate: 50 }],
    };
    let fee = calculate_fee(&env, 1000, &config);
    assert_eq!(fee, 5);
}


#[test]
fn test_overflow_protection() {
    let env = Env::default();
    let config = FeeConfig { default_fee_rate: 100, windows: vec![] };
    env.storage().persistent().set(&"fee_config", &config);

    let result = FeeContract::get_fee(env.clone(), i128::MAX);
    assert!(result.is_err());
}

#[test]
fn test_underflow_protection() {
    let env = Env::default();
    let config = FeeConfig { default_fee_rate: 100, windows: vec![] };
    env.storage().persistent().set(&"fee_config", &config);

    let result = FeeContract::get_fee(env.clone(), -1000);
    assert!(result.is_err());
}

#[test]
fn test_set_min_fee() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);
    
    // Initialize contract
    client.initialize(&admin, &token, &treasury, &300u32, &1u64);
    
    // Test initial min fee
    assert_eq!(client.get_min_fee(), 0);
    
    // Set new min fee as admin
    client.set_min_fee(&admin, &1000i128);
    assert_eq!(client.get_min_fee(), 1000);
    
    // Test unauthorized access
    let unauthorized = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_min_fee(&unauthorized, &2000i128);
    }));
    assert!(result.is_err());
    
    // Test invalid (negative) min fee
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_min_fee(&admin, &-100i128);
    }));
    assert!(result.is_err());
}

#[test]
fn test_zero_fee_config_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    
    let contract_id = env.register(FeeContract, ());
    let client = FeeContractClient::new(&env, &contract_id);
    
    // Initialize contract
    client.initialize(&admin, &token, &treasury, &300u32, &1u64);
    
    // Test zero fee bps is rejected
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_fee_bps(&admin, &0u32);
    }));
    assert!(result.is_err());
    
    // Test zero min fee should be allowed (it's non-negative)
    client.set_min_fee(&admin, &0i128);
    assert_eq!(client.get_min_fee(), 0);
}

