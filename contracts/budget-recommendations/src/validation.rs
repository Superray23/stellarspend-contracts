//! Validation utilities for budget recommendations.

use soroban_sdk::{Env, Vec};

use crate::types::UserProfile;

// ─────────────────────────────────────────────
// Contribution amount constants
// ─────────────────────────────────────────────

/// Minimum allowed contribution in stroops (0.0001 XLM).
pub const MIN_CONTRIBUTION: i128 = 100_000;

/// Maximum allowed contribution in stroops (1 000 000 XLM).
pub const MAX_CONTRIBUTION: i128 = 1_000_000_000_000_i128;

// ─────────────────────────────────────────────
// Validation error types
// ─────────────────────────────────────────────

/// Validation error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// Invalid user ID
    InvalidUserId,
    /// Invalid income amount
    InvalidIncome,
    /// Invalid expenses amount
    InvalidExpenses,
    /// Invalid savings balance
    InvalidSavings,
    /// Invalid risk tolerance
    InvalidRiskTolerance,
    /// Contribution amount is zero
    ContributionZero,
    /// Contribution amount is negative
    ContributionNegative,
    /// Contribution amount is below minimum
    ContributionTooSmall,
    /// Contribution amount exceeds maximum
    ContributionTooLarge,
}

// ─────────────────────────────────────────────
// User profile validation (unchanged from before)
// ─────────────────────────────────────────────

/// Validates a user profile for budget recommendations.
///
/// Returns Ok(()) if valid, or a ValidationError if invalid.
pub fn validate_user_profile(_env: &Env, profile: &UserProfile) -> Result<(), ValidationError> {
    // Validate user ID
    if profile.user_id == 0 {
        return Err(ValidationError::InvalidUserId);
    }

    // Validate income (must be positive)
    if profile.monthly_income <= 0 {
        return Err(ValidationError::InvalidIncome);
    }

    // Validate expenses (must be non-negative)
    if profile.monthly_expenses < 0 {
        return Err(ValidationError::InvalidExpenses);
    }

    // Validate savings (must be non-negative)
    if profile.savings_balance < 0 {
        return Err(ValidationError::InvalidSavings);
    }

    // Validate risk tolerance (must be 1-5)
    if profile.risk_tolerance < 1 || profile.risk_tolerance > 5 {
        return Err(ValidationError::InvalidRiskTolerance);
    }

    Ok(())
}

/// Validates a batch of user profiles.
///
/// `env` is now a proper parameter instead of `Env::default()` — this
/// ensures the correct environment context is used in every call site.
pub fn validate_batch(env: &Env, profiles: &Vec<UserProfile>) -> Result<(), &'static str> {
    let count = profiles.len();

    if count == 0 {
        return Err("Batch cannot be empty");
    }

    if count > crate::types::MAX_BATCH_SIZE {
        return Err("Batch exceeds maximum size");
    }

    for profile in profiles.iter() {
        if validate_user_profile(env, &profile).is_err() {
            return Err("Invalid user profile in batch");
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────
// Contribution amount validation (new)
// ─────────────────────────────────────────────

/// Validates a single contribution amount.
///
/// Call this at every entry point that accepts a contribution so the
/// rules are enforced consistently and the error messages are uniform.
///
/// # Errors
/// Returns a [`ValidationError`] variant — never panics — so the caller
/// decides how to surface it (e.g. `map_err`, `?`, or `panic!`).
pub fn validate_contribution_amount(amount: i128) -> Result<(), ValidationError> {
    if amount == 0 {
        return Err(ValidationError::ContributionZero);
    }
    if amount < 0 {
        return Err(ValidationError::ContributionNegative);
    }
    if amount < MIN_CONTRIBUTION {
        return Err(ValidationError::ContributionTooSmall);
    }
    if amount > MAX_CONTRIBUTION {
        return Err(ValidationError::ContributionTooLarge);
    }
    Ok(())
}

/// Validates a slice of contribution amounts.
///
/// Fails fast on the first invalid entry, which keeps error messages
/// specific rather than accumulating all failures at once.
pub fn validate_contribution_amounts(amounts: &[i128]) -> Result<(), ValidationError> {
    for &amount in amounts {
        validate_contribution_amount(amount)?;
    }
    Ok(())
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    // ── helpers ──────────────────────────────

    fn create_test_profile(env: &Env, user_id: u64, income: i128, expenses: i128) -> UserProfile {
        UserProfile {
            user_id,
            address: Address::generate(env),
            monthly_income: income,
            monthly_expenses: expenses,
            savings_balance: 0,
            spending_categories: Symbol::new(env, "food,transport"),
            risk_tolerance: 3,
        }
    }

    // ── existing profile tests (unchanged) ───

    #[test]
    fn test_validate_user_profile_valid() {
        let env = Env::default();
        let profile = create_test_profile(&env, 1, 100000, 50000);
        assert!(validate_user_profile(&env, &profile).is_ok());
    }

    #[test]
    fn test_validate_user_profile_invalid_user_id() {
        let env = Env::default();
        let mut profile = create_test_profile(&env, 1, 100000, 50000);
        profile.user_id = 0;
        assert_eq!(
            validate_user_profile(&env, &profile),
            Err(ValidationError::InvalidUserId)
        );
    }

    #[test]
    fn test_validate_user_profile_invalid_income() {
        let env = Env::default();
        let profile = create_test_profile(&env, 1, 0, 50000);
        assert_eq!(
            validate_user_profile(&env, &profile),
            Err(ValidationError::InvalidIncome)
        );
    }

    #[test]
    fn test_validate_user_profile_invalid_expenses() {
        let env = Env::default();
        let mut profile = create_test_profile(&env, 1, 100000, 50000);
        profile.monthly_expenses = -1;
        assert_eq!(
            validate_user_profile(&env, &profile),
            Err(ValidationError::InvalidExpenses)
        );
    }

    #[test]
    fn test_validate_user_profile_invalid_risk_tolerance() {
        let env = Env::default();
        let mut profile = create_test_profile(&env, 1, 100000, 50000);
        profile.risk_tolerance = 6;
        assert_eq!(
            validate_user_profile(&env, &profile),
            Err(ValidationError::InvalidRiskTolerance)
        );
    }

    // ── validate_batch: env param fix ────────

    #[test]
    fn test_validate_batch_empty_fails() {
        let env = Env::default();
        let profiles: Vec<UserProfile> = Vec::new(&env);
        assert_eq!(
            validate_batch(&env, &profiles),
            Err("Batch cannot be empty")
        );
    }

    #[test]
    fn test_validate_batch_valid_profiles() {
        let env = Env::default();
        let mut profiles = Vec::new(&env);
        profiles.push_back(create_test_profile(&env, 1, 100_000, 40_000));
        profiles.push_back(create_test_profile(&env, 2, 200_000, 80_000));
        assert!(validate_batch(&env, &profiles).is_ok());
    }

    #[test]
    fn test_validate_batch_rejects_invalid_profile() {
        let env = Env::default();
        let mut profiles = Vec::new(&env);
        profiles.push_back(create_test_profile(&env, 1, 100_000, 40_000));
        // user_id = 0 is invalid
        profiles.push_back(create_test_profile(&env, 0, 100_000, 40_000));
        assert_eq!(
            validate_batch(&env, &profiles),
            Err("Invalid user profile in batch")
        );
    }

    // ── contribution amount: valid ────────────

    #[test]
    fn test_contribution_minimum_is_valid() {
        assert!(validate_contribution_amount(MIN_CONTRIBUTION).is_ok());
    }

    #[test]
    fn test_contribution_maximum_is_valid() {
        assert!(validate_contribution_amount(MAX_CONTRIBUTION).is_ok());
    }

    #[test]
    fn test_contribution_typical_amount_is_valid() {
        assert!(validate_contribution_amount(10_000_000).is_ok()); // 1 XLM
    }

    // ── contribution amount: invalid ──────────

    #[test]
    fn test_contribution_zero_rejected() {
        assert_eq!(
            validate_contribution_amount(0),
            Err(ValidationError::ContributionZero)
        );
    }

    #[test]
    fn test_contribution_negative_rejected() {
        assert_eq!(
            validate_contribution_amount(-500),
            Err(ValidationError::ContributionNegative)
        );
    }

    #[test]
    fn test_contribution_below_minimum_rejected() {
        assert_eq!(
            validate_contribution_amount(MIN_CONTRIBUTION - 1),
            Err(ValidationError::ContributionTooSmall)
        );
    }

    #[test]
    fn test_contribution_above_maximum_rejected() {
        assert_eq!(
            validate_contribution_amount(MAX_CONTRIBUTION + 1),
            Err(ValidationError::ContributionTooLarge)
        );
    }

    // ── batch contributions ───────────────────

    #[test]
    fn test_contribution_batch_all_valid() {
        let amounts = [MIN_CONTRIBUTION, 10_000_000, MAX_CONTRIBUTION];
        assert!(validate_contribution_amounts(&amounts).is_ok());
    }

    #[test]
    fn test_contribution_batch_fails_on_first_zero() {
        let amounts = [10_000_000, 0, MIN_CONTRIBUTION];
        assert_eq!(
            validate_contribution_amounts(&amounts),
            Err(ValidationError::ContributionZero)
        );
    }

    #[test]
    fn test_contribution_empty_batch_is_valid() {
        assert!(validate_contribution_amounts(&[]).is_ok());
    }
}