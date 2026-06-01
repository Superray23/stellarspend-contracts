//! Comprehensive unit and integration tests for the savings goals contract.

#![cfg(test)]

use crate::{SavingsGoalsContract, SavingsGoalsContractClient};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, Symbol, Vec};

use crate::types::{
    ErrorCode, GoalResult, MilestoneAchievementRequest, MilestoneResult, SavingsGoalRequest,
};

/// Helper function to create a test environment with initialized contract.
fn setup_test_contract() -> (Env, Address, SavingsGoalsContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SavingsGoalsContract, ());
    let client = SavingsGoalsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, admin, client)
}

/// Helper function to create a valid savings goal request.
fn create_valid_request(
    env: &Env,
    user: &Address,
    goal_name: &str,
    amount: i128,
) -> SavingsGoalRequest {
    let current_ledger = env.ledger().sequence() as u64;
    SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(env, goal_name),
        target_amount: amount,
        deadline: current_ledger + 1000,
        initial_contribution: amount / 10, // 10% initial contribution
        lock_duration_seconds: 0,
        expiration_seconds: 0,
    }
}

#[test]
fn test_initialize() {
    let (_, admin, client) = setup_test_contract();

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_last_batch_id(), 0);
    assert_eq!(client.get_last_goal_id(), 0);
    assert_eq!(client.get_total_goals_created(), 0);
    assert_eq!(client.get_total_batches_processed(), 0);
    assert_eq!(client.get_last_milestone_id(), 0);
    assert_eq!(client.get_total_milestones_achieved(), 0);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialize_twice_fails() {
    let (env, _, client) = setup_test_contract();
    let new_admin = Address::generate(&env);
    client.initialize(&new_admin);
}

#[test]
fn test_auto_milestone_events() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);
    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "auto_milestone"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 25_000_000,
        lock_duration_seconds: 0,
        expiration_seconds: 0,
    });
    let result = client.batch_set_savings_goals(&admin, &requests);
    assert_eq!(result.successful, 1);

    client.contribute_to_goal(&user, &1, &25_000_000);
    client.contribute_to_goal(&user, &1, &25_000_000);
    client.contribute_to_goal(&user, &1, &25_000_000);

    let triggered = client.get_triggered_milestone_percents(&1);
    assert_eq!(triggered.len(), 4);
    assert!(triggered.contains(&25));
    assert!(triggered.contains(&50));
    assert!(triggered.contains(&75));
    assert!(triggered.contains(&100));

    client.check_and_emit_milestones(&1);
    let triggered_after = client.get_triggered_milestone_percents(&1);
    assert_eq!(triggered_after.len(), 4);
}

#[test]
fn test_batch_set_savings_goals_single_user() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 1);
    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);
    assert_eq!(result.batch_id, 1);

    // Verify storage updates
    assert_eq!(client.get_last_batch_id(), 1);
    assert_eq!(client.get_last_goal_id(), 1);
    assert_eq!(client.get_total_goals_created(), 1);
    assert_eq!(client.get_total_batches_processed(), 1);
}

#[test]
fn test_batch_set_savings_goals_multiple_users() {
    let (env, admin, client) = setup_test_contract();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));
    requests.push_back(create_valid_request(&env, &user2, "house", 500_000_000));
    requests.push_back(create_valid_request(&env, &user3, "emergency", 200_000_000));

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 3);
    assert_eq!(result.successful, 3);
    assert_eq!(result.failed, 0);
    assert_eq!(result.results.len(), 3);

    // Verify all goals were created successfully
    for goal_result in result.results.iter() {
        match goal_result {
            GoalResult::Success(goal) => {
                assert!(goal.goal_id > 0);
                assert!(goal.target_amount > 0);
                assert_eq!(goal.is_active, true);
            }
            GoalResult::Failure(_, _) => panic!("Expected success, got failure"),
        }
    }

    // Verify storage updates
    assert_eq!(client.get_total_goals_created(), 3);
    assert_eq!(client.get_last_goal_id(), 3);
}

#[test]
fn test_batch_set_savings_goals_with_invalid_requests() {
    let (env, admin, client) = setup_test_contract();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);

    // Valid request
    requests.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));

    // Invalid request - amount too low
    let mut invalid_request = create_valid_request(&env, &user2, "test", 1000);
    invalid_request.target_amount = 1000; // Below minimum
    requests.push_back(invalid_request);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 2);
    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 1);

    // Verify the first succeeded and second failed
    match &result.results.get(0).unwrap() {
        GoalResult::Success(_) => {}
        GoalResult::Failure(_, _) => panic!("Expected first request to succeed"),
    }

    match &result.results.get(1).unwrap() {
        GoalResult::Success(_) => panic!("Expected second request to fail"),
        GoalResult::Failure(_, error_code) => {
            assert_eq!(*error_code, ErrorCode::INVALID_AMOUNT);
        }
    }
}

#[test]
fn test_batch_set_savings_goals_invalid_deadline() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut request = create_valid_request(&env, &user, "vacation", 100_000_000);
    request.deadline = 0; // Past deadline
    requests.push_back(request);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        GoalResult::Failure(_, error_code) => {
            assert_eq!(*error_code, ErrorCode::INVALID_DEADLINE);
        }
        GoalResult::Success(_) => panic!("Expected failure"),
    }
}

#[test]
fn test_batch_set_savings_goals_invalid_initial_contribution() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut request = create_valid_request(&env, &user, "vacation", 100_000_000);
    request.initial_contribution = -1000; // Negative contribution
    requests.push_back(request);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        GoalResult::Failure(_, error_code) => {
            assert_eq!(*error_code, ErrorCode::INVALID_INITIAL_CONTRIBUTION);
        }
        GoalResult::Success(_) => panic!("Expected failure"),
    }
}

#[test]
#[should_panic]
fn test_batch_set_savings_goals_empty_batch() {
    let (env, admin, client) = setup_test_contract();
    let requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    client.batch_set_savings_goals(&admin, &requests);
}

#[test]
#[should_panic]
fn test_batch_set_savings_goals_batch_too_large() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    // Create 101 requests (exceeds MAX_BATCH_SIZE of 100)
    for i in 0..101 {
        requests.push_back(create_valid_request(
            &env,
            &user,
            "goal",
            100_000_000 + i as i128,
        ));
    }

    client.batch_set_savings_goals(&admin, &requests);
}

#[test]
fn test_get_goal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));

    let result = client.batch_set_savings_goals(&admin, &requests);

    // Get the created goal
    let goal = client.get_goal(&1).unwrap();

    assert_eq!(goal.goal_id, 1);
    assert_eq!(goal.user, user);
    assert_eq!(goal.target_amount, 100_000_000);
    assert_eq!(goal.current_amount, 10_000_000); // 10% initial
    assert_eq!(goal.is_active, true);
}

#[test]
fn test_get_goal_progress_and_completion() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));

    client.batch_set_savings_goals(&admin, &requests);

    let progress = client.get_goal_progress(&1).unwrap();
    assert_eq!(progress.goal_id, 1);
    assert_eq!(progress.current_amount, 10_000_000);
    assert_eq!(progress.target_amount, 100_000_000);
    assert_eq!(progress.progress_percentage, 10);
    assert_eq!(progress.is_complete, false);

    client.test_set_goal_current_amount(&1, &100_000_000);

    let completed_progress = client.get_goal_progress(&1).unwrap();
    assert_eq!(completed_progress.goal_id, 1);
    assert_eq!(completed_progress.current_amount, 100_000_000);
    assert_eq!(completed_progress.target_amount, 100_000_000);
    assert_eq!(completed_progress.progress_percentage, 100);
    assert_eq!(completed_progress.is_complete, true);

    let completed_goal = client.get_goal(&1).unwrap();
    assert_eq!(completed_goal.is_complete, true);
}

#[test]
fn test_get_user_goals() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));
    requests.push_back(create_valid_request(&env, &user, "house", 500_000_000));

    client.batch_set_savings_goals(&admin, &requests);

    let user_goals = client.get_user_goals(&user);
    assert_eq!(user_goals.len(), 2);
    assert_eq!(user_goals.get(0).unwrap(), 1);
    assert_eq!(user_goals.get(1).unwrap(), 2);
}

#[test]
fn test_batch_metrics() {
    let (env, admin, client) = setup_test_contract();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));
    requests.push_back(create_valid_request(&env, &user2, "house", 200_000_000));

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.metrics.total_requests, 2);
    assert_eq!(result.metrics.successful_goals, 2);
    assert_eq!(result.metrics.failed_goals, 0);
    assert_eq!(result.metrics.total_target_amount, 300_000_000);
    assert_eq!(result.metrics.total_initial_contributions, 30_000_000);
    assert_eq!(result.metrics.avg_goal_amount, 150_000_000);
}

#[test]
fn test_multiple_batches() {
    let (env, admin, client) = setup_test_contract();

    // First batch
    let user1 = Address::generate(&env);
    let mut requests1: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests1.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));
    let result1 = client.batch_set_savings_goals(&admin, &requests1);
    assert_eq!(result1.batch_id, 1);

    // Second batch
    let user2 = Address::generate(&env);
    let mut requests2: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests2.push_back(create_valid_request(&env, &user2, "house", 500_000_000));
    let result2 = client.batch_set_savings_goals(&admin, &requests2);
    assert_eq!(result2.batch_id, 2);

    // Verify totals
    assert_eq!(client.get_total_batches_processed(), 2);
    assert_eq!(client.get_total_goals_created(), 2);
    assert_eq!(client.get_last_goal_id(), 2);
}

#[test]
fn test_high_value_goal_event() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    // Create high-value goal (>= 100,000 XLM)
    requests.push_back(create_valid_request(
        &env,
        &user,
        "mansion",
        1_000_000_000_000,
    ));

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.successful, 1);
    // High-value event should be emitted (verified in event logs)
}

#[test]
fn test_set_admin() {
    let (env, admin, client) = setup_test_contract();
    let new_admin = Address::generate(&env);

    client.set_admin(&admin, &new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_mixed_valid_and_invalid_requests() {
    let (env, admin, client) = setup_test_contract();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let user4 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);

    // Valid
    requests.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));

    // Invalid - amount too low
    let mut invalid1 = create_valid_request(&env, &user2, "test", 1000);
    invalid1.target_amount = 1000;
    requests.push_back(invalid1);

    // Valid
    requests.push_back(create_valid_request(&env, &user3, "house", 500_000_000));

    // Invalid - deadline in past
    let mut invalid2 = create_valid_request(&env, &user4, "test", 100_000_000);
    invalid2.deadline = 0;
    requests.push_back(invalid2);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 4);
    assert_eq!(result.successful, 2);
    assert_eq!(result.failed, 2);

    // Only successful goals should be stored
    assert_eq!(client.get_total_goals_created(), 2);
}

#[test]
fn test_zero_initial_contribution() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut request = create_valid_request(&env, &user, "vacation", 100_000_000);
    request.initial_contribution = 0; // Zero initial contribution is valid
    requests.push_back(request);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);

    let goal = client.get_goal(&1).unwrap();
    assert_eq!(goal.current_amount, 0);
}

#[test]
fn test_duplicate_goal_name_same_user() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));
    requests.push_back(create_valid_request(&env, &user, "vacation", 200_000_000)); // Duplicate name

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 2);
    assert_eq!(result.successful, 1); // First one succeeds
    assert_eq!(result.failed, 1); // Second one fails (duplicate name)

    // Verify first succeeded
    match &result.results.get(0).unwrap() {
        GoalResult::Success(_) => {}
        GoalResult::Failure(_, _) => panic!("Expected first request to succeed"),
    }

    // Verify second failed with duplicate error
    match &result.results.get(1).unwrap() {
        GoalResult::Success(_) => panic!("Expected second request to fail"),
        GoalResult::Failure(_, error_code) => {
            assert_eq!(*error_code, ErrorCode::DUPLICATE_GOAL_NAME);
        }
    }

    assert_eq!(client.get_total_goals_created(), 1);
}

#[test]
fn test_same_goal_name_different_users() {
    let (env, admin, client) = setup_test_contract();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user1, "vacation", 100_000_000));
    requests.push_back(create_valid_request(&env, &user2, "vacation", 200_000_000)); // Same name, different user

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.total_requests, 2);
    assert_eq!(result.successful, 2); // Both should succeed
    assert_eq!(result.failed, 0);
    assert_eq!(client.get_total_goals_created(), 2);
}

#[test]
fn test_duplicate_goal_name_across_batches() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // First batch creates "vacation"
    let mut requests1: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests1.push_back(create_valid_request(&env, &user, "vacation", 100_000_000));
    let result1 = client.batch_set_savings_goals(&admin, &requests1);
    assert_eq!(result1.successful, 1);

    // Second batch tries to create "vacation" again for same user
    let mut requests2: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests2.push_back(create_valid_request(&env, &user, "vacation", 200_000_000));
    let result2 = client.batch_set_savings_goals(&admin, &requests2);
    assert_eq!(result2.successful, 0);
    assert_eq!(result2.failed, 1);

    match &result2.results.get(0).unwrap() {
        GoalResult::Failure(_, error_code) => {
            assert_eq!(*error_code, ErrorCode::DUPLICATE_GOAL_NAME);
        }
        GoalResult::Success(_) => panic!("Expected duplicate to fail"),
    }
}

#[test]
fn test_full_initial_contribution() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut request = create_valid_request(&env, &user, "vacation", 100_000_000);
    request.initial_contribution = 100_000_000; // Full amount
    requests.push_back(request);

    let result = client.batch_set_savings_goals(&admin, &requests);

    assert_eq!(result.successful, 1);

    let goal = client.get_goal(&1).unwrap();
    assert_eq!(goal.current_amount, 100_000_000);
    assert_eq!(goal.target_amount, 100_000_000);
}

// ==================== Goal Auto-Closure Tests (#599) ====================

#[test]
fn test_contribute_to_goal_increases_current_amount() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "house", 100_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    let updated = client.contribute_to_goal(&user, &1, &20_000_000_i128);

    assert_eq!(updated.current_amount, 20_000_000);
    assert_eq!(updated.is_active, true);
}

#[test]
fn test_goal_auto_closes_when_target_reached() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "car", 50_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    // Contribute exactly the target amount
    let updated = client.contribute_to_goal(&user, &1, &50_000_000_i128);

    // Goal should be auto-closed
    assert_eq!(updated.is_active, false);
    assert_eq!(updated.current_amount, 50_000_000);

    // get_goal_closed_at should return a value
    let closed_at = client.get_goal_closed_at(&1);
    assert!(closed_at.is_some());
}

#[test]
fn test_goal_auto_closes_on_over_contribution() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "fund", 50_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    // Contribute more than target — should be capped and goal auto-closed
    let updated = client.contribute_to_goal(&user, &1, &999_999_999_i128);

    assert_eq!(updated.is_active, false);
    assert_eq!(updated.current_amount, 50_000_000); // capped at target
}

#[test]
#[should_panic]
fn test_closed_goal_rejects_further_contributions() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "savings", 50_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    // Close the goal
    client.contribute_to_goal(&user, &1, &50_000_000_i128);

    // This contribution should panic because the goal is closed
    client.contribute_to_goal(&user, &1, &1_000_i128);
}

#[test]
#[should_panic]
fn test_contribute_with_wrong_caller_panics() {
    let (env, admin, client) = setup_test_contract();
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &owner, "trip", 100_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    // other is not the goal owner — should panic
    client.contribute_to_goal(&other, &1, &10_000_000_i128);
}

#[test]
#[should_panic]
fn test_contribute_zero_amount_panics() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "fund", 100_000_000);
    req.initial_contribution = 0;
    requests.push_back(req);
    client.batch_set_savings_goals(&admin, &requests);

    client.contribute_to_goal(&user, &1, &0_i128);
}

#[test]
fn test_get_goal_closed_at_none_for_open_goal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user, "fund", 100_000_000));
    client.batch_set_savings_goals(&admin, &requests);

    assert!(client.get_goal_closed_at(&1).is_none());
}

#[test]
fn test_full_initial_contribution_auto_closes_goal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut req = create_valid_request(&env, &user, "vacation", 100_000_000);
    req.initial_contribution = 100_000_000; // Full amount at creation
    requests.push_back(req);

    client.batch_set_savings_goals(&admin, &requests);

    // Goal created with 100% initial contribution should be auto-closed
    let goal = client.get_goal(&1).unwrap();
    assert_eq!(goal.current_amount, 100_000_000);
    assert_eq!(goal.target_amount, 100_000_000);
    assert_eq!(goal.is_active, false);
    assert!(client.get_goal_closed_at(&1).is_some());
}

// ==================== Milestone Achievement Tests ====================

#[test]
fn test_batch_mark_single_milestone() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal first
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);
    // Update goal's current_amount to meet milestone
    let mut goal = client.get_goal(&1).unwrap();
    goal.current_amount = 25_000_000; // 25% of 100_000_000
    client.test_set_goal_current_amount(&1, &25_000_000);

    // Mark a milestone
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 25,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);
    assert_eq!(result.total_requests, 1);
    assert_eq!(result.metrics.total_percentage_points, 25);
    assert_eq!(result.metrics.avg_percentage, 25);
    assert_eq!(client.get_total_milestones_achieved(), 1);
}

#[test]
fn test_batch_mark_multiple_milestones() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);
    // Update goal's current_amount to meet all milestones
    let mut goal = client.get_goal(&1).unwrap();
    goal.current_amount = 75_000_000; // 75% of 100_000_000
    client.test_set_goal_current_amount(&1, &75_000_000);

    // Mark multiple milestones in one batch
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 25,
        achieved_at: env.ledger().sequence() as u64,
    });
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 75,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.successful, 3);
    assert_eq!(result.failed, 0);
    assert_eq!(result.total_requests, 3);
    assert_eq!(result.metrics.total_percentage_points, 150);
    assert_eq!(result.metrics.avg_percentage, 50);

    // Verify all milestones were created
    let goal_milestones = client.get_goal_milestones(&1);
    assert_eq!(goal_milestones.len(), 3);
}

#[test]
fn test_milestone_invalid_percentage_zero() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);

    // Try to mark milestone with 0% (invalid)
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 0,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        MilestoneResult::Failure(_, code) => {
            assert_eq!(code, &ErrorCode::INVALID_MILESTONE_PERCENTAGE);
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_milestone_invalid_percentage_over_100() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);

    // Try to mark milestone with >100% (invalid)
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 101,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        MilestoneResult::Failure(_, code) => {
            assert_eq!(*code, ErrorCode::INVALID_MILESTONE_PERCENTAGE);
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_milestone_goal_not_found() {
    let (env, _admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Try to mark milestone for non-existent goal
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 999,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        MilestoneResult::Failure(_, code) => {
            assert_eq!(code, &ErrorCode::GOAL_NOT_FOUND);
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_milestone_unauthorized_user() {
    let (env, admin, client) = setup_test_contract();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Create a goal for user1
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user1, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);

    // Try to mark milestone as user2 (not the goal owner)
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user1.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user2, &milestone_requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        MilestoneResult::Failure(_, code) => {
            assert_eq!(code, &ErrorCode::UNAUTHORIZED_USER);
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_milestone_duplicate_percentage() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);

    // Update current amount so the 50% milestone is achievable.
    client.test_set_goal_current_amount(&1, &50_000_000);

    // Mark first milestone
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });
    client.batch_mark_milestones(&user, &milestone_requests);

    // Try to mark the same milestone again
    let mut duplicate_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    duplicate_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &duplicate_requests);

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    match &result.results.get(0).unwrap() {
        MilestoneResult::Failure(_, code) => {
            assert_eq!(code, &ErrorCode::MILESTONE_ALREADY_ACHIEVED);
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_milestone_partial_failures() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);
    // Update goal's current_amount to meet valid milestones
    let mut goal = client.get_goal(&1).unwrap();
    goal.current_amount = 75_000_000; // 75% of 100_000_000
    client.test_set_goal_current_amount(&1, &75_000_000);

    // Create a batch with mixed valid and invalid requests
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);

    // Valid
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 25,
        achieved_at: env.ledger().sequence() as u64,
    });

    // Invalid - percentage too high
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 101,
        achieved_at: env.ledger().sequence() as u64,
    });

    // Valid
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 75,
        achieved_at: env.ledger().sequence() as u64,
    });

    // Invalid - goal not found
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 999,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });

    let result = client.batch_mark_milestones(&user, &milestone_requests);

    assert_eq!(result.total_requests, 4);
    assert_eq!(result.successful, 2);
    assert_eq!(result.failed, 2);
    assert_eq!(result.metrics.total_percentage_points, 100);
    assert_eq!(result.metrics.avg_percentage, 50);

    // Verify only successful milestones were created
    let goal_milestones = client.get_goal_milestones(&1);
    assert_eq!(goal_milestones.len(), 2);
}

#[test]
fn test_milestone_retrieve_milestone() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);
    // Update goal's current_amount to meet milestone
    let mut goal = client.get_goal(&1).unwrap();
    goal.current_amount = 50_000_000; // 50% of 100_000_000
    client.test_set_goal_current_amount(&1, &50_000_000);

    // Mark a milestone
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    milestone_requests.push_back(MilestoneAchievementRequest {
        goal_id: 1,
        user: user.clone(),
        milestone_percentage: 50,
        achieved_at: env.ledger().sequence() as u64,
    });
    client.batch_mark_milestones(&user, &milestone_requests);

    // Retrieve and verify milestone
    let milestone = client.get_milestone(&1).unwrap();
    assert_eq!(milestone.milestone_id, 1);
    assert_eq!(milestone.goal_id, 1);
    assert_eq!(milestone.user, user);
    assert_eq!(milestone.milestone_percentage, 50);
}

#[test]
#[should_panic]
fn test_milestone_empty_batch() {
    let (env, _admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);

    client.batch_mark_milestones(&user, &milestone_requests);
}

#[test]
#[should_panic]
fn test_milestone_batch_too_large() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    // Create a goal
    let mut goal_requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    goal_requests.push_back(create_valid_request(&env, &user, "savings", 100_000_000));
    client.batch_set_savings_goals(&admin, &goal_requests);
    // Update goal's current_amount to meet milestone
    let mut goal = client.get_goal(&1).unwrap();
    goal.current_amount = 50_000_000; // 50% of 100_000_000
    client.test_set_goal_current_amount(&1, &50_000_000);

    // Create batch exceeding MAX_BATCH_SIZE
    let mut milestone_requests: Vec<MilestoneAchievementRequest> = Vec::new(&env);
    for i in 0..=100 {
        milestone_requests.push_back(MilestoneAchievementRequest {
            goal_id: 1,
            user: user.clone(),
            milestone_percentage: ((i % 100) + 1) as u32,
            achieved_at: env.ledger().sequence() as u64,
        });
    }

    client.batch_mark_milestones(&user, &milestone_requests);
}

// ==================== Lock Duration & Withdrawal Tests ====================

#[test]
fn test_locked_goal_rejects_withdrawal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "locked"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 50_000_000,
        lock_duration_seconds: 86_400,
        expiration_seconds: 0,
    });
    client.batch_set_savings_goals(&admin, &requests);

    let goal = client.get_goal(&1).unwrap();
    assert!(goal.unlock_at > 0);
    assert!(client.is_goal_locked(&1));

    let result = client.try_withdraw_from_goal(&user, &1, &10_000_000);
    assert!(result.is_err());
}

#[test]
fn test_unlocked_goal_allows_withdrawal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "unlocked"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 50_000_000,
        lock_duration_seconds: 0,
        expiration_seconds: 0,
    });
    client.batch_set_savings_goals(&admin, &requests);

    assert!(!client.is_goal_locked(&1));
    let remaining = client.withdraw_from_goal(&user, &1, &10_000_000);
    assert_eq!(remaining, 40_000_000);
}

#[test]
fn test_withdrawal_allowed_after_lock_expires() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "timed_lock"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 50_000_000,
        lock_duration_seconds: 3_600,
        expiration_seconds: 0,
    });
    client.batch_set_savings_goals(&admin, &requests);

    let goal = client.get_goal(&1).unwrap();
    env.ledger().set_timestamp(goal.unlock_at + 1);

    assert!(!client.is_goal_locked(&1));
    let remaining = client.withdraw_from_goal(&user, &1, &10_000_000);
    assert_eq!(remaining, 40_000_000);
}

#[test]
fn test_contribute_emits_milestone_events() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    let mut request = create_valid_request(&env, &user, "savings", 100_000_000);
    request.initial_contribution = 100_000_000; // Complete from start
    requests.push_back(request);
    client.batch_set_savings_goals(&admin, &requests);

    // Goal should be complete
    let progress = client.get_goal_progress(&1).unwrap();
    assert_eq!(progress.is_complete, true);

    // Verify goal_completed event was emitted
    let events = env.events().all();
    let completed_event = events.iter().find(|e| {
        if e.1.len() < 2 {
            return false;
        }
        if let Ok(topic0) = soroban_sdk::Symbol::try_from_val(&env, e.1.get(0).unwrap()) {
            if let Ok(topic1) = soroban_sdk::Symbol::try_from_val(&env, e.1.get(1).unwrap()) {
                return topic0 == soroban_sdk::symbol_short!("goal")
                    && topic1 == soroban_sdk::symbol_short!("completed");
            }
        }
        false
    });
    assert!(completed_event.is_some());
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "milestones"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 0,
        lock_duration_seconds: 0,
        expiration_seconds: 0,
    });
    client.batch_set_savings_goals(&admin, &requests);

    client.contribute_to_goal(&user, &1, &25_000_000);
    client.contribute_to_goal(&user, &1, &25_000_000);
    client.contribute_to_goal(&user, &1, &25_000_000);
    client.contribute_to_goal(&user, &1, &25_000_000);

    let triggered = client.get_triggered_milestone_percents(&1);
    assert_eq!(triggered.len(), 4);
    assert!(triggered.contains(&25));
    assert!(triggered.contains(&50));
    assert!(triggered.contains(&75));
    assert!(triggered.contains(&100));
}

#[test]
fn test_clone_savings_goal() {
    let (env, admin, client) = setup_test_contract();
    let user = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(SavingsGoalRequest {
        user: user.clone(),
        goal_name: Symbol::new(&env, "original"),
        target_amount: 100_000_000,
        deadline: env.ledger().sequence() as u64 + 1000,
        initial_contribution: 50_000_000,
        lock_duration_seconds: 3600,
        expiration_seconds: 0,
    });
    client.batch_set_savings_goals(&admin, &requests);

    let cloned_name = Symbol::new(&env, "cloned");
    let cloned_id = client.clone_savings_goal(&user, &1, &cloned_name);

    assert_eq!(cloned_id, 2);

    let cloned_goal = client.get_goal(&cloned_id).unwrap();
    assert_eq!(cloned_goal.goal_name, cloned_name);
    assert_eq!(cloned_goal.target_amount, 100_000_000);
    assert_eq!(cloned_goal.current_amount, 0); // Balance reset
    assert_eq!(cloned_goal.user, user);
    assert_eq!(cloned_goal.is_complete, false);
    assert!(cloned_goal.unlock_at > cloned_goal.created_at); // Lock inherited
}

#[test]
#[should_panic]
fn test_clone_savings_goal_unauthorized() {
    let (env, admin, client) = setup_test_contract();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let mut requests: Vec<SavingsGoalRequest> = Vec::new(&env);
    requests.push_back(create_valid_request(&env, &user1, "original", 100_000_000));
    client.batch_set_savings_goals(&admin, &requests);

    let cloned_name = Symbol::new(&env, "cloned");
    client.clone_savings_goal(&user2, &1, &cloned_name); // user2 tries to clone user1's goal
}
