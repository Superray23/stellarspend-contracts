use crate::{BatchHistoryContract, BatchHistoryContractClient};
use soroban_sdk::{
    testutils::Address as _,
    vec,
    Address,
    Env,
};

fn setup() -> (Env, BatchHistoryContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(BatchHistoryContract, ());
    let client = BatchHistoryContractClient::new(&env, &contract_id);

    (env, client)
}

#[test]
fn test_batch_retrieval_single_user() {
    let (env, client) = setup();

    let requester = Address::generate(&env);
    let user = Address::generate(&env);

    let users = vec![&env, user.clone()];
    let results = client.retrieve_histories(&requester, &users);

    assert_eq!(results.len(), 1);

    let history = results.get(0).unwrap();

    assert_eq!(history.user, user);
    assert_eq!(history.transactions.len(), 0);
}

#[test]
fn test_batch_retrieval_multiple_users() {
    let (env, client) = setup();

    let requester = Address::generate(&env);

    let user_1 = Address::generate(&env);
    let user_2 = Address::generate(&env);
    let user_3 = Address::generate(&env);

    let users = vec![
        &env,
        user_1.clone(),
        user_2.clone(),
        user_3.clone(),
    ];

    let results = client.retrieve_histories(&requester, &users);

    assert_eq!(results.len(), 3);

    assert_eq!(results.get(0).unwrap().user, user_1);
    assert_eq!(results.get(1).unwrap().user, user_2);
    assert_eq!(results.get(2).unwrap().user, user_3);
}

#[test]
fn test_batch_retrieval_empty_input() {
    let (env, client) = setup();

    let requester = Address::generate(&env);

    let users: soroban_sdk::Vec<Address> = vec![&env];

    let results = client.retrieve_histories(&requester, &users);

    assert_eq!(results.len(), 0);
}

#[test]
fn test_batch_retrieval_preserves_order() {
    let (env, client) = setup();

    let requester = Address::generate(&env);

    let user_1 = Address::generate(&env);
    let user_2 = Address::generate(&env);
    let user_3 = Address::generate(&env);

    let users = vec![
        &env,
        user_2.clone(),
        user_1.clone(),
        user_3.clone(),
    ];

    let results = client.retrieve_histories(&requester, &users);

    assert_eq!(results.len(), 3);

    assert_eq!(results.get(0).unwrap().user, user_2);
    assert_eq!(results.get(1).unwrap().user, user_1);
    assert_eq!(results.get(2).unwrap().user, user_3);
}

#[test]
fn test_batch_retrieval_duplicate_users() {
    let (env, client) = setup();

    let requester = Address::generate(&env);
    let user = Address::generate(&env);

    let users = vec![
        &env,
        user.clone(),
        user.clone(),
    ];

    let results = client.retrieve_histories(&requester, &users);

    assert_eq!(results.len(), 2);

    assert_eq!(results.get(0).unwrap().user, user);
    assert_eq!(results.get(1).unwrap().user, user);
}