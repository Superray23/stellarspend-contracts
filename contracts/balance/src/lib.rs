#![no_std]

use shared::utils::validate_amount;
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Balance(Address),
}
//  AlreadyInitialized = 1,

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BalanceError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
}

impl From<BalanceError> for soroban_sdk::Error {
    fn from(value: BalanceError) -> Self {
        soroban_sdk::Error::from_contract_error(value as u32)
    }
}

#[contract]
pub struct BalanceContract;

#[contractimpl]
impl BalanceContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, BalanceError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn set_user_balance(env: Env, admin: Address, user: Address, amount: i128) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        validate_amount(amount).unwrap_or_else(|_| {
            panic_with_error!(&env, BalanceError::InvalidAmount);
        });

        if amount == 0 {
            env.storage().persistent().remove(&DataKey::Balance(user));
        } else {
            env.storage()
                .persistent()
                .set(&DataKey::Balance(user), &amount);
        }
    }

    pub fn get_user_balance(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(user))
            .unwrap_or(0)
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, BalanceError::Unauthorized));

        if admin != *caller {
            panic_with_error!(env, BalanceError::Unauthorized);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{BalanceContract, BalanceContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup() -> (Env, Address, BalanceContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(BalanceContract, ());
        let client = BalanceContractClient::new(&env, &contract_id);
        client.initialize(&admin);
        (env, admin, client)
    }

    #[test]
    fn get_user_balance_returns_zero_when_missing() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);

        assert_eq!(client.get_user_balance(&user), 0);
    }

    #[test]
    fn get_user_balance_returns_stored_value() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.set_user_balance(&admin, &user, &750i128);

        assert_eq!(client.get_user_balance(&user), 750);
    }

    #[test]
    fn setting_zero_clears_balance_back_to_default() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.set_user_balance(&admin, &user, &750i128);
        client.set_user_balance(&admin, &user, &0i128);

        assert_eq!(client.get_user_balance(&user), 0);
    }

    #[test]
    #[should_panic]
    fn set_user_balance_rejects_negative_amounts() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.set_user_balance(&admin, &user, &-1i128);
    }
}
