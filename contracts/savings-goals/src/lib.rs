//! # Savings Goals Contract
//!
//! A Soroban smart contract for managing batch savings goal creation
//! and batch milestone achievement tracking for multiple users simultaneously.
//!
//! ## Features
//!
//! - **Batch Processing**: Efficiently create savings goals for multiple users in a single call
//! - **Batch Milestones**: Mark milestones achieved for multiple goals in a single call
//! - **Comprehensive Validation**: Validates goal amounts, deadlines, and milestone percentages
//! - **Event Emission**: Emits events for goal creation, milestone achievements, and batch processing
//! - **Error Handling**: Gracefully handles invalid inputs with detailed error codes
//! - **Optimized Storage**: Minimizes storage writes by batching operations
//! - **Partial Failure Support**: Batch operations continue even if some individual operations fail
//!
//! ## Optimization Strategies
//!
//! - Single-pass processing for O(n) complexity
//! - Minimized storage operations (batch writes at the end)
//! - Efficient data structures
//! - Batched event emissions

#![no_std]

mod types;
mod validation;

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, Symbol, Vec};

pub use crate::types::{
    BatchGoalMetrics, BatchGoalResult, BatchMilestoneMetrics, BatchMilestoneResult, DataKey,
    ErrorCode, GoalEvents, GoalResult, MilestoneAchievement, MilestoneAchievementRequest,
    MilestoneResult, SavingsGoal, SavingsGoalProgress, SavingsGoalRequest, MAX_BATCH_SIZE,
};
use crate::validation::{
    validate_goal_name_unique, validate_goal_request, validate_milestone_request,
};

/// Error codes for the savings goals contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SavingsGoalError {
    /// Contract not initialized
    NotInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Invalid batch data
    InvalidBatch = 3,
    /// Batch is empty
    EmptyBatch = 4,
    /// Batch exceeds maximum size
    BatchTooLarge = 5,
    /// Insufficient balance for withdrawal
    InsufficientBalance = 6,
    /// Goal is not active
    GoalNotActive = 7,
    /// Invalid goal name
    InvalidGoalName = 8,
    /// Invalid contribution or withdrawal amount
    InvalidAmount = 9,
    /// Goal not found
    GoalNotFound = 10,
    /// Goal is locked against withdrawals
    GoalLocked = 11,
    /// Goal has expired
    GoalExpired = 12,
    /// One or more prerequisite goals are not complete
    DependencyNotMet = 13,
}

impl From<SavingsGoalError> for soroban_sdk::Error {
    fn from(e: SavingsGoalError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}

#[contract]
pub struct SavingsGoalsContract;

#[contractimpl]
impl SavingsGoalsContract {
    /// Batch mark milestones for multiple goals and emit milestone events.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address calling this function (must be goal owner)
    /// * `requests` - Vector of milestone achievement requests
    ///
    /// # Returns
    /// * `BatchMilestoneResult` - Result containing milestone results and metrics
    pub fn batch_mark_milestones(
        env: Env,
        caller: Address,
        requests: Vec<MilestoneAchievementRequest>,
    ) -> BatchMilestoneResult {
        caller.require_auth();
        let mut results: Vec<MilestoneResult> = Vec::new(&env);
        let mut successful: u32 = 0;
        let mut failed: u32 = 0;
        let batch_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastBatchId)
            .unwrap_or(0)
            + 1;
        let mut total_percentage_points: u32 = 0;
        let mut processed_at = env.ledger().sequence() as u64;
        // Validate batch size
        let request_count = requests.len();
        if request_count == 0 {
            panic_with_error!(&env, SavingsGoalError::EmptyBatch);
        }
        if request_count > MAX_BATCH_SIZE {
            panic_with_error!(&env, SavingsGoalError::BatchTooLarge);
        }
        let mut last_milestone_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastMilestoneId)
            .unwrap_or(0);
        for req in requests.iter() {
            let goal: Option<SavingsGoal> =
                env.storage().persistent().get(&DataKey::Goal(req.goal_id));
            if let Some(goal) = goal {
                if goal.user != caller {
                    results.push_back(MilestoneResult::Failure(
                        req.goal_id,
                        ErrorCode::UNAUTHORIZED_USER,
                    ));
                    failed += 1;
                    continue;
                }
                let valid_percents = [25u32, 50, 75, 100];
                if !valid_percents.contains(&req.milestone_percentage) {
                    results.push_back(MilestoneResult::Failure(
                        req.goal_id,
                        ErrorCode::INVALID_MILESTONE_PERCENTAGE,
                    ));
                    failed += 1;
                    continue;
                }
                let mut triggered: Vec<u32> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::GoalMilestonesPercent(req.goal_id))
                    .unwrap_or(Vec::new(&env));
                if triggered.contains(&req.milestone_percentage) {
                    results.push_back(MilestoneResult::Failure(
                        req.goal_id,
                        ErrorCode::MILESTONE_ALREADY_ACHIEVED,
                    ));
                    failed += 1;
                    continue;
                }
                let progress = if goal.target_amount > 0 {
                    (goal.current_amount * 100 / goal.target_amount) as u32
                } else {
                    0
                };
                if progress < req.milestone_percentage {
                    results.push_back(MilestoneResult::Failure(
                        req.goal_id,
                        ErrorCode::MILESTONE_NOT_YET_ACHIEVED,
                    ));
                    failed += 1;
                    continue;
                }
                triggered.push_back(req.milestone_percentage);
                env.storage()
                    .persistent()
                    .set(&DataKey::GoalMilestonesPercent(req.goal_id), &triggered);
                GoalEvents::milestone_achieved_percent(&env, req.goal_id, req.milestone_percentage);
                // Store MilestoneAchievement and update milestone IDs
                last_milestone_id += 1;
                let achievement = MilestoneAchievement {
                    milestone_id: last_milestone_id,
                    goal_id: req.goal_id,
                    user: caller.clone(),
                    milestone_percentage: req.milestone_percentage,
                    goal_amount_at_achievement: goal.current_amount,
                    achieved_at: req.achieved_at,
                };
                env.storage()
                    .persistent()
                    .set(&DataKey::Milestone(last_milestone_id), &achievement);
                // Update goal's milestone ID list
                let mut milestone_ids: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::GoalMilestones(req.goal_id))
                    .unwrap_or(Vec::new(&env));
                milestone_ids.push_back(last_milestone_id);
                env.storage()
                    .persistent()
                    .set(&DataKey::GoalMilestones(req.goal_id), &milestone_ids);
                // Update last milestone ID and total milestones achieved
                env.storage()
                    .instance()
                    .set(&DataKey::LastMilestoneId, &last_milestone_id);
                let total_achieved = env
                    .storage()
                    .instance()
                    .get(&DataKey::TotalMilestonesAchieved)
                    .unwrap_or(0u64)
                    + 1;
                env.storage()
                    .instance()
                    .set(&DataKey::TotalMilestonesAchieved, &total_achieved);
                results.push_back(MilestoneResult::Success(achievement));
                total_percentage_points += req.milestone_percentage;
                successful += 1;
            } else {
                results.push_back(MilestoneResult::Failure(
                    req.goal_id,
                    ErrorCode::GOAL_NOT_FOUND,
                ));
                failed += 1;
            }
        }
        let avg_percentage = if successful > 0 {
            total_percentage_points / successful
        } else {
            0
        };
        let metrics = BatchMilestoneMetrics {
            total_requests: requests.len(),
            successful_milestones: successful,
            failed_milestones: failed,
            total_percentage_points,
            avg_percentage,
            processed_at,
        };
        BatchMilestoneResult {
            batch_id,
            total_requests: requests.len(),
            successful,
            failed,
            results,
            metrics,
        }
    }
    /// Initializes the contract with an admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address that can manage the contract
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::LastBatchId, &0u64);
        env.storage().instance().set(&DataKey::LastGoalId, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalGoalsCreated, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalBatchesProcessed, &0u64);
    }

    /// Creates savings goals for multiple users in a batch.
    ///
    /// This is the main entry point for batch goal creation. It validates all requests,
    /// creates goals, emits events, and updates storage efficiently.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address calling this function (must be admin)
    /// * `requests` - Vector of savings goal requests
    ///
    /// # Returns
    /// * `BatchGoalResult` - Result containing created goals and metrics
    ///
    /// # Events Emitted
    /// * `batch_started` - When processing begins
    /// * `goal_created` - For each successful goal creation
    /// * `goal_creation_failed` - For each failed goal creation
    /// * `high_value_goal` - For goals with high target amounts
    /// * `batch_completed` - When processing completes
    ///
    /// # Errors
    /// * `EmptyBatch` - If no requests provided
    /// * `BatchTooLarge` - If batch exceeds maximum size
    /// * `Unauthorized` - If caller is not admin
    pub fn batch_set_savings_goals(
        env: Env,
        caller: Address,
        requests: Vec<SavingsGoalRequest>,
    ) -> BatchGoalResult {
        // Verify authorization
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // Validate batch size
        let request_count = requests.len();
        if request_count == 0 {
            panic_with_error!(&env, SavingsGoalError::EmptyBatch);
        }
        if request_count > MAX_BATCH_SIZE {
            panic_with_error!(&env, SavingsGoalError::BatchTooLarge);
        }

        // Get batch ID and increment
        let batch_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastBatchId)
            .unwrap_or(0)
            + 1;

        // Emit batch started event
        GoalEvents::batch_started(&env, batch_id, request_count);

        // Get current ledger timestamp
        let current_ledger = env.ledger().sequence() as u64;

        // Initialize result tracking
        let mut results: Vec<GoalResult> = Vec::new(&env);
        let mut successful_count: u32 = 0;
        let mut failed_count: u32 = 0;
        let mut total_target_amount: i128 = 0;
        let mut total_initial_contributions: i128 = 0;
        let mut goal_id_counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastGoalId)
            .unwrap_or(0);

        // Process each request
        for request in requests.iter() {
            // Validate the request
            match validate_goal_request(&env, &request) {
                Ok(()) => {
                    // Check for duplicate goal name for this user
                    if let Err(error_code) =
                        validate_goal_name_unique(&env, &request.user, &request.goal_name)
                    {
                        failed_count += 1;
                        GoalEvents::goal_creation_failed(&env, batch_id, &request.user, error_code);
                        results.push_back(GoalResult::Failure(request.user.clone(), error_code));
                        continue;
                    }

                    // Validation succeeded - create the goal
                    goal_id_counter += 1;

                    let created_at = env.ledger().timestamp();
                    let unlock_at = if request.lock_duration_seconds > 0 {
                        created_at.saturating_add(request.lock_duration_seconds)
                    } else {
                        0
                    };
                    let is_complete = request.initial_contribution >= request.target_amount;
                    let mut goal = SavingsGoal {
                        goal_id: goal_id_counter,
                        user: request.user.clone(),
                        goal_name: request.goal_name.clone(),
                        target_amount: request.target_amount,
                        current_amount: request.initial_contribution,
                        deadline: request.deadline,
                        created_at,
                        is_active: true,
                        is_complete,
                        unlock_at,
                        expires_at,
                    };

                    // Accumulate metrics
                    total_target_amount = total_target_amount
                        .checked_add(request.target_amount)
                        .unwrap_or(i128::MAX);
                    total_initial_contributions = total_initial_contributions
                        .checked_add(request.initial_contribution)
                        .unwrap_or(i128::MAX);
                    successful_count += 1;

                    // Store the goal (optimized - one write per goal)
                    env.storage()
                        .persistent()
                        .set(&DataKey::Goal(goal_id_counter), &goal);
                    // Store name-to-id mapping for duplicate detection
                    env.storage().persistent().set(
                        &DataKey::GoalByName(request.user.clone(), request.goal_name.clone()),
                        &goal_id_counter,
                    );
                    // Emit milestone events for initial contribution
                    Self::check_and_emit_milestones(&env, goal_id_counter);

                    // Update local goal variable's is_complete if it reached target
                    if request.initial_contribution >= request.target_amount {
                        goal.is_complete = true;
                    }

                    // Update user's goal list
                    let mut user_goals: Vec<u64> = env
                        .storage()
                        .persistent()
                        .get(&DataKey::UserGoals(request.user.clone()))
                        .unwrap_or(Vec::new(&env));
                    user_goals.push_back(goal_id_counter);
                    env.storage()
                        .persistent()
                        .set(&DataKey::UserGoals(request.user.clone()), &user_goals);

                    // Emit success event
                    GoalEvents::goal_created(&env, batch_id, &goal);

                    // Emit high-value goal event if applicable (>= 100,000 XLM)
                    if request.target_amount >= 1_000_000_000_000 {
                        GoalEvents::high_value_goal(
                            &env,
                            batch_id,
                            goal_id_counter,
                            request.target_amount,
                        );
                    }

                    results.push_back(GoalResult::Success(goal));
                }
                Err(error_code) => {
                    // Validation failed - record failure
                    failed_count += 1;

                    // Emit failure event
                    GoalEvents::goal_creation_failed(&env, batch_id, &request.user, error_code);

                    results.push_back(GoalResult::Failure(request.user.clone(), error_code));
                }
            }
        }

        // Calculate average goal amount
        let avg_goal_amount = if successful_count > 0 {
            total_target_amount / successful_count as i128
        } else {
            0
        };

        // Create metrics
        let metrics = BatchGoalMetrics {
            total_requests: request_count,
            successful_goals: successful_count,
            failed_goals: failed_count,
            total_target_amount,
            total_initial_contributions,
            avg_goal_amount,
            processed_at: current_ledger,
        };

        // Update storage (batched at the end for efficiency)
        let total_goals: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalGoalsCreated)
            .unwrap_or(0);
        let total_batches: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalBatchesProcessed)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&DataKey::LastBatchId, &batch_id);
        env.storage()
            .instance()
            .set(&DataKey::LastGoalId, &goal_id_counter);
        env.storage().instance().set(
            &DataKey::TotalGoalsCreated,
            &(total_goals + successful_count as u64),
        );
        env.storage()
            .instance()
            .set(&DataKey::TotalBatchesProcessed, &(total_batches + 1));

        // Emit batch completed event
        GoalEvents::batch_completed(
            &env,
            batch_id,
            successful_count,
            failed_count,
            total_target_amount,
        );

        BatchGoalResult {
            batch_id,
            total_requests: request_count,
            successful: successful_count,
            failed: failed_count,
            results,
            metrics,
        }
    }

    /// Contributes funds to a savings goal and emits milestone events at 25/50/75/100%.
    pub fn contribute_to_goal(env: Env, caller: Address, goal_id: u64, amount: i128) -> i128 {
        caller.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, SavingsGoalError::InvalidAmount);
        }

        let mut goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::GoalNotFound));

        if goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }

        // Check if goal has expired
        let current_time = env.ledger().timestamp();
        if goal.expires_at > 0 && current_time >= goal.expires_at {
            // Mark goal as inactive
            goal.is_active = false;
            env.storage()
                .persistent()
                .set(&DataKey::Goal(goal_id), &goal);
            panic_with_error!(&env, SavingsGoalError::GoalExpired);
        }

        if !goal.is_active {
            panic_with_error!(&env, SavingsGoalError::GoalNotActive);
        }

        // Enforce prerequisite goals: cannot contribute until all prerequisites complete
        let prereqs: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::GoalPrereqs(goal_id))
            .unwrap_or(Vec::new(&env));
        for pid in prereqs.iter() {
            let pgoal_opt: Option<SavingsGoal> = env
                .storage()
                .persistent()
                .get(&DataKey::Goal(*pid));
            if let Some(pgoal) = pgoal_opt {
                if !pgoal.is_complete {
                    panic_with_error!(&env, SavingsGoalError::DependencyNotMet);
                }
            } else {
                panic_with_error!(&env, SavingsGoalError::GoalNotFound);
            }
        }

        goal.current_amount = goal.current_amount.checked_add(amount).unwrap_or(i128::MAX);
        goal.is_complete = goal.current_amount >= goal.target_amount;

        env.storage()
            .persistent()
            .set(&DataKey::Goal(goal_id), &goal);

        Self::check_and_emit_milestones(&env, goal_id);
        GoalEvents::goal_contributed(&env, goal_id, &caller, amount, goal.current_amount);

        goal.current_amount
    }

    /// Clones an existing savings goal with a new name and zero balance.
    pub fn clone_savings_goal(
        env: Env,
        caller: Address,
        goal_id: u64,
        new_goal_name: Symbol,
    ) -> u64 {
        caller.require_auth();

        let existing_goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::GoalNotFound));

        if existing_goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }

        // Check for duplicate goal name for this user
        if validate_goal_name_unique(&env, &caller, &new_goal_name).is_err() {
            panic_with_error!(&env, SavingsGoalError::InvalidGoalName);
        }

        let mut goal_id_counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastGoalId)
            .unwrap_or(0);
        goal_id_counter += 1;

        let created_at = env.ledger().timestamp();
        // Inherit lock duration if it was originally set
        let lock_duration = if existing_goal.unlock_at > existing_goal.created_at {
            existing_goal.unlock_at - existing_goal.created_at
        } else {
            0
        };
        let unlock_at = if lock_duration > 0 {
            created_at.saturating_add(lock_duration)
        } else {
            0
        };

        // Inherit expiration duration if it was originally set
        let expiration_duration = if existing_goal.expires_at > existing_goal.created_at {
            existing_goal.expires_at - existing_goal.created_at
        } else {
            0
        };
        let expires_at = if expiration_duration > 0 {
            created_at.saturating_add(expiration_duration)
        } else {
            0
        };

        let new_goal = SavingsGoal {
            goal_id: goal_id_counter,
            user: caller.clone(),
            goal_name: new_goal_name.clone(),
            target_amount: existing_goal.target_amount,
            current_amount: 0, // Reset balance
            deadline: existing_goal.deadline,
            created_at,
            is_active: true,
            is_complete: false,
            unlock_at,
            expires_at,
        };

        // Store the goal
        env.storage()
            .persistent()
            .set(&DataKey::Goal(goal_id_counter), &new_goal);
        // Store name-to-id mapping
        env.storage().persistent().set(
            &DataKey::GoalByName(caller.clone(), new_goal_name.clone()),
            &goal_id_counter,
        );

        // Update user's goal list
        let mut user_goals: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserGoals(caller.clone()))
            .unwrap_or(Vec::new(&env));
        user_goals.push_back(goal_id_counter);
        env.storage()
            .persistent()
            .set(&DataKey::UserGoals(caller.clone()), &user_goals);

        // Update global counter
        env.storage()
            .instance()
            .set(&DataKey::LastGoalId, &goal_id_counter);

        let total_goals: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalGoalsCreated)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalGoalsCreated, &(total_goals + 1));

        // Emit creation event (reusing batch 0 for manual creation)
        GoalEvents::goal_created(&env, 0, &new_goal);

        goal_id_counter
    }

    /// Withdraws funds from a savings goal. Rejects withdrawals before unlock time when locked.
    pub fn withdraw_from_goal(env: Env, caller: Address, goal_id: u64, amount: i128) -> i128 {
        caller.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, SavingsGoalError::InvalidAmount);
        }

        let mut goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::GoalNotFound));

        if goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }
        if !goal.is_active {
            panic_with_error!(&env, SavingsGoalError::GoalNotActive);
        }

        let now = env.ledger().timestamp();
        if goal.unlock_at > 0 && now < goal.unlock_at {
            GoalEvents::goal_withdraw_locked(&env, goal_id, &caller, goal.unlock_at);
            panic_with_error!(&env, SavingsGoalError::GoalLocked);
        }

        if amount > goal.current_amount {
            panic_with_error!(&env, SavingsGoalError::InsufficientBalance);
        }

        goal.current_amount -= amount;
        goal.is_complete = goal.current_amount >= goal.target_amount;

        env.storage()
            .persistent()
            .set(&DataKey::Goal(goal_id), &goal);

        GoalEvents::goal_withdrawn(&env, goal_id, &caller, amount, goal.current_amount);
        goal.current_amount
    }

    /// Returns milestone percentages already triggered for a goal.
    pub fn get_triggered_milestone_percents(env: Env, goal_id: u64) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::GoalMilestonesPercent(goal_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns whether a goal is currently locked against withdrawals.
    pub fn is_goal_locked(env: Env, goal_id: u64) -> bool {
        let goal: SavingsGoal = match env.storage().persistent().get(&DataKey::Goal(goal_id)) {
            Some(g) => g,
            None => return false,
        };
        goal.unlock_at > 0 && env.ledger().timestamp() < goal.unlock_at
    }

    /// Emits milestone events automatically when goal progress crosses thresholds.
    /// Call this after updating a goal's current_amount.
    pub fn check_and_emit_milestones(env: &Env, goal_id: u64) {
        let mut goal: SavingsGoal = match env.storage().persistent().get(&DataKey::Goal(goal_id)) {
            Some(g) => g,
            None => return,
        };
        let milestones = [25, 50, 75, 100];
        let mut triggered: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::GoalMilestonesPercent(goal_id))
            .unwrap_or(Vec::new(env));
        let mut progress = if goal.target_amount > 0 {
            (goal.current_amount * 100 / goal.target_amount) as u32
        } else {
            0
        };
        if progress > 100 {
            progress = 100;
        }
        let is_complete = goal.current_amount >= goal.target_amount;
        if goal.is_complete != is_complete {
            goal.is_complete = is_complete;
            env.storage().persistent().set(&DataKey::Goal(goal_id), &goal);
            if is_complete {
                GoalEvents::goal_completed(env, goal_id, &goal.user, goal.target_amount);
            }
            env.storage()
                .persistent()
                .set(&DataKey::Goal(goal_id), &goal);
        }
        for &milestone in milestones.iter() {
            if progress >= milestone && !triggered.contains(&milestone) {
                // Emit event
                GoalEvents::milestone_achieved_percent(env, goal_id, milestone);
                triggered.push_back(milestone);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::GoalMilestonesPercent(goal_id), &triggered);
    }
    // ...existing code...

    /// Partially withdraws funds from a savings goal.
    ///
    /// Updates the remaining current amount and goal completion state.
    /// The caller must be the goal owner.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address requesting the withdrawal (must be goal owner)
    /// * `goal_id` - The ID of the goal to withdraw from
    /// * `amount` - The amount to withdraw (must be > 0, <= current_amount)
    pub fn partial_withdraw(env: Env, caller: Address, goal_id: u64, amount: i128) {
        caller.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, SavingsGoalError::InvalidBatch);
        }

        let mut goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::InvalidBatch));

        // Verify caller is the goal owner
        if goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }

        // Verify goal is active
        if !goal.is_active {
            panic_with_error!(&env, SavingsGoalError::GoalNotActive);
        }

        let now = env.ledger().timestamp();
        if goal.unlock_at > 0 && now < goal.unlock_at {
            GoalEvents::goal_withdraw_locked(&env, goal_id, &caller, goal.unlock_at);
            panic_with_error!(&env, SavingsGoalError::GoalLocked);
        }

        // Verify sufficient balance
        if amount > goal.current_amount {
            panic_with_error!(&env, SavingsGoalError::InsufficientBalance);
        }

        // Update current amount
        goal.current_amount = goal.current_amount.checked_sub(amount).unwrap_or(0);

        // Update completion status
        let was_complete = goal.is_complete;
        goal.is_complete = goal.current_amount >= goal.target_amount;

        // If goal was complete but is no longer, it stays active
        if was_complete && !goal.is_complete {
            goal.is_active = true;
        }

        // Store updated goal
        env.storage()
            .persistent()
            .set(&DataKey::Goal(goal_id), &goal);

        // Update milestones if progress changed
        Self::check_and_emit_milestones(&env, goal_id);

        // Emit withdrawal event
        GoalEvents::partial_withdrawal(&env, goal_id, &caller, amount, goal.current_amount);
    }

    /// Updates the name of an existing savings goal.
    ///
    /// The caller must be the goal owner. Emits a rename event on success.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `caller` - The address requesting the rename (must be goal owner)
    /// * `goal_id` - The ID of the goal to rename
    /// * `new_name` - The new name for the goal
    pub fn update_goal_name(env: Env, caller: Address, goal_id: u64, new_name: Symbol) {
        caller.require_auth();

        let mut goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::InvalidBatch));

        // Verify caller is the goal owner
        if goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }

        let old_name = goal.goal_name.clone();
        goal.goal_name = new_name.clone();

        // Store updated goal
        env.storage()
            .persistent()
            .set(&DataKey::Goal(goal_id), &goal);

        // Emit rename event
        GoalEvents::goal_renamed(&env, goal_id, &old_name, &new_name);
    }

    /// Retrieves a savings goal by ID.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `goal_id` - The ID of the goal to retrieve
    ///
    /// # Returns
    /// * `Option<SavingsGoal>` - The goal if found
    pub fn get_goal(env: Env, goal_id: u64) -> Option<SavingsGoal> {
        env.storage().persistent().get(&DataKey::Goal(goal_id))
    }

    /// Gets current progress details for a savings goal.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `goal_id` - The ID of the goal to query
    ///
    /// # Returns
    /// * `Option<SavingsGoalProgress>` - Progress summary if goal exists
    pub fn get_goal_progress(env: Env, goal_id: u64) -> Option<SavingsGoalProgress> {
        env.storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .map(|goal: SavingsGoal| {
                let mut progress_percentage = if goal.target_amount > 0 {
                    (goal.current_amount * 100 / goal.target_amount) as u32
                } else {
                    0
                };
                if progress_percentage > 100 {
                    progress_percentage = 100;
                }
                let is_complete = goal.current_amount >= goal.target_amount;
                SavingsGoalProgress {
                    goal_id: goal.goal_id,
                    current_amount: goal.current_amount,
                    target_amount: goal.target_amount,
                    progress_percentage,
                    is_complete,
                }
            })
    }

    /// Retrieves all goal IDs for a specific user.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `user` - The user's address
    ///
    /// # Returns
    /// * `Vec<u64>` - Vector of goal IDs for the user
    pub fn get_user_goals(env: Env, user: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::UserGoals(user))
            .unwrap_or(Vec::new(&env))
    }

    /// Number of recurring auto-contribution cycles due between
    /// `last_contributed_at` and the current ledger time for a given interval.
    ///
    /// Use `604_800` seconds for a weekly schedule or `2_592_000` for monthly.
    /// Missed cycles are counted (not collapsed to one) so they can be settled
    /// safely in a single catch-up call.
    pub fn contributions_due(env: Env, last_contributed_at: u64, interval_seconds: u64) -> u64 {
        if interval_seconds == 0 {
            return 0;
        }

        let now = env.ledger().timestamp();
        if now <= last_contributed_at {
            0
        } else {
            (now - last_contributed_at) / interval_seconds
        }
    }

    /// Returns the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized")
    }

    /// Updates the admin address.
    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&env, &current_admin);

        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    /// Returns the last created batch ID.
    pub fn get_last_batch_id(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LastBatchId)
            .unwrap_or(0)
    }

    /// Returns the last created goal ID.
    pub fn get_last_goal_id(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LastGoalId)
            .unwrap_or(0)
    }

    /// Returns the total number of goals created.
    pub fn get_total_goals_created(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalGoalsCreated)
            .unwrap_or(0)
    }

    /// Returns the total number of batches processed.
    pub fn get_total_batches_processed(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalBatchesProcessed)
            .unwrap_or(0)
    }

    /// Retrieves a milestone by ID.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `milestone_id` - The ID of the milestone to retrieve
    ///
    /// # Returns
    /// * `Option<MilestoneAchievement>` - The milestone if found
    pub fn get_milestone(env: Env, milestone_id: u64) -> Option<MilestoneAchievement> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(milestone_id))
    }

    /// Retrieves all milestone IDs for a specific goal.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `goal_id` - The goal ID
    ///
    /// # Returns
    /// * `Vec<u64>` - Vector of milestone IDs for the goal
    pub fn get_goal_milestones(env: Env, goal_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::GoalMilestones(goal_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the last created milestone ID.
    pub fn get_last_milestone_id(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LastMilestoneId)
            .unwrap_or(0)
    }

    /// Returns the total number of milestones achieved.
    pub fn get_total_milestones_achieved(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalMilestonesAchieved)
            .unwrap_or(0)
    }

    // Internal helper to verify admin
    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        if *caller != admin {
            panic_with_error!(env, SavingsGoalError::Unauthorized);
        }
    }

    /// Sets prerequisite goal IDs for a specific goal. The caller must be the goal owner.
    /// Prevents circular dependencies when setting prerequisites.
    pub fn set_goal_prerequisites(env: Env, caller: Address, goal_id: u64, prereq_ids: Vec<u64>) {
        caller.require_auth();

        let goal: SavingsGoal = env
            .storage()
            .persistent()
            .get(&DataKey::Goal(goal_id))
            .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::GoalNotFound));

        if goal.user != caller {
            panic_with_error!(&env, SavingsGoalError::Unauthorized);
        }

        // Validate prerequisites exist, belong to the same owner, and do not form cycles
        for p in prereq_ids.iter() {
            if *p == goal_id {
                panic_with_error!(&env, SavingsGoalError::DependencyNotMet);
            }

            let pgoal: SavingsGoal = env
                .storage()
                .persistent()
                .get(&DataKey::Goal(*p))
                .unwrap_or_else(|| panic_with_error!(&env, SavingsGoalError::GoalNotFound));

            if pgoal.user != caller {
                // Disallow cross-user prerequisites
                panic_with_error!(&env, SavingsGoalError::Unauthorized);
            }

            // Check for circular dependency: is there a path from p -> goal_id ?
            if Self::has_dependency_path(&env, *p, goal_id) {
                panic_with_error!(&env, SavingsGoalError::DependencyNotMet);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::GoalPrereqs(goal_id), &prereq_ids);
    }

    // Depth-first search to detect if there's a path from `start` to `target`.
    fn has_dependency_path(env: &Env, start: u64, target: u64) -> bool {
        let mut stack: Vec<u64> = Vec::new(env);
        let mut visited: Vec<u64> = Vec::new(env);
        stack.push_back(start);

        while stack.len() > 0 {
            let node = stack.pop_back().unwrap();
            if node == target {
                return true;
            }
            if visited.contains(&node) {
                continue;
            }
            visited.push_back(node);

            let neighbors: Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::GoalPrereqs(node))
                .unwrap_or(Vec::new(env));

            for n in neighbors.iter() {
                if !visited.contains(&n) {
                    stack.push_back(*n);
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod test;

// Place test-only contract impl at the very end, outside all other blocks
#[cfg(test)]
#[contractimpl]
impl SavingsGoalsContract {
    /// Test-only: update a goal's current_amount for test setup.
    pub fn test_set_goal_current_amount(env: Env, goal_id: u64, amount: i128) {
        let contract_id = env.current_contract_address();
        env.as_contract(&contract_id, || {
            let key = crate::types::DataKey::Goal(goal_id);
            if let Some(mut goal) = env
                .storage()
                .persistent()
                .get::<crate::types::DataKey, crate::types::SavingsGoal>(&key)
            {
                goal.current_amount = amount;
                goal.is_complete = amount >= goal.target_amount;
                env.storage().persistent().set(&key, &goal);
            }
        });
    }
}
