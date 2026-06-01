use crate::storage::MAX_FEE_BPS;
use soroban_sdk::{contracterror, panic_with_error, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FeeValidationError {
    FeeTooLow = 1,
    FeeTooHigh = 2,
}

pub fn validate_fee(fee: i128, min_fee: i128, max_fee: i128) -> Result<(), FeeValidationError> {
    if fee < min_fee {
        return Err(FeeValidationError::FeeTooLow);
    }
    if fee > max_fee {
        return Err(FeeValidationError::FeeTooHigh);
    }
    Ok(())
}

pub fn validate_fee_percentage_bounds(env: &Env, fee_bps: u32) -> bool {
    if fee_bps == 0 {
        panic_with_error!(env, FeeValidationError::FeeTooLow);
    }
    if fee_bps > MAX_FEE_BPS {
        panic_with_error!(env, FeeValidationError::FeeTooHigh);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_fee() {
        assert!(validate_fee(50, 10, 100).is_ok());
    }

    #[test]
    fn test_fee_too_low() {
        assert_eq!(validate_fee(5, 10, 100), Err(FeeValidationError::FeeTooLow));
    }

    #[test]
    fn test_fee_too_high() {
        assert_eq!(
            validate_fee(200, 10, 100),
            Err(FeeValidationError::FeeTooHigh)
        );
    }
}
