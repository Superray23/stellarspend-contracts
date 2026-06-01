use soroban_sdk::Env;
use crate::storage;

pub fn get_max_fee(env: &Env) -> i128 {
    storage::get_max_fee(env).unwrap_or(10_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_get_max_fee_default() {
        let env = Env::default();
        let fee = get_max_fee(&env);
        assert!(fee > 0);
    }
}
