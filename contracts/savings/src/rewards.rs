use soroban_sdk::{Address, Env};

use crate::{
    storage::DataKey,
    types::SavingsGoal,
};

pub fn claim_reward(
    env: &Env,
    user: Address,
    goal_id: u64,
) -> i128 {
    user.require_auth();

    let goal: SavingsGoal = env
        .storage()
        .instance()
        .get(&DataKey::Goal(goal_id))
        .expect("Goal not found");

    if !goal.completed {
        panic!("Goal not completed");
    }

    let already_claimed = env
        .storage()
        .instance()
        .has(&DataKey::RewardClaimed(
            user.clone(),
            goal_id,
        ));

    if already_claimed {
        panic!("Reward already claimed");
    }

    let reward: i128 = env
        .storage()
        .instance()
        .get(&DataKey::RewardAmount)
        .unwrap_or(0);

    env.storage()
        .instance()
        .set(
            &DataKey::RewardClaimed(
                user,
                goal_id,
            ),
            &true,
        );

    reward
}