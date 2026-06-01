mod support;

use support::{amounts, setup};

#[test]
fn preview_sums_valid_batch_without_state_changes() {
    let ctx = setup();
    ctx.client.set_min_fee(&ctx.admin, &10i128);

    let batch = amounts(&ctx.env, &[10, 20, 30]);
    let total = ctx.client.preview_batch_fee(&ctx.payer, &batch);
    assert_eq!(total, 60);

    // Ensure no state mutated
    assert_eq!(ctx.client.get_escrow_balance(), 0);
    assert_eq!(ctx.client.get_pending_fees(&1), 0);
    assert_eq!(ctx.client.get_total_collected(), 0);
    assert_eq!(ctx.client.get_total_batch_calls(), 0);
}

#[test]
#[should_panic]
fn preview_rejects_empty_batch() {
    let ctx = setup();
    let empty = amounts(&ctx.env, &[]);
    ctx.client.preview_batch_fee(&ctx.payer, &empty);
}

#[test]
#[should_panic]
fn preview_rejects_item_below_min_fee() {
    let ctx = setup();
    ctx.client.set_min_fee(&ctx.admin, &25i128);
    let batch = amounts(&ctx.env, &[25, 10, 40]);
    ctx.client.preview_batch_fee(&ctx.payer, &batch);
}

#[test]
fn preview_accepts_edge_at_min_and_large_values() {
    let ctx = setup();
    ctx.client.set_min_fee(&ctx.admin, &1i128);
    // Includes very large values within i128 range; also includes value exactly at min
    let batch = amounts(&ctx.env, &[1, 2, 1_000_000_000_000i128]);
    let total = ctx.client.preview_batch_fee(&ctx.payer, &batch);
    assert_eq!(total, 1 + 2 + 1_000_000_000_000i128);
}

#[test]
#[should_panic]
fn set_max_fee_rejects_negative_value() {
    let ctx = setup();
    ctx.client.set_max_fee(&ctx.admin, &-1i128);
}

#[test]
#[should_panic]
fn set_max_fee_rejects_value_below_min_fee() {
    let ctx = setup();
    ctx.client.set_min_fee(&ctx.admin, &100i128);
    ctx.client.set_max_fee(&ctx.admin, &50i128);
}
