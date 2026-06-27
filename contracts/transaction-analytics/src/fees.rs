//! # Fee Calculation Engine
//!
//! Implements dynamic fee calculation for transactions with configurable fee structures.
//! Supports percentage-based fees, tiered pricing, fee cap enforcement, per-operation
//! configuration, fee distribution splitting, and a fee pause mechanism.

use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::types::{
    AnalyticsEvents, DataKey, FeeCalculationResult, FeeConfig, FeeRecipientShare, FeeTier,
    ValidationError,
};

/// Returns true if fees are currently paused.
pub fn is_fee_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::FeePaused)
        .unwrap_or(false)
}

/// Sets the fee pause flag (admin only).
pub fn set_fee_paused(env: &Env, admin: &Address, paused: bool) -> Result<(), ValidationError> {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("Contract not initialized");
    if *admin != stored_admin {
        return Err(ValidationError::InvalidAddress);
    }
    env.storage().instance().set(&DataKey::FeePaused, &paused);
    AnalyticsEvents::fee_pause_toggled(env, admin, paused);
    Ok(())
}

/// Validates that the sum of recipient shares is exactly 10000 (100%).
pub fn validate_recipient_shares(shares: &Vec<FeeRecipientShare>) -> Result<(), ValidationError> {
    let mut total: u32 = 0;
    for s in shares.iter() {
        total = total
            .checked_add(s.share_bps)
            .ok_or(ValidationError::InvalidAmount)?;
    }
    if total != 10000 {
        return Err(ValidationError::InvalidAmount);
    }
    Ok(())
}

/// Splits a fee amount among recipients and emits events.
pub fn distribute_fee(env: &Env, fee_amount: i128, shares: &Vec<FeeRecipientShare>) {
    validate_recipient_shares(shares).expect("Invalid recipient shares");
    let mut distributed = 0i128;
    for (i, s) in shares.iter().enumerate() {
        let is_last = (i as u32) == (shares.len() - 1);
        let share_amt = if is_last {
            fee_amount - distributed
        } else {
            let amt = (fee_amount * (s.share_bps as i128)) / 10000;
            distributed += amt;
            amt
        };
        AnalyticsEvents::fee_distributed(env, &s.recipient, share_amt, s.share_bps);
    }
}

/// Calculates fees for a single transaction based on the current fee configuration.
pub fn calculate_transaction_fee(
    _env: &Env,
    amount: i128,
    fee_config: &FeeConfig,
) -> FeeCalculationResult {
    if amount <= 0 {
        return FeeCalculationResult {
            gross_amount: amount,
            fee_amount: 0,
            net_amount: amount,
            fee_percentage_bps: 0,
        };
    }

    let fee_amount = match &fee_config.fee_model {
        crate::types::FeeModel::Flat(flat_fee) => *flat_fee,
        crate::types::FeeModel::Percentage(percentage_bps) => {
            (amount * (*percentage_bps as i128)) / 10000
        }
        crate::types::FeeModel::Tiered(tiers) => calculate_tiered_fee(amount, tiers),
    };

    let constrained_fee = constrain_fee_amount(fee_amount, fee_config);
    let final_fee = if constrained_fee > amount {
        amount
    } else {
        constrained_fee
    };

    FeeCalculationResult {
        gross_amount: amount,
        fee_amount: final_fee,
        net_amount: amount - final_fee,
        fee_percentage_bps: calculate_effective_rate(amount, final_fee),
    }
}

fn calculate_tiered_fee(amount: i128, tiers: &Vec<FeeTier>) -> i128 {
    if tiers.is_empty() {
        return 0;
    }

    let mut applicable_tier = tiers.get(0).unwrap();
    for tier in tiers.iter() {
        if amount >= tier.threshold {
            applicable_tier = tier;
        } else {
            break;
        }
    }

    match &applicable_tier.fee_model {
        crate::types::FeeModel::Flat(flat_fee) => *flat_fee,
        crate::types::FeeModel::Percentage(percentage_bps) => {
            (amount * (*percentage_bps as i128)) / 10000
        }
        crate::types::FeeModel::Tiered(_) => {
            (amount * (applicable_tier.default_percentage_bps as i128)) / 10000
        }
    }
}

fn constrain_fee_amount(calculated_fee: i128, config: &FeeConfig) -> i128 {
    let mut constrained_fee = calculated_fee;

    if let Some(min_fee) = config.min_fee {
        if constrained_fee < min_fee as i128 {
            constrained_fee = min_fee as i128;
        }
    }

    if let Some(max_fee) = config.max_fee {
        if constrained_fee > max_fee as i128 {
            constrained_fee = max_fee as i128;
        }
    }

    constrained_fee
}

fn calculate_effective_rate(gross_amount: i128, fee_amount: i128) -> u32 {
    if gross_amount == 0 {
        return 0;
    }
    ((fee_amount * 10000) / gross_amount) as u32
}

pub fn calculate_batch_fees(
    env: &Env,
    amounts: &Vec<i128>,
    fee_config: &FeeConfig,
) -> Vec<FeeCalculationResult> {
    let mut results = Vec::new(env);
    for amount in amounts.iter() {
        results.push_back(calculate_transaction_fee(env, amount, fee_config));
    }
    results
}

pub fn validate_fee_config(config: &FeeConfig) -> Result<(), ValidationError> {
    match &config.fee_model {
        crate::types::FeeModel::Percentage(percentage_bps) => {
            if *percentage_bps > 10000 {
                return Err(ValidationError::InvalidPercentage);
            }
        }
        crate::types::FeeModel::Tiered(tiers) => {
            for tier in tiers.iter() {
                if tier.threshold < 0 {
                    return Err(ValidationError::InvalidAmount);
                }
                if let crate::types::FeeModel::Percentage(percentage_bps) = &tier.fee_model {
                    if *percentage_bps > 10000 {
                        return Err(ValidationError::InvalidPercentage);
                    }
                }
            }

            let mut prev_threshold = 0i128;
            for tier in tiers.iter() {
                if tier.threshold < prev_threshold {
                    return Err(ValidationError::InvalidAmount);
                }
                prev_threshold = tier.threshold;
            }
        }
        _ => {}
    }

    if let Some(min_fee) = config.min_fee {
        if min_fee > i64::MAX as u64 {
            return Err(ValidationError::InvalidAmount);
        }
    }

    if let Some(max_fee) = config.max_fee {
        if max_fee > i64::MAX as u64 {
            return Err(ValidationError::InvalidAmount);
        }
    }

    if let (Some(min_fee), Some(max_fee)) = (config.min_fee, config.max_fee) {
        if min_fee > max_fee {
            return Err(ValidationError::InvalidAmount);
        }
    }

    Ok(())
}

pub fn store_fee_config(env: &Env, config: &FeeConfig) -> Result<(), ValidationError> {
    validate_fee_config(config)?;
    env.storage()
        .instance()
        .set(&DataKey::CurrentFeeConfig, config);
    Ok(())
}

pub fn get_current_fee_config(env: &Env) -> Option<FeeConfig> {
    env.storage().instance().get(&DataKey::CurrentFeeConfig)
}

pub fn get_operation_fee_config(env: &Env, operation: &Symbol) -> Option<FeeConfig> {
    env.storage()
        .instance()
        .get(&DataKey::OperationFeeConfig(operation.clone()))
}

pub fn store_operation_fee_config(
    env: &Env,
    operation: &Symbol,
    config: &FeeConfig,
) -> Result<(), ValidationError> {
    validate_fee_config(config)?;
    env.storage()
        .instance()
        .set(&DataKey::OperationFeeConfig(operation.clone()), config);
    Ok(())
}

pub fn deduct_fees(env: &Env, gross_amount: i128) -> FeeCalculationResult {
    let config = get_current_fee_config(env).unwrap_or(default_fee_config());
    let result = calculate_transaction_fee(env, gross_amount, &config);

    AnalyticsEvents::fee_deducted(
        env,
        result.gross_amount,
        result.fee_amount,
        result.net_amount,
        result.fee_percentage_bps,
    );

    result
}

fn default_fee_config() -> FeeConfig {
    use crate::types::FeeModel;

    FeeConfig {
        fee_model: FeeModel::Percentage(10),
        min_fee: Some(1),
        max_fee: None,
        enabled: true,
        description: Some(Symbol::new(&Env::default(), "Default 0.1% fee")),
    }
}

pub fn update_fee_config(
    env: &Env,
    admin: &Address,
    new_config: FeeConfig,
) -> Result<(), ValidationError> {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("Contract not initialized");

    if *admin != stored_admin {
        return Err(ValidationError::InvalidAddress);
    }

    validate_fee_config(&new_config)?;
    store_fee_config(env, &new_config)?;
    Ok(())
}

pub fn update_operation_fee_config(
    env: &Env,
    admin: &Address,
    operation: &Symbol,
    new_config: FeeConfig,
) -> Result<(), ValidationError> {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("Contract not initialized");
    if *admin != stored_admin {
        return Err(ValidationError::InvalidAddress);
    }

    let previous = get_operation_fee_config(env, operation);
    validate_fee_config(&new_config)?;
    store_operation_fee_config(env, operation, &new_config)?;
    AnalyticsEvents::operation_fee_updated(env, admin, operation, previous, new_config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeeModel, FeeTier};
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_flat_fee_calculation() {
        let env = Env::default();
        let config = FeeConfig {
            fee_model: FeeModel::Flat(100),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };

        let result = calculate_transaction_fee(&env, 1000, &config);
        assert_eq!(result.gross_amount, 1000);
        assert_eq!(result.fee_amount, 100);
        assert_eq!(result.net_amount, 900);
    }

    #[test]
    fn test_percentage_fee_calculation() {
        let env = Env::default();
        let config = FeeConfig {
            fee_model: FeeModel::Percentage(50),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };

        let result = calculate_transaction_fee(&env, 1000, &config);
        assert_eq!(result.gross_amount, 1000);
        assert_eq!(result.fee_amount, 5);
        assert_eq!(result.net_amount, 995);
    }

    #[test]
    fn test_min_max_constraints() {
        let env = Env::default();
        let config = FeeConfig {
            fee_model: FeeModel::Percentage(1),
            min_fee: Some(10),
            max_fee: Some(100),
            enabled: true,
            description: None,
        };

        let result = calculate_transaction_fee(&env, 50, &config);
        assert_eq!(result.fee_amount, 10);

        let result = calculate_transaction_fee(&env, 1_000_000, &config);
        assert_eq!(result.fee_amount, 100);
    }

    #[test]
    fn test_zero_negative_amount() {
        let env = Env::default();
        let config = FeeConfig {
            fee_model: FeeModel::Percentage(100),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };

        let result = calculate_transaction_fee(&env, 0, &config);
        assert_eq!(result.fee_amount, 0);
        assert_eq!(result.net_amount, 0);

        let result = calculate_transaction_fee(&env, -100, &config);
        assert_eq!(result.fee_amount, 0);
        assert_eq!(result.net_amount, -100);
    }

    #[test]
    fn test_tiered_fee_calculation() {
        let env = Env::default();
        let mut tiers = Vec::new(&env);
        tiers.push_back(FeeTier {
            threshold: 0,
            fee_model: FeeModel::Percentage(100),
            default_percentage_bps: 100,
        });
        tiers.push_back(FeeTier {
            threshold: 101,
            fee_model: FeeModel::Percentage(50),
            default_percentage_bps: 50,
        });

        let config = FeeConfig {
            fee_model: FeeModel::Tiered(tiers),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };

        let result = calculate_transaction_fee(&env, 50, &config);
        assert_eq!(result.fee_amount, 0);

        let result = calculate_transaction_fee(&env, 200, &config);
        assert_eq!(result.fee_amount, 1);
    }

    #[test]
    fn test_fee_constraint_validation() {
        let mut config = FeeConfig {
            fee_model: FeeModel::Percentage(10),
            min_fee: Some(100),
            max_fee: Some(50),
            enabled: true,
            description: None,
        };

        assert!(validate_fee_config(&config).is_err());

        config.max_fee = Some(150);
        assert!(validate_fee_config(&config).is_ok());
    }

    #[test]
    fn test_percentage_limit_validation() {
        let config = FeeConfig {
            fee_model: FeeModel::Percentage(10001),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };

        assert!(validate_fee_config(&config).is_err());
    }

    #[test]
    fn test_fee_distribution_splits_correctly() {
        let env = Env::default();
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let shares = [
            FeeRecipientShare {
                recipient: r1.clone(),
                share_bps: 6000,
            },
            FeeRecipientShare {
                recipient: r2.clone(),
                share_bps: 4000,
            },
        ];
        let shares_vec = Vec::from_array(&env, &shares);
        distribute_fee(&env, 1000, &shares_vec);
        let events = env.events().all();
        assert!(events.iter().any(|e| e.topics().contains(&r1)));
        assert!(events.iter().any(|e| e.topics().contains(&r2)));
    }

    #[test]
    fn test_fee_pausing_mechanism() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.storage().instance().set(&DataKey::Admin, &admin);

        let config = FeeConfig {
            fee_model: FeeModel::Percentage(100),
            min_fee: None,
            max_fee: None,
            enabled: true,
            description: None,
        };
        store_fee_config(&env, &config).unwrap();

        let result = calculate_transaction_fee(&env, 10000, &config);
        assert_eq!(result.fee_amount, 100);

        set_fee_paused(&env, &admin, true).unwrap();
        let result_paused = calculate_transaction_fee(&env, 10000, &config);
        assert_eq!(result_paused.fee_amount, 0);

        set_fee_paused(&env, &admin, false).unwrap();
        let result_resumed = calculate_transaction_fee(&env, 10000, &config);
        assert_eq!(result_resumed.fee_amount, 100);
    }
}
