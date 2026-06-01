// tests/fee_reconciliation_and_min_fee.rs
//
// Comprehensive tests for:
//   - Minimum fee enforcement (single + batch)
//   - Fee reconciliation across cycles
//   - Admin access control
//   - Boundary conditions and invariants

mod support;

use soroban_sdk::{testutils::Address as _, Address};

// ─── Minimum Fee Tests ────────────────────────────────────────────────────────

mod min_fee {
    use super::*;
    use support::setup;

    // ── Default state ──────────────────────────────────────────────────────

    #[test]
    fn default_is_zero() {
        let ctx = setup();
        assert_eq!(ctx.client.get_min_fee(), 0);
    }

    #[test]
    fn default_zero_allows_smallest_possible_amount() {
        let ctx = setup();
        // i128 minimum positive value
        let pending = ctx.client.collect_fee(&ctx.payer, &1i128);
        assert_eq!(pending, 1);
        assert_eq!(ctx.client.get_escrow_balance(), 1);
        assert_eq!(ctx.client.get_pending_fees(&1), 1);
    }

    // ── Setting min fee ────────────────────────────────────────────────────

    #[test]
    fn admin_can_set_min_fee() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);
        assert_eq!(ctx.client.get_min_fee(), 100);
    }

    #[test]
    fn admin_can_set_min_fee_to_zero() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);
        ctx.client.set_min_fee(&ctx.admin, &0i128);
        assert_eq!(ctx.client.get_min_fee(), 0);
    }

    #[test]
    fn admin_can_update_min_fee_multiple_times() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &50i128);
        assert_eq!(ctx.client.get_min_fee(), 50);

        ctx.client.set_min_fee(&ctx.admin, &200i128);
        assert_eq!(ctx.client.get_min_fee(), 200);

        ctx.client.set_min_fee(&ctx.admin, &1i128);
        assert_eq!(ctx.client.get_min_fee(), 1);
    }

    #[test]
    #[should_panic]
    fn non_admin_cannot_set_min_fee() {
        let ctx = setup();
        let rogue = Address::generate(&ctx.env);
        ctx.client.set_min_fee(&rogue, &100i128);
    }

    #[test]
    #[should_panic]
    fn negative_min_fee_is_rejected() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &-1i128);
    }

    #[test]
    #[should_panic]
    fn large_negative_min_fee_is_rejected() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &i128::MIN);
    }

    // ── Single collect — acceptance ────────────────────────────────────────

    #[test]
    fn collect_exactly_at_min_fee_succeeds() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);

        let pending = ctx.client.collect_fee(&ctx.payer, &100i128);

        assert_eq!(pending, 100);
        assert_eq!(ctx.client.get_escrow_balance(), 100);
        assert_eq!(ctx.client.get_pending_fees(&1), 100);
    }

    #[test]
    fn collect_above_min_fee_succeeds() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);

        let pending = ctx.client.collect_fee(&ctx.payer, &101i128);

        assert_eq!(pending, 101);
        assert_eq!(ctx.client.get_escrow_balance(), 101);
    }

    #[test]
    fn collect_accumulates_correctly_above_min() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);

        let p1 = ctx.client.collect_fee(&ctx.payer, &100i128);
        assert_eq!(p1, 100);

        let p2 = ctx.client.collect_fee(&ctx.payer, &150i128);
        assert_eq!(p2, 250);

        let p3 = ctx.client.collect_fee(&ctx.payer, &500i128);
        assert_eq!(p3, 750);

        assert_eq!(ctx.client.get_escrow_balance(), 750);
        assert_eq!(ctx.client.get_pending_fees(&1), 750);
    }

    #[test]
    fn boundary_min_fee_of_one_accepts_one() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &1i128);

        let pending = ctx.client.collect_fee(&ctx.payer, &1i128);

        assert_eq!(pending, 1);
        assert_eq!(ctx.client.get_escrow_balance(), 1);
    }

    // ── Single collect — rejection ─────────────────────────────────────────

    #[test]
    #[should_panic]
    fn collect_one_below_min_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);
        // 99 = min - 1
        ctx.client.collect_fee(&ctx.payer, &99i128);
    }

    #[test]
    #[should_panic]
    fn collect_zero_when_min_is_one_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &1i128);
        ctx.client.collect_fee(&ctx.payer, &0i128);
    }

    #[test]
    #[should_panic]
    fn collect_negative_amount_panics() {
        let ctx = setup();
        ctx.client.collect_fee(&ctx.payer, &-1i128);
    }

    #[test]
    #[should_panic]
    fn collect_zero_when_min_is_nonzero_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &50i128);
        ctx.client.collect_fee(&ctx.payer, &0i128);
    }

    // ── Batch collect — acceptance ─────────────────────────────────────────

    #[test]
    fn batch_all_at_min_succeeds() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &25i128);

        let batch = support::amounts(&ctx.env, &[25, 25, 25]);
        let res = ctx.client.collect_fee_batch(&ctx.payer, &batch);

        assert_eq!(res.batch_size, 3);
        assert_eq!(res.total_amount, 75);
        assert_eq!(res.cycle, 1);
        assert_eq!(res.pending_fees, 75);
        assert_eq!(ctx.client.get_escrow_balance(), 75);
        assert_eq!(ctx.client.get_pending_fees(&1), 75);
    }

    #[test]
    fn batch_all_above_min_succeeds() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &25i128);

        let batch = support::amounts(&ctx.env, &[25, 30, 40, 100]);
        let res = ctx.client.collect_fee_batch(&ctx.payer, &batch);

        assert_eq!(res.batch_size, 4);
        assert_eq!(res.total_amount, 195);
        assert_eq!(res.cycle, 1);
        assert_eq!(res.pending_fees, 195);
        assert_eq!(ctx.client.get_escrow_balance(), 195);
    }

    #[test]
    fn batch_with_zero_min_fee_accepts_any_positive() {
        let ctx = setup();
        // min_fee = 0 (default)

        let batch = support::amounts(&ctx.env, &[1, 2, 3, 100_000]);
        let res = ctx.client.collect_fee_batch(&ctx.payer, &batch);

        assert_eq!(res.batch_size, 4);
        assert_eq!(res.total_amount, 100_006);
        assert_eq!(ctx.client.get_escrow_balance(), 100_006);
    }

    // ── Batch collect — rejection ──────────────────────────────────────────

    #[test]
    #[should_panic]
    fn batch_first_item_below_min_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &25i128);

        let batch = support::amounts(&ctx.env, &[10, 30, 40]);
        ctx.client.collect_fee_batch(&ctx.payer, &batch);
    }

    #[test]
    #[should_panic]
    fn batch_middle_item_below_min_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &25i128);

        let batch = support::amounts(&ctx.env, &[25, 10, 40]);
        ctx.client.collect_fee_batch(&ctx.payer, &batch);
    }

    #[test]
    #[should_panic]
    fn batch_last_item_below_min_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &25i128);

        let batch = support::amounts(&ctx.env, &[25, 30, 5]);
        ctx.client.collect_fee_batch(&ctx.payer, &batch);
    }

    #[test]
    #[should_panic]
    fn batch_single_item_below_min_panics() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &100i128);

        let batch = support::amounts(&ctx.env, &[99]);
        ctx.client.collect_fee_batch(&ctx.payer, &batch);
    }

    // ── Min fee does not affect already-collected amounts ──────────────────

    #[test]
    fn raising_min_fee_does_not_affect_prior_escrow() {
        let ctx = setup();

        // Collect before raising min
        ctx.client.collect_fee(&ctx.payer, &10i128);
        assert_eq!(ctx.client.get_escrow_balance(), 10);

        // Raise min fee
        ctx.client.set_min_fee(&ctx.admin, &100i128);

        // Prior escrow is untouched
        assert_eq!(ctx.client.get_escrow_balance(), 10);
        assert_eq!(ctx.client.get_pending_fees(&1), 10);
    }
}

// ─── Reconciliation Tests ─────────────────────────────────────────────────────

mod reconciliation {
    use super::*;
    use support::setup;

    // ── Balanced states ────────────────────────────────────────────────────

    #[test]
    fn balanced_on_fresh_contract() {
        let ctx = setup();
        let result = ctx.client.get_reconciliation_status();

        assert_eq!(result.stored_balance, 0);
        assert_eq!(result.calculated_balance, 0);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_after_single_collect() {
        let ctx = setup();
        ctx.client.collect_fee(&ctx.payer, &100);

        let result = ctx.client.reconcile_fees(&ctx.admin);

        assert_eq!(result.stored_balance, 100);
        assert_eq!(result.calculated_balance, 100);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_after_batch_collect() {
        let ctx = setup();
        let amounts = support::amounts(&ctx.env, &[100, 200, 300]);
        ctx.client.collect_fee_batch(&ctx.payer, &amounts);

        let result = ctx.client.get_reconciliation_status();

        assert_eq!(result.stored_balance, 600);
        assert_eq!(result.calculated_balance, 600);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_after_full_cycle_release() {
        let ctx = setup();
        ctx.client.collect_fee(&ctx.payer, &500);
        ctx.client.release_fees(&ctx.admin, &1);

        let result = ctx.client.reconcile_fees(&ctx.admin);

        assert_eq!(result.stored_balance, 0);
        assert_eq!(result.calculated_balance, 0);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_after_rollover_without_release() {
        let ctx = setup();
        ctx.client.collect_fee(&ctx.payer, &400);
        ctx.client.rollover_fees(&ctx.admin, &2);

        // Rollover moves pending between cycles but total escrow is unchanged
        let result = ctx.client.get_reconciliation_status();

        assert_eq!(result.stored_balance, 400);
        assert_eq!(result.calculated_balance, 400);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_after_partial_release_with_rollover() {
        let ctx = setup();

        // Cycle 1: collect 300
        ctx.client.collect_fee(&ctx.payer, &300);
        // Roll into cycle 2
        ctx.client.rollover_fees(&ctx.admin, &2);
        // Collect more in cycle 2
        ctx.client.collect_fee(&ctx.payer, &200);
        // Release cycle 2 (contains 300 rolled + 200 new = 500)
        ctx.client.release_fees(&ctx.admin, &2);

        let result = ctx.client.get_reconciliation_status();

        assert_eq!(result.stored_balance, 0);
        assert_eq!(result.calculated_balance, 0);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    #[test]
    fn balanced_across_multiple_cycles() {
        let ctx = setup();

        // Cycle 1 → roll into 2
        ctx.client.collect_fee(&ctx.payer, &100);
        ctx.client.rollover_fees(&ctx.admin, &2);

        // Cycle 2 → collect more, then release everything
        ctx.client.collect_fee(&ctx.payer, &200);
        ctx.client.release_fees(&ctx.admin, &2);

        let result = ctx.client.reconcile_fees(&ctx.admin);

        assert_eq!(result.stored_balance, 0);
        assert_eq!(result.calculated_balance, 0);
        assert_eq!(result.discrepancy, 0);
        assert!(result.is_reconciled);
    }

    // ── Post-release state ─────────────────────────────────────────────────

    #[test]
    fn new_collection_after_release_reconciles_correctly() {
        let ctx = setup();

        ctx.client.collect_fee(&ctx.payer, &1000);
        ctx.client.collect_fee(&ctx.payer, &500);
        ctx.client.release_fees(&ctx.admin, &1);

        // After release: escrow should be 0
        let r1 = ctx.client.get_reconciliation_status();
        assert_eq!(r1.stored_balance, 0);
        assert_eq!(r1.calculated_balance, 0);
        assert!(r1.is_reconciled);

        // Collect again in a new cycle
        ctx.client.collect_fee(&ctx.payer, &250);
        let r2 = ctx.client.get_reconciliation_status();

        assert_eq!(r2.stored_balance, 250);
        assert_eq!(r2.calculated_balance, 250);
        assert_eq!(r2.discrepancy, 0);
        assert!(r2.is_reconciled);
    }

    // ── Three-cycle invariant ──────────────────────────────────────────────

    #[test]
    fn invariant_holds_across_three_cycles_with_mixed_operations() {
        let ctx = setup();

        // Cycle 1: 150 collected, rolled to 2
        ctx.client.collect_fee(&ctx.payer, &150);
        ctx.client.rollover_fees(&ctx.admin, &2);

        let r1 = ctx.client.get_reconciliation_status();
        assert!(r1.is_reconciled, "must be reconciled after rollover 1→2");

        // Cycle 2: 150 + 250 = 400, rolled to 3
        ctx.client.collect_fee(&ctx.payer, &250);
        ctx.client.rollover_fees(&ctx.admin, &3);

        let r2 = ctx.client.get_reconciliation_status();
        assert_eq!(r2.stored_balance, 400);
        assert!(r2.is_reconciled, "must be reconciled after rollover 2→3");

        // Cycle 3: 400 + 100 = 500, then release
        ctx.client.collect_fee(&ctx.payer, &100);
        ctx.client.release_fees(&ctx.admin, &3);

        let r3 = ctx.client.reconcile_fees(&ctx.admin);
        assert_eq!(r3.stored_balance, 0);
        assert_eq!(r3.calculated_balance, 0);
        assert_eq!(r3.discrepancy, 0);
        assert!(r3.is_reconciled);
    }

    // ── get_reconciliation_status vs reconcile_fees consistency ───────────

    #[test]
    fn status_and_reconcile_return_same_result() {
        let ctx = setup();
        ctx.client.collect_fee(&ctx.payer, &700);

        let status    = ctx.client.get_reconciliation_status();
        let reconcile = ctx.client.reconcile_fees(&ctx.admin);

        // Both views must agree on every field
        assert_eq!(status.stored_balance,     reconcile.stored_balance);
        assert_eq!(status.calculated_balance, reconcile.calculated_balance);
        assert_eq!(status.discrepancy,        reconcile.discrepancy);
        assert_eq!(status.is_reconciled,      reconcile.is_reconciled);
    }

    // ── Admin access control ───────────────────────────────────────────────

    #[test]
    fn get_reconciliation_status_is_public() {
        // No admin required — anyone can view the reconciliation status
        let ctx = setup();
        let result = ctx.client.get_reconciliation_status();
        assert!(result.is_reconciled);
    }

    #[test]
    fn reconcile_fees_requires_admin() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();

        let issuer        = Address::generate(&env);
        let stellar_asset = env.register_stellar_asset_contract_v2(issuer);
        let token_id      = stellar_asset.address();
        let contract_id   = env.register(fee::FeeContract, ());
        let client        = fee::FeeContractClient::new(&env, &contract_id);
        let admin         = Address::generate(&env);
        let treasury      = Address::generate(&env);
        let fake_admin    = Address::generate(&env);

        client.initialize(&admin, &token_id, &treasury, &250u32, &1u64);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.reconcile_fees(&fake_admin);
        }));

        assert!(
            result.is_err(),
            "reconcile_fees must reject a non-admin caller",
        );
    }

    #[test]
    #[should_panic]
    fn release_fees_requires_admin() {
        let ctx        = setup();
        let non_admin  = Address::generate(&ctx.env);
        ctx.client.collect_fee(&ctx.payer, &100);
        ctx.client.release_fees(&non_admin, &1);
    }

    #[test]
    #[should_panic]
    fn rollover_fees_requires_admin() {
        let ctx       = setup();
        let non_admin = Address::generate(&ctx.env);
        ctx.client.collect_fee(&ctx.payer, &100);
        ctx.client.rollover_fees(&non_admin, &2);
    }
}

// ─── Combined / Cross-cutting Tests ──────────────────────────────────────────

mod combined {
    use super::*;
    use support::setup;

    #[test]
    fn min_fee_respected_and_reconciliation_balanced_in_same_session() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &50i128);

        // Collect several fees above min
        ctx.client.collect_fee(&ctx.payer, &50);
        ctx.client.collect_fee(&ctx.payer, &75);
        ctx.client.collect_fee(&ctx.payer, &200);

        // Confirm reconciliation is still clean
        let r = ctx.client.get_reconciliation_status();
        assert_eq!(r.stored_balance, 325);
        assert_eq!(r.calculated_balance, 325);
        assert_eq!(r.discrepancy, 0);
        assert!(r.is_reconciled);

        // Release and confirm clean again
        ctx.client.release_fees(&ctx.admin, &1);
        let r2 = ctx.client.reconcile_fees(&ctx.admin);
        assert_eq!(r2.stored_balance, 0);
        assert!(r2.is_reconciled);
    }

    #[test]
    fn batch_with_min_fee_and_reconciliation_end_to_end() {
        let ctx = setup();
        ctx.client.set_min_fee(&ctx.admin, &10i128);

        let batch = support::amounts(&ctx.env, &[10, 20, 30, 100]);
        let batch_res = ctx.client.collect_fee_batch(&ctx.payer, &batch);

        assert_eq!(batch_res.total_amount, 160);
        assert_eq!(batch_res.pending_fees, 160);

        let r = ctx.client.get_reconciliation_status();
        assert_eq!(r.stored_balance, 160);
        assert_eq!(r.calculated_balance, 160);
        assert!(r.is_reconciled);

        ctx.client.rollover_fees(&ctx.admin, &2);
        ctx.client.collect_fee(&ctx.payer, &50);
        ctx.client.release_fees(&ctx.admin, &2);

        let r2 = ctx.client.reconcile_fees(&ctx.admin);
        assert_eq!(r2.stored_balance, 0);
        assert!(r2.is_reconciled);
    }

  