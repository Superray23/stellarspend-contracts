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
//! - #838 Emit Allowance Payment Events  — `("allow","payment",id)` → (recipient, amount) on every payment
//! - #839 Add Allowance Expiration      — `set_expiration` / `is_expired`; `distribute` stops past `end_date`

#![no_std]

mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token, Address, Env, Vec,
};

use types::{AllowanceError, Allowance, DataKey, Frequency};

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

        let allowance = Allowance {
            owner: owner.clone(),
            recipient: recipient.clone(),
            token,
            amount,
            frequency: frequency.clone(),
            next_distribution: start_time,
            distribution_count: 0,
            active: true,
            paused: false,
            end_date: 0, // never expires until an owner sets an end date (#839)
        };

        env.storage().persistent().set(&DataKey::Allowance(count), &allowance);
        env.storage().instance().set(&DataKey::AllowanceCount, &count);

        let mut owner_ids: Vec<u64> = env
            .storage().persistent()
            .get(&DataKey::OwnerAllowances(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_ids.push_back(count);
        env.storage().persistent().set(&DataKey::OwnerAllowances(owner.clone()), &owner_ids);

        let mut recip_ids: Vec<u64> = env
            .storage().persistent()
            .get(&DataKey::RecipientAllowances(recipient.clone()))
            .unwrap_or(Vec::new(&env));
        recip_ids.push_back(count);
        env.storage().persistent().set(&DataKey::RecipientAllowances(recipient.clone()), &recip_ids);

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
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

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
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if allowance.paused   { panic_with_error!(&env, AllowanceError::AlreadyPaused); }

        allowance.paused = true;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("paused"), allowance_id),
            allowance.owner,
        );
    }

    /// Resumes a paused allowance. Only the owner may resume.
    pub fn resume_allowance(env: Env, allowance_id: u64) {
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if !allowance.paused  { panic_with_error!(&env, AllowanceError::NotPaused); }

        allowance.paused = false;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("resumed"), allowance_id),
            allowance.owner,
        );
    }

    // ── Cancellation (#834) ───────────────────────────────────────────────

    /// Permanently cancels an allowance. Only the owner may cancel.
    pub fn cancel_allowance(env: Env, allowance_id: u64) {
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        allowance.active = false;
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("canceled"), allowance_id),
            allowance.owner,
        );
    }

    // ── Beneficiary update (#835) ─────────────────────────────────────────

    /// Updates the recipient of an active allowance. Only the owner may call.
    /// Future distributions go to `new_recipient`; history is preserved.
    pub fn update_beneficiary(env: Env, allowance_id: u64, new_recipient: Address) {
        let mut allowance: Allowance = env
            .storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        let old_recipient = allowance.recipient.clone();
        allowance.recipient = new_recipient.clone();
        env.storage().persistent().set(&DataKey::Allowance(allowance_id), &allowance);

        // Update recipient index for new beneficiary
        let mut recip_ids: Vec<u64> = env
            .storage().persistent()
            .get(&DataKey::RecipientAllowances(new_recipient.clone()))
            .unwrap_or(Vec::new(&env));
        recip_ids.push_back(allowance_id);
        env.storage().persistent().set(&DataKey::RecipientAllowances(new_recipient.clone()), &recip_ids);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("ben_upd"), allowance_id),
            (old_recipient, new_recipient),
        );
    }

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

        allowance.owner.require_auth();
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
        env.storage().persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound))
    }

    pub fn get_owner_allowances(env: Env, owner: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::OwnerAllowances(owner))
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
