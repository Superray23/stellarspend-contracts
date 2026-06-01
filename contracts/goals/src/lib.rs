pub fn create_goal(
    env: Env,
    owner: Address,
    name: String,
    target_amount: i128,
    priority: u32,
) -> u64 {
    owner.require_auth();

    let id = get_next_goal_id(&env);

    let goal = Goal {
        id,
        owner,
        name,
        target_amount,
        saved_amount: 0,
        priority,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Goal(id), &goal);

    id
}