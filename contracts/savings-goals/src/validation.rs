//! Validation logic for savings goal requests.

use soroban_sdk::{Address, Env, Symbol};

use crate::types::{
    DataKey, ErrorCode, MilestoneAchievementRequest, SavingsGoal, SavingsGoalRequest,
    MAX_GOAL_AMOUNT, MIN_GOAL_AMOUNT,
};

/// Checks whether the user already has a goal with the same name.
/// Returns an error code if a duplicate is found.
pub fn validate_goal_name_unique(env: &Env, user: &Address, goal_name: &Symbol) -> Result<(), u32> {
    if env
        .storage()
        .persistent()
        .has(&DataKey::GoalByName(user.clone(), goal_name.clone()))
    {
        return Err(ErrorCode::DUPLICATE_GOAL_NAME);
    }
    Ok(())
}

/// Validates a savings goal request.
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(error_code)` if invalid
pub fn validate_goal_request(env: &Env, request: &SavingsGoalRequest) -> Result<(), u32> {
    // Validate user address - ensure it's not empty/invalid
    // Note: Soroban SDK doesn't provide a direct way to validate Address format,
    // but we can check basic properties
    if !is_valid_address(&request.user) {
        return Err(ErrorCode::INVALID_USER_ADDRESS);
    }

    // Validate goal name - should not be empty
    // Symbol validation: In Soroban, symbols are always valid by construction
    // and cannot be empty. This check is for consistency with validation patterns.
    // Note: Symbol doesn't have to_string() in no_std environment

    // Validate target amount
    if !is_valid_amount(request.target_amount) {
        return Err(ErrorCode::INVALID_AMOUNT);
    }

    // Validate deadline
    if !is_valid_deadline(env, request.deadline) {
        return Err(ErrorCode::INVALID_DEADLINE);
    }

    // Validate initial contribution
    if !is_valid_initial_contribution(request.initial_contribution, request.target_amount) {
        return Err(ErrorCode::INVALID_INITIAL_CONTRIBUTION);
    }

    Ok(())
}

/// Validates that an address is valid.
///
/// In Soroban, all Address instances are valid by construction,
/// but we can check for basic sanity.
fn is_valid_address(_address: &Address) -> bool {
    // Address is always valid in Soroban SDK by construction
    // This function exists for consistency with validation patterns
    // and potential future enhancements
    true
}

/// Validates that an amount is within acceptable bounds.
///
/// # Arguments
/// * `amount` - The amount to validate
///
/// # Returns
/// * `true` if amount is >= MIN_GOAL_AMOUNT and <= MAX_GOAL_AMOUNT
pub fn is_valid_amount(amount: i128) -> bool {
    amount >= MIN_GOAL_AMOUNT && amount <= MAX_GOAL_AMOUNT
}

/// Validates that a deadline is in the future but not too far.
///
/// # Arguments
/// * `env` - The contract environment
/// * `deadline` - The deadline ledger sequence number
///
/// # Returns
/// * `true` if deadline is valid
pub fn is_valid_deadline(env: &Env, deadline: u64) -> bool {
    let current_ledger = env.ledger().sequence() as u64;

    // Deadline must be in the future
    if deadline <= current_ledger {
        return false;
    }

    // Deadline should not be more than ~5 years in the future
    // Use saturating_add to avoid overflow
    let max_future_ledgers = 31_536_000u64; // ~5 years
    if deadline > current_ledger.saturating_add(max_future_ledgers) {
        return false;
    }

    true
}

/// Validates that initial contribution is valid.
///
/// # Arguments
/// * `initial_contribution` - The initial contribution amount
/// * `target_amount` - The target goal amount
///
/// # Returns
/// * `true` if initial contribution is valid
pub fn is_valid_initial_contribution(initial_contribution: i128, target_amount: i128) -> bool {
    // Initial contribution must be non-negative
    if initial_contribution < 0 {
        return false;
    }

    // Initial contribution cannot exceed target amount
    if initial_contribution > target_amount {
        return false;
    }

    true
}

/// Validates a batch of goal requests.
///
/// # Returns
/// * `Ok(())` if all requests have valid structure
/// * `Err(())` if any request is malformed
pub fn validate_batch(requests: &soroban_sdk::Vec<SavingsGoalRequest>) -> Result<(), ()> {
    // Check that batch is not empty
    if requests.is_empty() {
        return Err(());
    }

    // Basic structural validation
    // Individual validation happens during processing
    Ok(())
}

/// Validates a milestone achievement request.
///
/// # Arguments
/// * `env` - The contract environment
/// * `request` - The milestone achievement request
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(error_code)` if invalid
pub fn validate_milestone_request(
    env: &Env,
    request: &MilestoneAchievementRequest,
) -> Result<(), u32> {
    // Validate milestone percentage (must be 1-100)
    if request.milestone_percentage < 1 || request.milestone_percentage > 100 {
        return Err(ErrorCode::INVALID_MILESTONE_PERCENTAGE);
    }

    // Verify goal exists
    let goal: Option<SavingsGoal> = env
        .storage()
        .persistent()
        .get(&DataKey::Goal(request.goal_id));

    if goal.is_none() {
        return Err(ErrorCode::GOAL_NOT_FOUND);
    }

    let goal = goal.unwrap();

    // Verify goal is active
    if !goal.is_active {
        return Err(ErrorCode::GOAL_NOT_ACTIVE);
    }

    // Verify user is the goal owner
    if goal.user != request.user {
        return Err(ErrorCode::UNAUTHORIZED_USER);
    }

    // Verify milestone hasn't already been achieved
    if let Some(milestones) = env
        .storage()
        .persistent()
        .get::<_, soroban_sdk::Vec<u64>>(&DataKey::GoalMilestones(request.goal_id))
    {
        for milestone_id in milestones.iter() {
            if let Some(milestone) = env
                .storage()
                .persistent()
                .get::<_, crate::types::MilestoneAchievement>(&DataKey::Milestone(milestone_id))
            {
                if milestone.milestone_percentage == request.milestone_percentage {
                    return Err(ErrorCode::MILESTONE_ALREADY_ACHIEVED);
                }
            }
        }
    }

    Ok(())
}

/// Validates milestone percentage is in valid range.
///
/// # Arguments
/// * `percentage` - The milestone percentage
///
/// # Returns
/// * `true` if percentage is 1-100
pub fn is_valid_milestone_percentage(percentage: u32) -> bool {
    percentage >= 1 && percentage <= 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol_short, testutils::Address as _, Env};

    fn create_valid_request(env: &Env) -> SavingsGoalRequest {
        SavingsGoalRequest {
            user: Address::generate(env),
            goal_name: symbol_short!("vacation"),
            target_amount: 100_000_000, // 10 XLM
            deadline: env.ledger().sequence() as u64 + 1000,
            initial_contribution: 10_000_000, // 1 XLM
            lock_duration_seconds: 0,
        }
    }

    #[test]
    fn test_valid_goal_request() {
        let env = Env::default();
        let request = create_valid_request(&env);
        assert!(validate_goal_request(&env, &request).is_ok());
    }

    #[test]
    fn test_invalid_amount_too_low() {
        let env = Env::default();
        let mut request = create_valid_request(&env);
        request.target_amount = 1000; // Below minimum
        assert_eq!(
            validate_goal_request(&env, &request),
            Err(ErrorCode::INVALID_AMOUNT)
        );
    }

    #[test]
    fn test_invalid_amount_negative() {
        let env = Env::default();
        let mut request = create_valid_request(&env);
        request.target_amount = -1000;
        assert_eq!(
            validate_goal_request(&env, &request),
            Err(ErrorCode::INVALID_AMOUNT)
        );
    }

    #[test]
    fn test_invalid_deadline_past() {
        let env = Env::default();
        let mut request = create_valid_request(&env);
        request.deadline = 0; // Past deadline
        assert_eq!(
            validate_goal_request(&env, &request),
            Err(ErrorCode::INVALID_DEADLINE)
        );
    }

    #[test]
    fn test_invalid_initial_contribution_negative() {
        let env = Env::default();
        let mut request = create_valid_request(&env);
        request.initial_contribution = -1000;
        assert_eq!(
            validate_goal_request(&env, &request),
            Err(ErrorCode::INVALID_INITIAL_CONTRIBUTION)
        );
    }

    #[test]
    fn test_invalid_initial_contribution_exceeds_target() {
        let env = Env::default();
        let mut request = create_valid_request(&env);
        request.initial_contribution = request.target_amount + 1;
        assert_eq!(
            validate_goal_request(&env, &request),
            Err(ErrorCode::INVALID_INITIAL_CONTRIBUTION)
        );
    }

    #[test]
    fn test_is_valid_amount() {
        assert!(is_valid_amount(MIN_GOAL_AMOUNT));
        assert!(is_valid_amount(MAX_GOAL_AMOUNT));
        assert!(is_valid_amount(100_000_000));
        assert!(!is_valid_amount(MIN_GOAL_AMOUNT - 1));
        assert!(!is_valid_amount(MAX_GOAL_AMOUNT + 1));
        assert!(!is_valid_amount(-1000));
    }

    #[test]
    fn test_is_valid_deadline() {
        let env = Env::default();
        let current = env.ledger().sequence() as u64;

        assert!(is_valid_deadline(&env, current + 100));
        assert!(is_valid_deadline(&env, current + 1000000));
        assert!(!is_valid_deadline(&env, current));
        assert!(!is_valid_deadline(&env, current.saturating_sub(100)));
    }

    #[test]
    fn test_is_valid_initial_contribution() {
        assert!(is_valid_initial_contribution(0, 100_000_000));
        assert!(is_valid_initial_contribution(50_000_000, 100_000_000));
        assert!(is_valid_initial_contribution(100_000_000, 100_000_000));
        assert!(!is_valid_initial_contribution(-1, 100_000_000));
        assert!(!is_valid_initial_contribution(100_000_001, 100_000_000));
    }

    #[test]
    fn test_validate_goal_name_unique_no_duplicate() {
        let env = Env::default();
        let contract_id = env.register(crate::SavingsGoalsContract, ());
        let user = Address::generate(&env);
        let name = symbol_short!("vacation");
        env.as_contract(&contract_id, || {
            assert!(validate_goal_name_unique(&env, &user, &name).is_ok());
        });
    }

    #[test]
    fn test_validate_goal_name_unique_detects_duplicate() {
        let env = Env::default();
        let contract_id = env.register(crate::SavingsGoalsContract, ());
        let user = Address::generate(&env);
        let name = symbol_short!("vacation");

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::GoalByName(user.clone(), name.clone()), &1u64);

            assert_eq!(
                validate_goal_name_unique(&env, &user, &name),
                Err(ErrorCode::DUPLICATE_GOAL_NAME)
            );
        });
    }

    #[test]
    fn test_validate_goal_name_unique_allows_same_name_different_user() {
        let env = Env::default();
        let contract_id = env.register(crate::SavingsGoalsContract, ());
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let name = symbol_short!("vacation");

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::GoalByName(user1.clone(), name.clone()), &1u64);

            assert!(validate_goal_name_unique(&env, &user2, &name).is_ok());
        });
    }
}
