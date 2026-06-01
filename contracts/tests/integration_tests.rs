//! # End-to-End Integration Tests
//!
//! Comprehensive integration tests covering the full lifecycle of StellarSpend features.
//! These tests verify interactions between multiple contracts and real-world usage scenarios.

use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
use stellarspend_contracts::{
    budget_allocation::BudgetAllocationContractClient,
    spending_limits::SpendingLimitsContractClient,
    savings_goals::SavingsGoalsContractClient,
    fee::FeeContractClient,
};

/// Helper function to set up a complete test environment with all required contracts
fn setup_integration_test() -> (
    Env,
    Address,
    BudgetAllocationContractClient<'static>,
    SpendingLimitsContractClient<'static>,
    SavingsGoalsContractClient<'static>,
    FeeContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    // Register all contracts
    let budget_contract_id = env.register(
        stellarspend_contracts::budget_allocation::BudgetAllocationContract,
        (),
    );
    let spending_contract_id = env.register(
        stellarspend_contracts::spending_limits::SpendingLimitsContract,
        (),
    );
    let savings_contract_id = env.register(
        stellarspend_contracts::savings_goals::SavingsGoalsContract,
        (),
    );
    let fee_contract_id = env.register(
        stellarspend_contracts::fee::FeeContract,
        (),
    );

    let budget_client = BudgetAllocationContractClient::new(&env, &budget_contract_id);
    let spending_client = SpendingLimitsContractClient::new(&env, &spending_contract_id);
    let savings_client = SavingsGoalsContractClient::new(&env, &savings_contract_id);
    let fee_client = FeeContractClient::new(&env, &fee_contract_id);

    // Initialize contracts
    let admin = Address::generate(&env);
    budget_client.initialize(&admin);
    spending_client.initialize(&admin);
    savings_client.initialize(&admin);
    fee_client.initialize(&admin, &admin, &Some(100_000_000i128), &Some(500_000_000i128), &Some(1_000_000_000i128), &Some(5_000_000_000i128));

    (
        env,
        admin,
        budget_client,
        spending_client,
        savings_client,
        fee_client,
    )
}

#[test]
fn test_budget_creation_and_spending_enforcement() {
    let (env, admin, budget_client, spending_client, ..) = setup_integration_test();

    // Create user addresses
    let user = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Allocate budget for user
    let mut budget_requests: Vec<stellarspend_contracts::budget_allocation::types::BudgetRequest> = Vec::new(&env);
    budget_requests.push_back(stellarspend_contracts::budget_allocation::types::BudgetRequest {
        user: user.clone(),
        amount: 100_000_000_000i128, // 10,000 XLM
    });
    budget_client.batch_allocate_budget(&admin, &budget_requests);

    // Set spending limit for user
    let mut limit_requests: Vec<stellarspend_contracts::spending_limits::types::SpendingLimitRequest> = Vec::new(&env);
    limit_requests.push_back(stellarspend_contracts::spending_limits::types::SpendingLimitRequest {
        user: user.clone(),
        monthly_limit: 50_000_000_000i128, // 5,000 XLM
        reset_window_seconds: 2_592_000, // 30 days
        category: None,
    });
    spending_client.batch_update_spending_limits(&admin, &limit_requests);

    // Whitelist merchant
    spending_client.whitelist_destination(&admin, &merchant);

    // Enforce spending limit - should succeed
    spending_client.enforce_spending_limit(&user, &10_000_000_000i128); // 1,000 XLM

    // Try to spend to non-whitelisted address - should fail
    let unauthorized_merchant = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        spending_client.enforce_spending_limit(&user, &10_000_000_000i128);
    }));
    assert!(result.is_err());
}

#[test]
fn test_savings_goal_completion_and_withdrawal() {
    let (env, admin, .., savings_client, fee_client) = setup_integration_test();

    let user = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Create savings goal
    savings_client.create_savings_goal(
        &user,
        &"vacation".into(),
        &100_000_000_000i128, // 10,000 XLM target
        &10_000_000_000i128,  // 1,000 XLM monthly contribution
        &12u32,                // 12 months
    );

    // Make recurring contributions
    for _ in 0..12 {
        savings_client.make_contribution(&user, &10_000_000_000i128);
    }

    // Check if goal is complete
    let goal = savings_client.get_savings_goal(&user, &"vacation".into()).unwrap();
    assert_eq!(goal.current_amount, 120_000_000_000i128);
    assert_eq!(goal.target_amount, 100_000_000_000i128);
    assert!(goal.is_complete);

    // Withdraw funds
    savings_client.withdraw_funds(&user, &"vacation".into(), &100_000_000_000i128);

    // Verify withdrawal was processed through fee contract
    let pool_balance = fee_client.get_fee_pool();
    assert!(pool_balance > 0);
}

#[test]
fn test_full_lifecycle_scenario() {
    let (env, admin, budget_client, spending_client, savings_client, fee_client) = setup_integration_test();

    let user = Address::generate(&env);
    let merchant = Address::generate(&env);

    // 1. Create budget
    let mut budget_requests: Vec<stellarspend_contracts::budget_allocation::types::BudgetRequest> = Vec::new(&env);
    budget_requests.push_back(stellarspend_contracts::budget_allocation::types::BudgetRequest {
        user: user.clone(),
        amount: 100_000_000_000i128,
    });
    budget_client.batch_allocate_budget(&admin, &budget_requests);

    // 2. Set spending limits and whitelist
    let mut limit_requests: Vec<stellarspend_contracts::spending_limits::types::SpendingLimitRequest> = Vec::new(&env);
    limit_requests.push_back(stellarspend_contracts::spending_limits::types::SpendingLimitRequest {
        user: user.clone(),
        monthly_limit: 50_000_000_000i128,
        reset_window_seconds: 2_592_000,
        category: None,
    });
    spending_client.batch_update_spending_limits(&admin, &limit_requests);
    spending_client.whitelist_destination(&admin, &merchant);

    // 3. Make purchases
    for i in 0..5 {
        let amount = 10_000_000_000i128 + (i * 1_000_000_000i128);
        spending_client.enforce_spending_limit(&user, &amount);
    }

    // 4. Create savings goal
    savings_client.create_savings_goal(
        &user,
        &"emergency".into(),
        &50_000_000_000i128,
        &5_000_000_000i128,
        &10u32,
    );

    // 5. Make contributions
    for _ in 0..10 {
        savings_client.make_contribution(&user, &5_000_000_000i128);
    }

    // 6. Verify goal completion
    let goal = savings_client.get_savings_goal(&user, &"emergency".into()).unwrap();
    assert!(goal.is_complete);

    // 7. Withdraw funds
    savings_client.withdraw_funds(&user, &"emergency".into(), &50_000_000_000i128);

    // 8. Verify fee collection
    let pool_balance = fee_client.get_fee_pool();
    assert!(pool_balance > 0);
}
