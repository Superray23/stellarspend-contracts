//! # Allowances Contract
//!
//! Manages recurring spending allowances on Stellar/Soroban.
//!
//! ## Issues resolved
//! - #822 Create Allowance Contract — storage schema + contract scaffold
//! - #823 Add Allowance Creation    — `create_allowance` with event emission
//! - #824 Implement Weekly Allowances  — `Frequency::Weekly` (7-day interval)
//! - #825 Implement Monthly Allowances — `Frequency::Monthly` (30-day interval)
//! - #832 Add Daily Allowances         — `Frequency::Daily` (24-hour interval)
//! - #833 Add Allowance Pause/Resume   — `pause_allowance` / `resume_allowance`
//! - #834 Add Allowance Cancellation   — `cancel_allowance` (already present, confirmed)
//! - #835 Add Allowance Beneficiary Update — `update_beneficiary`
//! - #845 Allowance Approval Workflow   — `set_approval_config` / `approve_allowance` (large allowances stay inactive until approved) + `transfer_ownership`
//! - #846 Add Allowance Analytics       — `get_allowance_analytics` (total distributed, average payment, remaining)
//! - #847 Optimize Allowance Storage    — shared `load`/`save`/`append_index` helpers (one accessor per op)
//! - #836 Implement Allowance Spending Limits — `set_spending_limit` + cumulative cap enforced in `distribute`
//! - #837 Add Allowance History         — per-distribution `PaymentRecord` log + `get_allowance_history`
//! - #838 Emit Allowance Payment Events  — `("allow","payment",id)` → (recipient, amount) on every payment
//! - #839 Add Allowance Expiration      — `set_expiration` / `is_expired`; `distribute` stops past `end_date`

#![no_std]

mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token, Address, Env, Vec,
};

use types::{AllowanceError, Allowance, AllowanceAnalytics, DataKey, Frequency};
use types::{AllowanceError, Allowance, DataKey, Frequency, PaymentRecord};

// ── Internal storage helpers (#847) ───────────────────────────────────────────
//
// Centralize allowance reads/writes so every operation performs exactly one
// load and one store, and the persistent-storage accessor + key are built once
// per access rather than being duplicated across call sites. Previously each
// mutator re-derived the `env.storage().persistent()` accessor for both its
// read and its write (and index updates did so twice); these helpers collapse
// that to a single accessor per logical operation.

/// Loads an allowance by id, panicking with `NotFound` if it does not exist.
fn load_allowance(env: &Env, allowance_id: u64) -> Allowance {
    env.storage()
        .persistent()
        .get(&DataKey::Allowance(allowance_id))
        .unwrap_or_else(|| panic_with_error!(env, AllowanceError::NotFound))
}

/// Persists an allowance record.
fn save_allowance(env: &Env, allowance_id: u64, allowance: &Allowance) {
    env.storage()
        .persistent()
        .set(&DataKey::Allowance(allowance_id), allowance);
}

/// Appends an id to an index vector (`OwnerAllowances` / `RecipientAllowances`)
/// using a single storage accessor for the read-modify-write.
fn append_index(env: &Env, key: DataKey, allowance_id: u64) {
    let store = env.storage().persistent();
    let mut ids: Vec<u64> = store.get(&key).unwrap_or(Vec::new(env));
    ids.push_back(allowance_id);
    store.set(&key, &ids);
}

#[contract]
pub struct AllowancesContract;

#[contractimpl]
impl AllowancesContract {
    // ── Creation ──────────────────────────────────────────────────────────

    pub fn create_allowance(
        env: Env,
        owner: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        frequency: Frequency,
        start_time: u64,
    ) -> u64 {
        owner.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, AllowanceError::InvalidAmount);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AllowanceCount)
            .unwrap_or(0);
        count += 1;

        // Large allowances require approval before they become active (#845).
        // When no threshold is configured, every allowance is active on
        // creation (unchanged behaviour).
        let requires_approval = match env
            .storage()
            .instance()
            .get::<DataKey, i128>(&DataKey::ApprovalThreshold)
        {
            Some(threshold) => amount > threshold,
            None => false,
        };

        let allowance = Allowance {
            owner: owner.clone(),
            recipient: recipient.clone(),
            token,
            amount,
            frequency: frequency.clone(),
            next_distribution: start_time,
            distribution_count: 0,
            active: !requires_approval,
            paused: false,
            pending_approval: requires_approval,
            spending_limit: 0, // unlimited until an owner sets one (#836)
            end_date: 0, // never expires until an owner sets an end date (#839)
        };

        save_allowance(&env, count, &allowance);
        env.storage().instance().set(&DataKey::AllowanceCount, &count);

        append_index(&env, DataKey::OwnerAllowances(owner.clone()), count);
        append_index(&env, DataKey::RecipientAllowances(recipient.clone()), count);

        let freq_tag = match &frequency {
            Frequency::Once    => symbol_short!("once"),
            Frequency::Daily   => symbol_short!("daily"),
            Frequency::Weekly  => symbol_short!("weekly"),
            Frequency::Monthly => symbol_short!("monthly"),
        };
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("created"), count),
            (owner, recipient, amount, freq_tag),
        );

        count
    }

    // ── Distribution ──────────────────────────────────────────────────────

    pub fn distribute(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        if allowance.pending_approval {
            panic_with_error!(&env, AllowanceError::ApprovalRequired);
        }
        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }
        if allowance.paused {
            panic_with_error!(&env, AllowanceError::Paused);
        }

        let now = env.ledger().timestamp();

        // Past the end date the allowance is expired and distributions stop
        // automatically (#839). `0` means no expiry.
        if allowance.end_date != 0 && now >= allowance.end_date {
            panic_with_error!(&env, AllowanceError::Expired);
        }

        if now < allowance.next_distribution {
            panic_with_error!(&env, AllowanceError::TooEarlyToDistribute);
        }

        // Enforce the cumulative spending cap (#836). `0` means unlimited.
        if allowance.spending_limit > 0 {
            let projected = allowance
                .amount
                .checked_mul((allowance.distribution_count + 1) as i128)
                .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::SpendingLimitExceeded));
            if projected > allowance.spending_limit {
                panic_with_error!(&env, AllowanceError::SpendingLimitExceeded);
            }
        }

        let token_client = token::Client::new(&env, &allowance.token);
        if token_client.balance(&allowance.owner) < allowance.amount {
            panic_with_error!(&env, AllowanceError::InsufficientBalance);
        }

        token_client.transfer_from(
            &env.current_contract_address(),
            &allowance.owner,
            &allowance.recipient,
            &allowance.amount,
        );

        allowance.distribution_count += 1;

        // Append to the allowance's payment history (#837): amount, timestamp,
        // and the recipient at the time of this payment.
        let mut history: Vec<PaymentRecord> = env
            .storage().persistent()
            .get(&DataKey::AllowanceHistory(allowance_id))
            .unwrap_or(Vec::new(&env));
        history.push_back(PaymentRecord {
            amount: allowance.amount,
            timestamp: now,
            recipient: allowance.recipient.clone(),
        });
        env.storage().persistent().set(&DataKey::AllowanceHistory(allowance_id), &history);

        match allowance.frequency.interval_seconds() {
            None => {
                allowance.active = false;
                allowance.next_distribution = 0;
            }
            Some(interval) => {
                allowance.next_distribution += interval;
                if allowance.next_distribution <= now {
                    let missed = (now - allowance.next_distribution) / interval;
                    allowance.next_distribution += (missed + 1) * interval;
                }
            }
        }

        save_allowance(&env, allowance_id, &allowance);
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);

        // Dedicated payment event for off-chain indexers (#838): a stable
        // `("allow", "payment", allowance_id)` topic carrying (recipient, amount)
        // is emitted on every payment, alongside the richer `distrib` event.
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("payment"), allowance_id),
            (allowance.recipient.clone(), allowance.amount),
        );
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("distrib"), allowance_id),
            (allowance.recipient, allowance.amount, allowance.next_distribution),
        );
    }

    // ── Pause / Resume (#833) ─────────────────────────────────────────────

    /// Temporarily suspends distributions. Only the owner may pause.
    pub fn pause_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if allowance.paused   { panic_with_error!(&env, AllowanceError::AlreadyPaused); }

        allowance.paused = true;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("paused"), allowance_id),
            allowance.owner,
        );
    }

    /// Resumes a paused allowance. Only the owner may resume.
    pub fn resume_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if !allowance.paused  { panic_with_error!(&env, AllowanceError::NotPaused); }

        allowance.paused = false;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("resumed"), allowance_id),
            allowance.owner,
        );
    }

    // ── Cancellation (#834) ───────────────────────────────────────────────

    /// Permanently cancels an allowance. Only the owner may cancel.
    pub fn cancel_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        allowance.active = false;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("canceled"), allowance_id),
            allowance.owner,
        );
    }

    // ── Beneficiary update (#835) ─────────────────────────────────────────

    /// Updates the recipient of an active allowance. Only the owner may call.
    /// Future distributions go to `new_recipient`; history is preserved.
    pub fn update_beneficiary(env: Env, allowance_id: u64, new_recipient: Address) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        let old_recipient = allowance.recipient.clone();
        allowance.recipient = new_recipient.clone();
        save_allowance(&env, allowance_id, &allowance);

        // Update recipient index for new beneficiary
        append_index(&env, DataKey::RecipientAllowances(new_recipient.clone()), allowance_id);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("ben_upd"), allowance_id),
            (old_recipient, new_recipient),
        );
    }

    // ── Approval workflow (#845) ──────────────────────────────────────────

    /// Configures the approval policy: the `approver` address and the `threshold`
    /// above which new allowances require approval before becoming active.
    ///
    /// First call is authorized by the incoming `approver`; subsequent calls
    /// (rotation / threshold changes) must be authorized by the current approver.
    pub fn set_approval_config(env: Env, approver: Address, threshold: i128) {
        if threshold <= 0 {
            panic_with_error!(&env, AllowanceError::InvalidThreshold);
        }

        match env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Approver)
        {
            Some(current) => current.require_auth(),
            None => approver.require_auth(),
        }

        env.storage().instance().set(&DataKey::Approver, &approver);
        env.storage().instance().set(&DataKey::ApprovalThreshold, &threshold);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("apprcfg")),
            (approver, threshold),
        );
    }

    /// Approves a pending (over-threshold) allowance, activating it.
    /// Only the configured approver may call.
    pub fn approve_allowance(env: Env, allowance_id: u64) {
        let approver: Address = env
            .storage().instance()
            .get(&DataKey::Approver)
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::ApproverNotConfigured));
        approver.require_auth();

    // ── Spending limit (#836) ─────────────────────────────────────────────

    /// Sets the maximum cumulative amount that may ever be distributed for an
    /// allowance. Only the owner may call. A limit of `0` removes the cap
    /// (unlimited). A positive limit caps total spend at that value; once the
    /// cumulative `amount × distribution_count` would exceed it, `distribute`
    /// returns `SpendingLimitExceeded`.
    ///
    /// # Errors
    /// * `AllowanceError::NotFound`        - allowance does not exist
    /// * `AllowanceError::AlreadyInactive` - allowance is no longer active
    /// * `AllowanceError::InvalidLimit`    - `limit` is negative
    pub fn set_spending_limit(env: Env, allowance_id: u64, limit: i128) {
    // ── Expiration (#839) ─────────────────────────────────────────────────

    /// Sets (or clears) the allowance's end date. Only the owner may call.
    /// Once the ledger time reaches `end_date`, `distribute` stops automatically
    /// (returns `Expired`). Pass `0` to remove the expiry.
    ///
    /// # Errors
    /// * `AllowanceError::NotFound`          - allowance does not exist
    /// * `AllowanceError::AlreadyInactive`   - allowance is no longer active
    /// * `AllowanceError::InvalidExpiration` - `end_date` is non-zero and not in the future
    pub fn set_expiration(env: Env, allowance_id: u64, end_date: u64) {
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        if !allowance.pending_approval {
            panic_with_error!(&env, AllowanceError::NotPendingApproval);
        }

        allowance.pending_approval = false;
        allowance.active = true;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("approved"), allowance_id),
            approver,
        );
    }

    // ── Ownership transfer (#845) ─────────────────────────────────────────

    /// Reassigns ownership of an allowance to `new_owner`. Only the current
    /// owner may call. After transfer, only the new owner can manage the
    /// allowance (pause, resume, cancel, update beneficiary, transfer again).
    pub fn transfer_ownership(env: Env, allowance_id: u64, new_owner: Address) {
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();
        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }

        let old_owner = allowance.owner.clone();

        // Remove the id from the previous owner's index.
        let prev_ids: Vec<u64> = env
            .storage().persistent()
            .get(&DataKey::OwnerAllowances(old_owner.clone()))
            .unwrap_or(Vec::new(&env));
        let mut remaining = Vec::new(&env);
        for id in prev_ids.iter() {
            if id != allowance_id {
                remaining.push_back(id);
            }
        }
        env.storage().persistent().set(&DataKey::OwnerAllowances(old_owner.clone()), &remaining);

        // Add the id to the new owner's index.
        let mut new_ids: Vec<u64> = env
            .storage().persistent()
            .get(&DataKey::OwnerAllowances(new_owner.clone()))
            .unwrap_or(Vec::new(&env));
        new_ids.push_back(allowance_id);
        env.storage().persistent().set(&DataKey::OwnerAllowances(new_owner.clone()), &new_ids);

        allowance.owner = new_owner.clone();
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("own_xfer"), allowance_id),
            (old_owner, new_owner),
        );
    }

        allowance.owner.require_auth();
        if limit < 0 {
            panic_with_error!(&env, AllowanceError::InvalidLimit);
        }
        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }

        allowance.spending_limit = limit;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("limit"), allowance_id),
            limit,
        );
    }

        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }
        if end_date != 0 && end_date <= env.ledger().timestamp() {
            panic_with_error!(&env, AllowanceError::InvalidExpiration);
        }

        allowance.end_date = end_date;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("expiry"), allowance_id),
            end_date,
        );
    }

    /// Returns `true` if the allowance has an end date that the current ledger
    /// time has reached or passed (#839).
    pub fn is_expired(env: Env, allowance_id: u64) -> bool {
        let allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));
        allowance.end_date != 0 && env.ledger().timestamp() >= allowance.end_date
    }

    // ── Queries ───────────────────────────────────────────────────────────

    pub fn get_allowance(env: Env, allowance_id: u64) -> Allowance {
        load_allowance(&env, allowance_id)
    }

    /// Returns usage analytics for an allowance (#846): total amount
    /// distributed, the average payment, and the owner's remaining spendable
    /// balance in the allowance token.
    pub fn get_allowance_analytics(env: Env, allowance_id: u64) -> AllowanceAnalytics {
        let allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        let count = allowance.distribution_count as i128;
        let total_distributed = allowance.amount.saturating_mul(count);
        let average_payment = if count == 0 {
            0
        } else {
            total_distributed / count
        };
        let remaining = token::Client::new(&env, &allowance.token).balance(&allowance.owner);

        AllowanceAnalytics {
            total_distributed,
            distribution_count: allowance.distribution_count,
            average_payment,
            remaining,
        }
    }

    pub fn get_owner_allowances(env: Env, owner: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::OwnerAllowances(owner))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the full payment history for an allowance (#837), oldest first.
    /// Empty if no distributions have occurred (or the allowance does not exist).
    pub fn get_allowance_history(env: Env, allowance_id: u64) -> Vec<PaymentRecord> {
        env.storage().persistent()
            .get(&DataKey::AllowanceHistory(allowance_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_recipient_allowances(env: Env, recipient: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::RecipientAllowances(recipient))
            .unwrap_or(Vec::new(&env))
    }

    pub fn allowance_count(env: Env) -> u64 {
        env.storage().instance()
            .get(&DataKey::AllowanceCount)
            .unwrap_or(0)
    }
}
