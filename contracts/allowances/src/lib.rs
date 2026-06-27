//! # Allowances Contract
//!
//! Manages recurring spending allowances on Stellar/Soroban.
//!
//! ## Issues resolved
//! - #822 Create Allowance Contract — storage schema + contract scaffold
//! - #823 Add Allowance Creation    — `create_allowance` with event emission
//! - #824 Implement Weekly Allowances  — `Frequency::Weekly` (7-day interval)
//! - #825 Implement Monthly Allowances — `Frequency::Monthly` (30-day interval)
//!
//! ## Features
//!
//! - **`create_allowance`** — owner defines recipient, token, amount, and
//!   frequency (`Once` / `Weekly` / `Monthly`).
//! - **`distribute`** — anyone triggers a due distribution; tokens are
//!   transferred from owner to recipient via `token::Client::transfer_from`.
//! - **`cancel_allowance`** — owner deactivates the allowance.
//! - **Query helpers** — `get_allowance`, `get_owner_allowances`,
//!   `get_recipient_allowances`.

#![no_std]

mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token, Address, Env, Vec,
};

use types::{AllowanceError, Allowance, DataKey, Frequency};

/// Allowance contract.
#[contract]
pub struct AllowancesContract;

#[contractimpl]
impl AllowancesContract {
    // ── Creation ──────────────────────────────────────────────────────────

    /// Creates a new allowance.
    ///
    /// # Arguments
    /// * `owner`      — Address funding the allowance (must authorize).
    /// * `recipient`  — Address entitled to receive distributions.
    /// * `token`      — Token contract for the distribution.
    /// * `amount`     — Tokens transferred per distribution (must be > 0).
    /// * `frequency`  — `Once`, `Weekly`, or `Monthly`.
    /// * `start_time` — Ledger timestamp of the first allowed distribution.
    ///
    /// # Returns
    /// The unique allowance ID.
    ///
    /// # Events
    /// Emits `("allow", "created", id)` → `(owner, recipient, amount, frequency_tag)`.
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

        // Validate frequency (Once is always valid; recurring must have a
        // positive interval, which is guaranteed by the enum itself).
        let _ = frequency.interval_seconds(); // no-op check, satisfies future extensibility

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
        };

        env.storage()
            .persistent()
            .set(&DataKey::Allowance(count), &allowance);
        env.storage()
            .instance()
            .set(&DataKey::AllowanceCount, &count);

        // Update owner index
        let mut owner_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerAllowances(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_ids.push_back(count);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerAllowances(owner.clone()), &owner_ids);

        // Update recipient index
        let mut recip_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::RecipientAllowances(recipient.clone()))
            .unwrap_or(Vec::new(&env));
        recip_ids.push_back(count);
        env.storage()
            .persistent()
            .set(&DataKey::RecipientAllowances(recipient.clone()), &recip_ids);

        // Emit creation event
        let freq_tag = match &frequency {
            Frequency::Once => symbol_short!("once"),
            Frequency::Weekly => symbol_short!("weekly"),
            Frequency::Monthly => symbol_short!("monthly"),
        };
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("created"), count),
            (owner, recipient, amount, freq_tag),
        );

        count
    }

    // ── Distribution ──────────────────────────────────────────────────────

    /// Executes a due distribution for the given allowance.
    ///
    /// Callable by anyone once `env.ledger().timestamp() >= next_distribution`.
    /// For recurring allowances (`Weekly` / `Monthly`) the next distribution
    /// window is advanced automatically after each successful transfer.
    ///
    /// Requires that the owner has pre-approved the contract via the token's
    /// `approve` method so `transfer_from` can be used trustlessly.
    ///
    /// # Events
    /// Emits `("allow", "distrib", id)` → `(recipient, amount, next_distribution)`.
    pub fn distribute(env: Env, allowance_id: u64) {
        let mut allowance: Allowance = env
            .storage()
            .persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }

        let now = env.ledger().timestamp();
        if now < allowance.next_distribution {
            panic_with_error!(&env, AllowanceError::TooEarlyToDistribute);
        }

        // Transfer tokens from owner → recipient.
        let token_client = token::Client::new(&env, &allowance.token);
        let owner_balance = token_client.balance(&allowance.owner);
        if owner_balance < allowance.amount {
            panic_with_error!(&env, AllowanceError::InsufficientBalance);
        }

        // Owner must have pre-approved this contract as spender.
        token_client.transfer_from(
            &env.current_contract_address(),
            &allowance.owner,
            &allowance.recipient,
            &allowance.amount,
        );

        allowance.distribution_count += 1;

        // Advance schedule or deactivate for one-time allowances.
        match allowance.frequency.interval_seconds() {
            None => {
                // Once — mark inactive after first distribution.
                allowance.active = false;
                allowance.next_distribution = 0;
            }
            Some(interval) => {
                // Recurring — skip missed cycles and move forward.
                allowance.next_distribution += interval;
                if allowance.next_distribution <= now {
                    let missed = (now - allowance.next_distribution) / interval;
                    allowance.next_distribution += (missed + 1) * interval;
                }
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Allowance(allowance_id), &allowance);

        env.events().publish(
            (
                symbol_short!("allow"),
                symbol_short!("distrib"),
                allowance_id,
            ),
            (
                allowance.recipient,
                allowance.amount,
                allowance.next_distribution,
            ),
        );
    }

    // ── Cancellation ──────────────────────────────────────────────────────

    /// Cancels an active allowance.  Only the owner may cancel.
    ///
    /// # Events
    /// Emits `("allow", "canceled", id)` → `owner`.
    pub fn cancel_allowance(env: Env, allowance_id: u64) {
        let mut allowance: Allowance = env
            .storage()
            .persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound));

        allowance.owner.require_auth();

        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }

        allowance.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Allowance(allowance_id), &allowance);

        env.events().publish(
            (
                symbol_short!("allow"),
                symbol_short!("canceled"),
                allowance_id,
            ),
            allowance.owner,
        );
    }

    // ── Queries ───────────────────────────────────────────────────────────

    /// Returns the full record for an allowance.
    pub fn get_allowance(env: Env, allowance_id: u64) -> Allowance {
        env.storage()
            .persistent()
            .get(&DataKey::Allowance(allowance_id))
            .unwrap_or_else(|| panic_with_error!(&env, AllowanceError::NotFound))
    }

    /// Returns all allowance IDs created by `owner`.
    pub fn get_owner_allowances(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerAllowances(owner))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns all allowance IDs where `recipient` is the beneficiary.
    pub fn get_recipient_allowances(env: Env, recipient: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::RecipientAllowances(recipient))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the total number of allowances ever created.
    pub fn allowance_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AllowanceCount)
            .unwrap_or(0)
    }
}
