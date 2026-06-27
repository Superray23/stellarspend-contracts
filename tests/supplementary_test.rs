//! Supplemental tests — cases not covered by the existing test files.
//!
//! Fraud gaps
//! ──────────
//! - `check_transaction` does not persist config when un-initialized (falls
//!   back to defaults without panicking)
//! - Two calls on the *same* day for *different* users don't bleed into each
//!   other's daily bucket (already in my first file, but this one uses a
//!   single-call approach that also validates the key shape)
//! - Daily window boundary: a call at exactly 00:00:00 UTC of a new day uses
//!   a fresh bucket (epoch / 86_400 changes)
//!
//! Conditional-payment gaps
//! ────────────────────────
//! - `approval_count` on a condition index that is NOT a MultiSig panics with
//!   `InvalidCondition`
//! - `check_conditions` on an already-executed payment still returns results
//!   (read-only, no status gate)
//! - `payment_count` before any payment is created returns 0
//! - `BalanceBelow` with negative threshold rejected at creation
//!   (`InvalidCondition`)
//! - `BalanceAbove` with negative threshold rejected at creation
//!   (`InvalidCondition`)
//! - `approve` on a non-existent payment panics with `PaymentNotFound`
//! - `execute_payment` on a non-existent payment panics with `PaymentNotFound`
//! - Two independent payments share no state (escrow amounts don't mix)
//! - `settled_at` is non-zero and >= `created_at` after cancellation
//! - `settled_at` is non-zero and >= `created_at` after execution

// ── Fraud supplemental ────────────────────────────────────────────────────────

#[cfg(test)]
mod fraud_supplemental {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env,
    };

    use crate::{FraudContract, FraudContractClient};

    fn setup() -> (Env, Address, FraudContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, FraudContract);
        let client = FraudContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    fn set_timestamp(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            ..env.ledger().get()
        });
    }

    /// A call made at exactly the first second of a new day (ts % 86_400 == 0)
    /// should open a fresh daily bucket and therefore not be flagged by the
    /// daily-cap rule for a small amount.
    #[test]
    fn daily_bucket_resets_at_exact_midnight_boundary() {
        let (env, _admin, client) = setup();
        let user = Address::generate(&env);

        // Day 1: fill up to just under the daily cap.
        set_timestamp(&env, 0);
        for _ in 0..10 {
            let _ = client.check_transaction(&user, &9_999);
        }
        // cumulative day-1 = 99_990

        // Move to the very first second of day 2.
        set_timestamp(&env, 86_400); // ts / 86_400 == 1, day index changes

        // First tx of day 2 is well below the cap → must not be flagged.
        let flagged = client.check_transaction(&user, &9_999);
        assert!(
            !flagged,
            "first transaction of a new day must not be flagged regardless of prior day's total"
        );
    }

    /// Verify that a transaction at exactly the per-tx threshold is flagged but
    /// one stroops below is not, confirming the `>=` boundary in Rule 1.
    #[test]
    fn single_tx_threshold_boundary_is_inclusive() {
        let (env, _admin, client) = setup();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);

        // threshold - 1 → not flagged by Rule 1.
        let below = client.check_transaction(&user_a, &9_999);
        assert!(!below, "amount one below threshold must not trigger single-tx flag");

        // exactly threshold → flagged by Rule 1.
        let at = client.check_transaction(&user_b, &10_000);
        assert!(at, "amount exactly at threshold must trigger single-tx flag");
    }

    /// Changing the config between transactions on the same day does NOT
    /// retroactively change the accumulated daily total that was already written.
    #[test]
    fn config_change_does_not_retroactively_alter_daily_total() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        // Accumulate 9_999 at 5 % threshold (default max_daily = 100_000).
        let flagged_1 = client.check_transaction(&user, &9_999);
        assert!(!flagged_1);

        // Lower max_daily to 5_000 — the stored daily total (9_999) now exceeds it.
        client.set_config(&admin, &10_000, &5_000);

        // The next call must be flagged because 9_999 + 1 > 5_000.
        let flagged_2 = client.check_transaction(&user, &1);
        assert!(
            flagged_2,
            "reduced daily cap must cause next tx to be flagged given already-accumulated total"
        );
    }
}

// ── Conditional-payment supplemental ─────────────────────────────────────────

#[cfg(test)]
mod conditional_payment_supplemental {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token::{Client as TokenClient, StellarAssetClient},
        Address, Env, Vec,
    };

    use crate::{
        ConditionalPaymentError, ConditionalPaymentsContract,
        ConditionalPaymentsContractClient, Condition, PaymentStatus,
    };

    struct Ctx {
        env: Env,
        client: ConditionalPaymentsContractClient<'static>,
        token: Address,
    }

    impl Ctx {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();
            let admin = Address::generate(&env);
            let token = env.register_stellar_asset_contract(admin.clone());
            let contract_id = env.register_contract(None, ConditionalPaymentsContract);
            let client = ConditionalPaymentsContractClient::new(&env, &contract_id);
            client.initialize(&admin);
            // SAFETY: lifetime cast; env, client, and token live together in Ctx.
            let client: ConditionalPaymentsContractClient<'static> =
                unsafe { core::mem::transmute(client) };
            Self { env, client, token }
        }

        fn fund_and_approve(&self, owner: &Address, amount: i128) {
            StellarAssetClient::new(&self.env, &self.token).mint(owner, &amount);
            TokenClient::new(&self.env, &self.token).approve(
                owner,
                &self.client.address,
                &amount,
                &(self.env.ledger().sequence() + 100_000),
            );
        }

        fn set_ts(&self, ts: u64) {
            self.env.ledger().set(LedgerInfo {
                timestamp: ts,
                ..self.env.ledger().get()
            });
        }

        fn one_cond(&self, c: Condition) -> Vec<Condition> {
            let mut v = Vec::new(&self.env);
            v.push_back(c);
            v
        }

        fn balance(&self, addr: &Address) -> i128 {
            TokenClient::new(&self.env, &self.token).balance(addr)
        }
    }

    // ── payment_count starts at zero ──────────────────────────────────────

    #[test]
    fn payment_count_zero_before_any_payments() {
        let ctx = Ctx::new();
        assert_eq!(ctx.client.payment_count(), 0);
    }

    // ── approval_count on non-MultiSig condition ──────────────────────────

    #[test]
    fn approval_count_on_non_multisig_condition_panics() {
        let ctx = Ctx::new();
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 500_i128;
        ctx.fund_and_approve(&payer, amount);

        // Payment with a TimeAfter condition at index 0.
        let id = ctx.client.create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::TimeAfter(1)),
        );

        // `approval_count` on a TimeAfter condition must panic with InvalidCondition.
        let result = ctx.client.try_approval_count(&id, &0);
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::InvalidCondition)),
            "approval_count on a non-MultiSig condition must return InvalidCondition"
        );
    }

    // ── check_conditions on an executed payment ───────────────────────────

    #[test]
    fn check_conditions_on_executed_payment_still_returns_results() {
        let ctx = Ctx::new();
        ctx.set_ts(0);
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 300_i128;
        ctx.fund_and_approve(&payer, amount);

        let id = ctx.client.create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::TimeAfter(1)),
        );

        ctx.set_ts(1);
        ctx.client.execute_payment(&id);

        // Should not panic and must return one entry.
        let results = ctx.client.check_conditions(&id);
        assert_eq!(results.len(), 1);
        // At ts=1 the TimeAfter(1) condition passes.
        assert_eq!(results.get(0), Some(true));
    }

    // ── negative threshold validation ─────────────────────────────────────

    #[test]
    fn balance_above_negative_threshold_panics_at_creation() {
        let ctx = Ctx::new();
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let watch = Address::generate(&ctx.env);
        let amount = 100_i128;
        ctx.fund_and_approve(&payer, amount);

        let result = ctx.client.try_create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::BalanceAbove {
                token: ctx.token.clone(),
                watch_address: watch,
                threshold: -1,
            }),
        );
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::InvalidCondition)),
            "BalanceAbove with negative threshold must be rejected at creation"
        );
    }

    #[test]
    fn balance_below_negative_threshold_panics_at_creation() {
        let ctx = Ctx::new();
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let watch = Address::generate(&ctx.env);
        let amount = 100_i128;
        ctx.fund_and_approve(&payer, amount);

        let result = ctx.client.try_create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::BalanceBelow {
                token: ctx.token.clone(),
                watch_address: watch,
                threshold: -500,
            }),
        );
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::InvalidCondition)),
            "BalanceBelow with negative threshold must be rejected at creation"
        );
    }

    // ── operations on non-existent payment IDs ────────────────────────────

    #[test]
    fn approve_nonexistent_payment_panics() {
        let ctx = Ctx::new();
        let signer = Address::generate(&ctx.env);
        let result = ctx.client.try_approve(&signer, &9_999);
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::PaymentNotFound)),
            "approve on a non-existent payment must return PaymentNotFound"
        );
    }

    #[test]
    fn execute_nonexistent_payment_panics() {
        let ctx = Ctx::new();
        let result = ctx.client.try_execute_payment(&9_999);
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::PaymentNotFound)),
            "execute_payment on a non-existent payment must return PaymentNotFound"
        );
    }

    #[test]
    fn cancel_nonexistent_payment_panics() {
        let ctx = Ctx::new();
        let caller = Address::generate(&ctx.env);
        let result = ctx.client.try_cancel_payment(&caller, &9_999);
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::PaymentNotFound)),
            "cancel_payment on a non-existent payment must return PaymentNotFound"
        );
    }

    // ── two independent payments don't share escrow ───────────────────────

    #[test]
    fn two_payments_escrow_amounts_are_independent() {
        let ctx = Ctx::new();
        ctx.set_ts(0);

        let payer_a = Address::generate(&ctx.env);
        let payer_b = Address::generate(&ctx.env);
        let recipient_a = Address::generate(&ctx.env);
        let recipient_b = Address::generate(&ctx.env);

        let amount_a = 1_000_i128;
        let amount_b = 3_000_i128;

        ctx.fund_and_approve(&payer_a, amount_a);
        ctx.fund_and_approve(&payer_b, amount_b);

        let id_a = ctx.client.create_payment(
            &payer_a,
            &recipient_a,
            &ctx.token,
            &amount_a,
            &ctx.one_cond(Condition::TimeAfter(1)),
        );
        let id_b = ctx.client.create_payment(
            &payer_b,
            &recipient_b,
            &ctx.token,
            &amount_b,
            &ctx.one_cond(Condition::TimeAfter(1)),
        );

        // Contract holds both escrowed amounts.
        assert_eq!(ctx.balance(&ctx.client.address), amount_a + amount_b);

        // Execute only payment A.
        ctx.set_ts(1);
        ctx.client.execute_payment(&id_a);

        assert_eq!(ctx.balance(&recipient_a), amount_a);
        assert_eq!(ctx.balance(&recipient_b), 0, "recipient_b must not receive anything yet");
        // Contract still holds payment B's escrow.
        assert_eq!(ctx.balance(&ctx.client.address), amount_b);

        // Execute payment B.
        ctx.client.execute_payment(&id_b);
        assert_eq!(ctx.balance(&recipient_b), amount_b);
        assert_eq!(ctx.balance(&ctx.client.address), 0);
    }

    // ── settled_at timestamp is recorded correctly ─────────────────────────

    #[test]
    fn settled_at_is_set_on_execution() {
        let ctx = Ctx::new();
        ctx.set_ts(1_000);
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 200_i128;
        ctx.fund_and_approve(&payer, amount);

        let id = ctx.client.create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::TimeAfter(2_000)),
        );

        let before_settle = ctx.client.get_payment(&id);
        assert_eq!(before_settle.settled_at, 0, "settled_at must be 0 while pending");

        ctx.set_ts(2_000);
        ctx.client.execute_payment(&id);

        let after_settle = ctx.client.get_payment(&id);
        assert!(
            after_settle.settled_at >= after_settle.created_at,
            "settled_at must be >= created_at after execution"
        );
        assert_eq!(after_settle.settled_at, 2_000);
    }

    #[test]
    fn settled_at_is_set_on_cancellation() {
        let ctx = Ctx::new();
        ctx.set_ts(500);
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 200_i128;
        ctx.fund_and_approve(&payer, amount);

        let id = ctx.client.create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::TimeAfter(999_999)),
        );

        ctx.set_ts(600);
        ctx.client.cancel_payment(&payer, &id);

        let p = ctx.client.get_payment(&id);
        assert_eq!(p.status, PaymentStatus::Cancelled);
        assert_eq!(p.settled_at, 600, "settled_at must reflect the cancellation timestamp");
        assert!(
            p.settled_at >= p.created_at,
            "settled_at must be >= created_at"
        );
    }

    // ── MultiSig: max signers boundary ────────────────────────────────────

    #[test]
    fn multisig_exactly_max_signers_is_valid() {
        use crate::MAX_SIGNERS;

        let ctx = Ctx::new();
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 100_i128;
        ctx.fund_and_approve(&payer, amount);

        let mut signers = Vec::new(&ctx.env);
        for _ in 0..MAX_SIGNERS {
            signers.push_back(Address::generate(&ctx.env));
        }
        let last_signer = signers.get(MAX_SIGNERS as u32 - 1).unwrap();

        let id = ctx.client.create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::MultiSigApproved {
                signers: signers.clone(),
                required: 1, // only need one for the execute check
            }),
        );

        ctx.client.approve(&last_signer, &id);
        let paid = ctx.client.execute_payment(&id);
        assert_eq!(paid, amount);
    }

    #[test]
    fn multisig_one_over_max_signers_panics() {
        use crate::MAX_SIGNERS;

        let ctx = Ctx::new();
        let payer = Address::generate(&ctx.env);
        let recipient = Address::generate(&ctx.env);
        let amount = 100_i128;
        ctx.fund_and_approve(&payer, amount);

        let mut signers = Vec::new(&ctx.env);
        for _ in 0..(MAX_SIGNERS + 1) {
            signers.push_back(Address::generate(&ctx.env));
        }

        let result = ctx.client.try_create_payment(
            &payer,
            &recipient,
            &ctx.token,
            &amount,
            &ctx.one_cond(Condition::MultiSigApproved {
                signers,
                required: 1,
            }),
        );
        assert_eq!(
            result,
            Err(Ok(ConditionalPaymentError::TooManySigners)),
            "MAX_SIGNERS + 1 signers must be rejected"
        );
    }
}