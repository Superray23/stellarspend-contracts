#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, Env, IntoVal, Symbol, TryFromVal,
};

use crate::{AllowancesContract, AllowancesContractClient};
use crate::types::{AllowanceError, Frequency};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Spin up the contract + a SAC token, mint `initial_balance` to `owner`,
/// and approve the contract to spend on behalf of `owner`.
fn setup(
    initial_balance: i128,
) -> (
    Env,
    AllowancesContractClient<'static>,
    Address, // owner
    Address, // recipient
    Address, // token
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AllowancesContract, ());
    let client = AllowancesContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Fund owner
    StellarAssetClient::new(&env, &token_id).mint(&owner, &initial_balance);

    // Approve contract to transfer on behalf of owner (large ledger approval)
    TokenClient::new(&env, &token_id).approve(
        &owner,
        &contract_id,
        &initial_balance,
        &(env.ledger().sequence() + 10_000),
    );

    (env, client, owner, recipient, token_id)
}

// ── Storage schema / contract deploy (#822) ───────────────────────────────────

#[test]
fn contract_deploys_and_count_starts_at_zero() {
    let (env, client, _, _, _) = setup(1_000);
    assert_eq!(client.allowance_count(), 0, "fresh contract must start at 0");
    let _ = env;
}

// ── create_allowance (#823) ────────────────────────────────────────────────────

#[test]
fn create_allowance_increments_count() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);
    assert_eq!(id, 1);
    assert_eq!(client.allowance_count(), 1);
}

#[test]
fn create_allowance_stores_correct_fields() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &250, &Frequency::Weekly, &now);
    let a = client.get_allowance(&id);

    assert_eq!(a.owner, owner);
    assert_eq!(a.recipient, recipient);
    assert_eq!(a.token, token);
    assert_eq!(a.amount, 250);
    assert!(matches!(a.frequency, Frequency::Weekly));
    assert_eq!(a.next_distribution, now);
    assert_eq!(a.distribution_count, 0);
    assert!(a.active);
}

#[test]
fn create_allowance_emits_created_event() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);

    let events = env.events().all();
    // At least one event should have been emitted
    assert!(!events.is_empty(), "expected at least one event");
    let _ = id;
}

#[test]
fn create_allowance_rejects_zero_amount() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let err = client
        .try_create_allowance(&owner, &recipient, &token, &0, &Frequency::Once, &now)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::InvalidAmount.into());
}

#[test]
fn create_allowance_rejects_negative_amount() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let err = client
        .try_create_allowance(&owner, &recipient, &token, &-1, &Frequency::Once, &now)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::InvalidAmount.into());
}

#[test]
fn owner_and_recipient_indices_are_populated() {
    let (env, client, owner, recipient, token) = setup(2_000);
    let now = env.ledger().timestamp();

    let id1 = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);
    let id2 = client.create_allowance(&owner, &recipient, &token, &200, &Frequency::Weekly, &now);

    let owner_ids = client.get_owner_allowances(&owner);
    assert!(owner_ids.contains(&id1));
    assert!(owner_ids.contains(&id2));

    let recip_ids = client.get_recipient_allowances(&recipient);
    assert!(recip_ids.contains(&id1));
    assert!(recip_ids.contains(&id2));
}

// ── Weekly distribution (#824) ────────────────────────────────────────────────

#[test]
fn weekly_distribution_transfers_tokens() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_allowance(&owner, &recipient, &token, &300, &Frequency::Weekly, &now);

    // Move ledger past the first weekly window
    env.ledger().with_mut(|l| l.timestamp = now + 604_800 + 1);

    client.distribute(&id);

    assert_eq!(token_client.balance(&recipient), 300);
    assert_eq!(token_client.balance(&owner), 700);

    let a = client.get_allowance(&id);
    assert_eq!(a.distribution_count, 1);
    assert!(a.active, "weekly allowance must stay active after one distribution");
    // Next window should be ~2 weeks from start
    assert!(a.next_distribution > now + 604_800);
}

#[test]
fn weekly_interval_is_604800_seconds() {
    assert_eq!(Frequency::Weekly.interval_seconds(), Some(604_800));
}

#[test]
fn weekly_distribute_too_early_is_rejected() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &(now + 10_000));

    let err = client
        .try_distribute(&id)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::TooEarlyToDistribute.into());
}

// ── Monthly distribution (#825) ───────────────────────────────────────────────

#[test]
fn monthly_distribution_transfers_tokens() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_allowance(&owner, &recipient, &token, &500, &Frequency::Monthly, &now);

    // Move ledger past the first monthly window
    env.ledger().with_mut(|l| l.timestamp = now + 2_592_000 + 1);

    client.distribute(&id);

    assert_eq!(token_client.balance(&recipient), 500);
    assert_eq!(token_client.balance(&owner), 500);

    let a = client.get_allowance(&id);
    assert_eq!(a.distribution_count, 1);
    assert!(a.active, "monthly allowance must stay active after one distribution");
    assert!(a.next_distribution > now + 2_592_000);
}

#[test]
fn monthly_interval_is_2592000_seconds() {
    assert_eq!(Frequency::Monthly.interval_seconds(), Some(2_592_000));
}

#[test]
fn monthly_distribute_too_early_is_rejected() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    // Start 30 days in the future
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Monthly, &(now + 2_592_000));

    let err = client
        .try_distribute(&id)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::TooEarlyToDistribute.into());
}

// ── Once frequency ────────────────────────────────────────────────────────────

#[test]
fn once_allowance_deactivates_after_distribution() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);
    client.distribute(&id);

    let a = client.get_allowance(&id);
    assert!(!a.active, "Once allowance must be inactive after distribution");
}

#[test]
fn once_interval_is_none() {
    assert_eq!(Frequency::Once.interval_seconds(), None);
}

// ── Cancellation ──────────────────────────────────────────────────────────────

#[test]
fn cancel_allowance_deactivates_it() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);
    client.cancel_allowance(&id);

    let a = client.get_allowance(&id);
    assert!(!a.active);
}

#[test]
fn cancel_already_inactive_returns_error() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);
    client.distribute(&id); // deactivates it

    let err = client
        .try_cancel_allowance(&id)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::AlreadyInactive.into());
}

#[test]
fn distribute_inactive_allowance_returns_error() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();

    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);
    client.cancel_allowance(&id);

    let err = client
        .try_distribute(&id)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::AlreadyInactive.into());
}

#[test]
fn distribute_nonexistent_returns_error() {
    let (_env, client, ..) = setup(1_000);

    let err = client
        .try_distribute(&999)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::NotFound.into());
}

// ── Payment events (#838) ───────────────────────────────────────────────────

const PWEEK: u64 = 604_800;

/// Counts emitted events whose topics are `("allow", "payment", _)`.
fn payment_event_count(env: &Env) -> u32 {
    let mut count = 0u32;
    for (_addr, topics, _data) in env.events().all().iter() {
        if topics.len() == 3 {
            let t1 = Symbol::try_from_val(env, &topics.get(1).unwrap());
            if t1.map(|s| s == symbol_short!("payment")).unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn distribution_emits_payment_event_with_recipient_and_amount() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);

    client.distribute(&id);

    // Exact topic/data: ("allow","payment", id) → (recipient, amount).
    let expected = (
        client.address.clone(),
        (symbol_short!("allow"), symbol_short!("payment"), id).into_val(&env),
        (recipient.clone(), 100i128).into_val(&env),
    );
    assert!(
        env.events().all().contains(expected),
        "a payment event with (recipient, amount) must be emitted"
    );
}

#[test]
fn payment_event_emitted_on_every_payment() {
    let (env, client, owner, recipient, token) = setup(10_000);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &50, &Frequency::Weekly, &now);

    // `events().all()` reflects the most recent invocation. Each `distribute`
    // invocation must emit exactly one payment event; creation emits none.
    assert_eq!(payment_event_count(&env), 0, "creation emits no payment event");

    client.distribute(&id);
    assert_eq!(payment_event_count(&env), 1, "first payment emits one event");

    env.ledger().with_mut(|l| l.timestamp = now + PWEEK + 1);
    client.distribute(&id);
    assert_eq!(payment_event_count(&env), 1, "second payment emits one event");

    env.ledger().with_mut(|l| l.timestamp = now + 2 * PWEEK + 1);
    client.distribute(&id);
    assert_eq!(payment_event_count(&env), 1, "third payment emits one event");
}

#[test]
fn payment_event_uses_current_recipient_after_beneficiary_change() {
    let (env, client, owner, recipient, token) = setup(10_000);
    let now = env.ledger().timestamp();
    let new_recipient = Address::generate(&env);
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);

    client.update_beneficiary(&id, &new_recipient);
    client.distribute(&id);

    let expected = (
        client.address.clone(),
        (symbol_short!("allow"), symbol_short!("payment"), id).into_val(&env),
        (new_recipient.clone(), 100i128).into_val(&env),
    );
    assert!(
        env.events().all().contains(expected),
        "payment event must reflect the updated recipient"
    );
// ── Allowance expiration (#839) ─────────────────────────────────────────────

const EWEEK: u64 = 604_800;

#[test]
fn allowance_has_no_expiry_by_default() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &now);
    assert_eq!(client.get_allowance(&id).end_date, 0);
    assert!(!client.is_expired(&id));
}

#[test]
fn set_expiration_stores_end_date() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);

    client.set_expiration(&id, &(now + 10 * EWEEK));
    assert_eq!(client.get_allowance(&id).end_date, now + 10 * EWEEK);
    assert!(!client.is_expired(&id));
}

#[test]
fn set_expiration_rejects_past_timestamp() {
    let (env, client, owner, recipient, token) = setup(1_000);
    let now = env.ledger().timestamp();
    env.ledger().with_mut(|l| l.timestamp = now + 1_000);
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Once, &(now + 1_000));

    let err = client
        .try_set_expiration(&id, &(now + 500)) // in the past relative to current ledger time
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::InvalidExpiration.into());
}

#[test]
fn distribution_before_expiry_succeeds() {
    let (env, client, owner, recipient, token) = setup(10_000);
    let token_client = TokenClient::new(&env, &token);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);

    client.set_expiration(&id, &(now + 3 * EWEEK));

    client.distribute(&id); // now < end_date → ok
    assert_eq!(token_client.balance(&recipient), 100);
}

#[test]
fn expired_allowance_stops_distributing() {
    let (env, client, owner, recipient, token) = setup(10_000);
    let token_client = TokenClient::new(&env, &token);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);

    // Expire after two weeks.
    client.set_expiration(&id, &(now + 2 * EWEEK));

    client.distribute(&id); // week 0 — ok
    env.ledger().with_mut(|l| l.timestamp = now + EWEEK + 1);
    client.distribute(&id); // week 1 — ok (still before end)
    assert_eq!(token_client.balance(&recipient), 200);

    // Move past the end date → distributions stop automatically.
    env.ledger().with_mut(|l| l.timestamp = now + 2 * EWEEK + 1);
    assert!(client.is_expired(&id));
    let err = client
        .try_distribute(&id)
        .err()
        .expect("expired distribute must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::Expired.into());

    // No further funds moved.
    assert_eq!(token_client.balance(&recipient), 200);
}

#[test]
fn clearing_expiration_resumes_distribution() {
    let (env, client, owner, recipient, token) = setup(10_000);
    let token_client = TokenClient::new(&env, &token);
    let now = env.ledger().timestamp();
    let id = client.create_allowance(&owner, &recipient, &token, &100, &Frequency::Weekly, &now);

    client.set_expiration(&id, &(now + EWEEK)); // expires in a week
    env.ledger().with_mut(|l| l.timestamp = now + EWEEK + 1);
    assert!(client.is_expired(&id));

    // Owner clears the expiry (0) → no longer expired, distribution works.
    client.set_expiration(&id, &0);
    assert!(!client.is_expired(&id));
    client.distribute(&id);
    assert_eq!(token_client.balance(&recipient), 100);
}

#[test]
fn set_expiration_fails_for_missing_allowance() {
    let (_env, client, _o, _r, _t) = setup(1_000);
    let err = client
        .try_set_expiration(&999, &1)
        .err()
        .expect("must fail")
        .expect("contract error");
    assert_eq!(err, AllowanceError::NotFound.into());
}
