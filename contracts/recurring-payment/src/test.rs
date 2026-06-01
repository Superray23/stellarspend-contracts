#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env};

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (Address, token::Client<'a>) {
    let addr = e.register_stellar_asset_contract(admin.clone());
    (addr.clone(), token::Client::new(e, &addr))
}

#[test]
fn test_recurring_payment_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, token_client) = create_token_contract(&env, &admin);
    let amount = 1000i128;
    let interval = 3600u64; // 1 hour
    let start_time = 1000u64;

    token_client.mint(&sender, &5000i128);

    let contract_id = env.register_contract(None, RecurringPaymentContract);
    let client = RecurringPaymentContractClient::new(&env, &contract_id);

    // 1. Create payment
    let payment_id = client.create_payment(
        &sender,
        &recipient,
        &token_addr,
        &amount,
        &interval,
        &start_time,
    );
    assert_eq!(payment_id, 1);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.next_execution, start_time);
    assert!(payment.active);
    assert_eq!(payment.execution_count, 0);

    // 2. Try to execute too early
    env.ledger().set_timestamp(start_time - 1);
    // client.execute_payment(&payment_id); // This should panic

    // 3. Execute at start_time
    env.ledger().set_timestamp(start_time);
    client.execute_payment(&payment_id);

    assert_eq!(token_client.balance(&sender), 4000);
    assert_eq!(token_client.balance(&recipient), 1000);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.next_execution, start_time + interval);
    assert_eq!(payment.execution_count, 1);

    // 4. Cancel payment
    client.cancel_payment(&payment_id);
    let payment = client.get_payment(&payment_id);
    assert!(!payment.active);

    // 5. Try to execute canceled payment
    env.ledger().set_timestamp(start_time + interval);
    // client.execute_payment(&payment_id); // This should panic
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_create_with_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    let contract_id = env.register_contract(None, RecurringPaymentContract);
    let client = RecurringPaymentContractClient::new(&env, &contract_id);

    client.create_payment(&sender, &recipient, &token, &0, &3600, &1000);
}

#[test]
fn test_execute_with_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, token_client) = create_token_contract(&env, &admin);
    let amount = 1000i128;
    let interval = 3600u64;
    let start_time = 1000u64;

    token_client.mint(&sender, &5000i128);

    let contract_id = env.register_contract(None, RecurringPaymentContract);
    let client = RecurringPaymentContractClient::new(&env, &contract_id);

    client.create_payment(
        &sender,
        &recipient,
        &token_addr,
        &amount,
        &interval,
        &start_time,
    );

    // Set time way ahead (e.g., 2.5 intervals ahead)
    env.ledger().set_timestamp(start_time + interval * 2 + 500);
    client.execute_payment(&1);

    let payment = client.get_payment(&1);
    // next_execution should be start_time + 3 * interval
    assert_eq!(payment.next_execution, start_time + 3 * interval);
    assert_eq!(token_client.balance(&recipient), 1000);
    assert_eq!(payment.execution_count, 1);

    #[test]
    fn test_missed_payment_increments_count() {
        let env = Env::default();
        env.mock_all_auths();

        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token.address();

        // Fund sender with less than required amount
        let token_client = token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&sender, &50);

        let contract_id = env.register(RecurringPaymentContract, ());
        let client = RecurringPaymentContractClient::new(&env, &contract_id);

        let payment_id = client.create_payment(
            &sender,
            &recipient,
            &token_address,
            &100,
            &3600,
            &0,
        );

        // Try to execute with insufficient funds — should panic
        let result = std::panic::catch_unwind(|| {
            client.execute_payment(&payment_id);
        });
        assert!(result.is_err());

        // Check missed_count incremented
        let payment = client.get_payment(&payment_id);
        assert_eq!(payment.missed_count, 1);
        assert!(payment.last_missed_at > 0);
    }

    #[test]
    fn test_successful_execution_resets_missed_count() {
        let env = Env::default();
        env.mock_all_auths();

        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token.address();

        let token_client = token::StellarAssetClient::new(&env, &token_address);

        let contract_id = env.register(RecurringPaymentContract, ());
        let client = RecurringPaymentContractClient::new(&env, &contract_id);

        let payment_id = client.create_payment(
            &sender,
            &recipient,
            &token_address,
            &100,
            &3600,
            &0,
        );

        // First cause a miss with insufficient funds
        token_client.mint(&sender, &50);
        let _ = std::panic::catch_unwind(|| {
            client.execute_payment(&payment_id);
        });

        // Now fund properly and execute successfully
        token_client.mint(&sender, &200);
        client.execute_payment(&payment_id);

        // missed_count should be reset to 0
        let payment = client.get_payment(&payment_id);
        assert_eq!(payment.missed_count, 0);
        assert_eq!(payment.last_missed_at, 0);
    }

    #[test]
    fn test_missed_count_increments_multiple_times() {
        let env = Env::default();
        env.mock_all_auths();

        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token.address();

        let token_client = token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&sender, &10); // not enough for 100

        let contract_id = env.register(RecurringPaymentContract, ());
        let client = RecurringPaymentContractClient::new(&env, &contract_id);

        let payment_id = client.create_payment(
            &sender,
            &recipient,
            &token_address,
            &100,
            &1,
            &0,
        );

        // Miss twice
        let _ = std::panic::catch_unwind(|| { client.execute_payment(&payment_id); });
        let _ = std::panic::catch_unwind(|| { client.execute_payment(&payment_id); });

        let payment = client.get_payment(&payment_id);
        assert_eq!(payment.missed_count, 2);
    }
}
