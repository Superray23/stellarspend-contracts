#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env,
};

#[path = "../contracts/transactions.rs"]
mod transactions;

use transactions::{TimelockedTx, TransactionsContract, TransactionsContractClient};

fn setup_test_contract() -> (Env, Address, TransactionsContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    // Ensure deterministic starting timestamp.
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, admin, client)
}

#[test]
fn test_schedule_timelocked_transaction_stores_record_and_emits_event() {
    let (env, _admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 500;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let execute_at = env.ledger().timestamp() + 60;

    let scheduled: TimelockedTx =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    assert_eq!(scheduled.from, from);
    assert_eq!(scheduled.to, to);
    assert_eq!(scheduled.amount, amount);
    assert_eq!(scheduled.execute_at, execute_at);
    assert_eq!(scheduled.executed, false);
    assert_eq!(scheduled.canceled, false);
    assert_eq!(scheduled.executed_at, None);
    assert_eq!(scheduled.canceled_at, None);

    // Fetch via getter
    let fetched = client
        .get_timelocked_transaction(&scheduled.id)
        .expect("expected stored timelocked tx");
    assert_eq!(fetched.id, scheduled.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_cannot_schedule_with_past_or_current_timestamp() {
    let (env, _admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 100;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let now = env.ledger().timestamp();

    // Using execute_at <= now should be rejected.
    client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &now);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_cannot_schedule_with_invalid_amount() {
    let (env, _admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let invalid_amount: i128 = 0;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;
    let execute_at = env.ledger().timestamp() + 10;

    client.schedule_timelocked_transaction(
        &from,
        &to,
        &invalid_amount,
        &payload,
        &asset,
        &execute_at,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_cannot_execute_before_execute_at() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 250;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let execute_at = env.ledger().timestamp() + 300;
    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    // Even the admin cannot execute before the scheduled time.
    client.execute_timelocked_transaction(&admin, &scheduled.id);
}

#[test]
fn test_execute_after_execute_at_moves_balance_and_marks_executed() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Seed balance using existing admin-only setter.
    client.set_balance(&admin, &from, &1_000);

    let amount: i128 = 400;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let execute_at = env.ledger().timestamp() + 10;
    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    // Advance time to just after execute_at.
    env.ledger().set_timestamp(execute_at + 1);

    // Allow admin to execute on behalf of the user.
    client.execute_timelocked_transaction(&admin, &scheduled.id);

    let executed = client
        .get_timelocked_transaction(&scheduled.id)
        .expect("missing timelocked tx");
    assert!(executed.executed);
    assert!(!executed.canceled);
    assert_eq!(executed.executed_at, Some(execute_at + 1));
    assert_eq!(executed.canceled_at, None);

    // Balance should have moved.
    assert_eq!(client.get_balance(&from), 600);
    assert_eq!(client.get_balance(&to), 400);
}

#[test]
fn test_cancel_before_execution_prevents_later_execution() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    client.set_balance(&admin, &from, &1_000);

    let amount: i128 = 200;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let execute_at = env.ledger().timestamp() + 100;
    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    // User cancels before execution window.
    client.cancel_timelocked_transaction(&from, &scheduled.id);

    let cancelled = client
        .get_timelocked_transaction(&scheduled.id)
        .expect("missing timelocked tx");
    assert!(cancelled.canceled);
    assert!(!cancelled.executed);
    assert_eq!(cancelled.canceled_at, Some(env.ledger().timestamp()));
    assert_eq!(cancelled.executed_at, None);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_execute_fails_after_cancellation() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    client.set_balance(&admin, &from, &1_000);

    let amount: i128 = 200;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;
    let execute_at = env.ledger().timestamp() + 20;

    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    client.cancel_timelocked_transaction(&from, &scheduled.id);

    // Even after the time window, canceled transactions cannot execute.
    env.ledger().set_timestamp(execute_at + 1);
    client.execute_timelocked_transaction(&admin, &scheduled.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_only_owner_or_admin_can_cancel() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    client.set_balance(&admin, &from, &1_000);

    let amount: i128 = 100;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;

    let execute_at = env.ledger().timestamp() + 50;
    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    let outsider = Address::generate(&env);
    client.cancel_timelocked_transaction(&outsider, &scheduled.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_only_owner_or_admin_can_execute() {
    let (env, admin, client) = setup_test_contract();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    client.set_balance(&admin, &from, &1_000);

    let amount: i128 = 100;
    let payload = symbol_short!("pay");
    let asset: Option<Address> = None;
    let execute_at = env.ledger().timestamp() + 50;

    let scheduled =
        client.schedule_timelocked_transaction(&from, &to, &amount, &payload, &asset, &execute_at);

    env.ledger().set_timestamp(execute_at + 1);
    let outsider = Address::generate(&env);
    client.execute_timelocked_transaction(&outsider, &scheduled.id);
}
