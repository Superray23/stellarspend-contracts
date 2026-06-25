//! # Fee Escrow — Internal Bookkeeping Module
//!
//! ╔══════════════════════════════════════════════════════════════════════╗
//! ║  THIS IS NOT A REAL ESCROW CONTRACT                                ║
//! ║  See `contracts/escrow/src/lib.rs` for the standalone escrow       ║
//! ║  contract that manages individual depositor↔recipient escrows.     ║
//! ╚══════════════════════════════════════════════════════════════════════╝
//!
//! This module is **internal** to the Fee contract (`crate::escrow`). It
//! provides pooled fee-collection bookkeeping, **not** per-party escrows.
//!
//! ## What it does
//!
//! - `collect_to_escrow` / `collect_batch_to_escrow` — transfer tokens from
//!   a payer into the Fee contract and increment internal counters
//!   (`escrow_balance`, `pending_fees` per cycle, `total_collected`).
//! - `release_cycle_fees` — sweep a cycle's pending fees to the treasury
//!   and decrement `escrow_balance`.
//! - `rollover_cycle_fees` — carry forward pending fees to a later cycle.
//!
//! ## Relationship to `contracts/escrow`
//!
//! | Aspect              | Fee escrow (this module)      | Standalone escrow crate       |
//! |---------------------|-------------------------------|-------------------------------|
//! | Scope               | Internal Fee contract module  | Deployable Soroban contract   |
//! | Granularity         | Pooled (one balance counter)  | Per-escrow records            |
//! | Parties             | Payer → Fee treasury          | Depositor → Recipient         |
//! | Reversal            | No individual reversal        | Batch release/reversal        |
//! | Lifecycle tracking  | Per-cycle pending fees only   | Per-escrow status (Active/…)  |
//!
//! The two are **independent** — the Fee contract never imports or calls
//! the standalone escrow crate, and vice versa.

use shared::utils::validate_amount as validate_non_negative_amount;
use soroban_sdk::{panic_with_error, token, Address, Env, Vec};

use crate::storage::{
    add_batch_call, add_escrow_balance, add_pending_fees, add_total_collected, add_total_released,
    clear_pending_fees, read_current_cycle, read_min_fee, read_pending_fees, read_token,
    read_treasury, sub_escrow_balance, BatchFeeResult,
};
use crate::FeeContractError;

fn require_positive_amount(env: &Env, amount: i128) {
    if validate_non_negative_amount(amount).is_err() || amount == 0 {
        panic_with_error!(env, FeeContractError::InvalidAmount);
    }
}

pub fn collect_to_escrow(env: &Env, payer: &Address, amount: i128) -> i128 {
    require_positive_amount(env, amount);
    let min_fee = read_min_fee(env);
    if amount < min_fee {
        panic_with_error!(env, FeeContractError::InvalidAmount);
    }

    let token_id = read_token(env);
    let token_client = token::Client::new(env, &token_id);
    let contract_address = env.current_contract_address();
    token_client.transfer(payer, &contract_address, &amount);

    let cycle = read_current_cycle(env);
    let pending = add_pending_fees(env, cycle, amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    add_escrow_balance(env, amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    add_total_collected(env, amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));

    pending
}

pub fn collect_batch_to_escrow(env: &Env, payer: &Address, amounts: &Vec<i128>) -> BatchFeeResult {
    let mut total_amount: i128 = 0;
    let min_fee = read_min_fee(env);

    for amount in amounts.iter() {
        require_positive_amount(env, amount);
        if amount < min_fee {
            panic_with_error!(env, FeeContractError::InvalidAmount);
        }
        total_amount = total_amount
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    }

    let token_id = read_token(env);
    let token_client = token::Client::new(env, &token_id);
    let contract_address = env.current_contract_address();
    token_client.transfer(payer, &contract_address, &total_amount);

    let cycle = read_current_cycle(env);
    let pending_fees = add_pending_fees(env, cycle, total_amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    add_escrow_balance(env, total_amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    add_total_collected(env, total_amount)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    add_batch_call(env).unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));

    BatchFeeResult {
        batch_size: amounts.len(),
        total_amount,
        cycle,
        pending_fees,
    }
}

pub fn release_cycle_fees(env: &Env, cycle: u64) -> i128 {
    let pending = read_pending_fees(env, cycle);
    if pending <= 0 {
        panic_with_error!(env, FeeContractError::NoPendingFees);
    }

    let token_id = read_token(env);
    let treasury = read_treasury(env);
    let token_client = token::Client::new(env, &token_id);
    let contract_address = env.current_contract_address();
    token_client.transfer(&contract_address, &treasury, &pending);

    clear_pending_fees(env, cycle);
    sub_escrow_balance(env, pending)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::InsufficientEscrow));
    add_total_released(env, pending)
        .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));

    pending
}

pub fn rollover_cycle_fees(env: &Env, from_cycle: u64, to_cycle: u64) -> i128 {
    let pending = read_pending_fees(env, from_cycle);
    if pending > 0 {
        add_pending_fees(env, to_cycle, pending)
            .unwrap_or_else(|| panic_with_error!(env, FeeContractError::Overflow));
    }
    clear_pending_fees(env, from_cycle);
    pending
}
