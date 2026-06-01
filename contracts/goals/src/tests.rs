#[test]
fn goals_are_returned_in_priority_order() {
    let env = Env::default();

    let owner = Address::generate(&env);

    create_goal(
        env.clone(),
        owner.clone(),
        String::from_str(&env, "Emergency"),
        1000,
        1,
    );

    create_goal(
        env.clone(),
        owner.clone(),
        String::from_str(&env, "House"),
        5000,
        5,
    );

    create_goal(
        env.clone(),
        owner.clone(),
        String::from_str(&env, "Car"),
        3000,
        3,
    );

    let goals = get_goals_by_priority(
        env.clone(),
        owner.clone(),
    );

    assert_eq!(goals.get(0).unwrap().priority, 5);
    assert_eq!(goals.get(1).unwrap().priority, 3);
    assert_eq!(goals.get(2).unwrap().priority, 1);
}