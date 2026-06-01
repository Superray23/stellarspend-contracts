use soroban_sdk::{
    contract,
    contractimpl,
    Address,
    Env,
};

#[contract]
pub struct SavingsContract;

#[contractimpl]
impl SavingsContract {
    pub fn claim_reward(
        env: Env,
        user: Address,
        goal_id: u64,
    ) -> i128 {
        crate::rewards::claim_reward(
            &env,
            user,
            goal_id,
        )
    }

    pub fn set_reward_amount(
        env: Env,
        amount: i128,
    ) {
        crate::rewards::set_reward_amount(
            &env,
            amount,
        );
    }
}