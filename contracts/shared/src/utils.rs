use soroban_sdk::{Env, Symbol};

/// Shared validation errors for simple reusable helpers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    NegativeAmount,
    InvalidAddress,
}

/// Validates that an amount is not negative.
pub fn validate_amount(amount: i128) -> Result<(), ValidationError> {
    if amount < 0 {
        Err(ValidationError::NegativeAmount)
    } else {
        Ok(())
    }
}

/// Validates a user address string format.
///
/// Accepts 56-character classic Stellar-style strings with `G` account
/// or `C` contract prefixes and only base32 characters (`A-Z`, `2-7`).
pub fn validate_user_address(address: &soroban_sdk::String) -> Result<(), ValidationError> {
    if address.len() != 56 {
        return Err(ValidationError::InvalidAddress);
    }

    let mut bytes = [0u8; 56];
    address.copy_into_slice(&mut bytes);

    let prefix = bytes[0];
    if prefix != b'G' && prefix != b'C' {
        return Err(ValidationError::InvalidAddress);
    }

    for b in bytes.iter() {
        let is_upper_alpha = *b >= b'A' && *b <= b'Z';
        let is_base32_digit = *b >= b'2' && *b <= b'7';
        if !is_upper_alpha && !is_base32_digit {
            return Err(ValidationError::InvalidAddress);
        }
    }

    Ok(())
}

/// Increment a counter in storage and return the new value
pub fn increment_counter(env: &Env, counter_key: &Symbol) -> u64 {
    let mut counter: u64 = env.storage().persistent().get(counter_key).unwrap_or(0);

    counter += 1;
    env.storage().persistent().set(counter_key, &counter);

    counter
}

#[cfg(test)]
mod tests {
    use super::{increment_counter, validate_amount, validate_user_address, ValidationError};
    use soroban_sdk::{contract, contractimpl, Env, String, Symbol};

    #[contract]
    struct TestContract;

    #[contractimpl]
    impl TestContract {
        pub fn noop() {}
    }

    #[test]
    fn accepts_zero_and_positive_amounts() {
        assert_eq!(validate_amount(0), Ok(()));
        assert_eq!(validate_amount(1), Ok(()));
        assert_eq!(validate_amount(1_000_000), Ok(()));
    }

    #[test]
    fn rejects_negative_amounts() {
        assert_eq!(validate_amount(-1), Err(ValidationError::NegativeAmount));
        assert_eq!(validate_amount(-99), Err(ValidationError::NegativeAmount));
    }

    #[test]
    fn accepts_valid_user_address() {
        let env = Env::default();
        let address = String::from_str(
            &env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        assert_eq!(validate_user_address(&address), Ok(()));
    }

    #[test]
    fn rejects_invalid_user_address() {
        let env = Env::default();

        let empty = String::from_str(&env, "");
        assert_eq!(
            validate_user_address(&empty),
            Err(ValidationError::InvalidAddress)
        );

        let bad_prefix = String::from_str(
            &env,
            "XAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        assert_eq!(
            validate_user_address(&bad_prefix),
            Err(ValidationError::InvalidAddress)
        );

        let bad_chars = String::from_str(&env, "GINVALID!ADDRESS");
        assert_eq!(
            validate_user_address(&bad_chars),
            Err(ValidationError::InvalidAddress)
        );
    }

    #[test]
    fn increment_counter_works() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let counter_key = Symbol::new(&env, "test_counter");

        env.as_contract(&contract_id, || {
            assert_eq!(increment_counter(&env, &counter_key), 1);
            assert_eq!(increment_counter(&env, &counter_key), 2);
            assert_eq!(increment_counter(&env, &counter_key), 3);
        });
    }
}
