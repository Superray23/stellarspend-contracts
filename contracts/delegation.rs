#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DelegationError {
    InvalidAddress = 1,
    InvalidAmount = 2,
    Unauthorized = 3,
    AmountTooLarge = 4,
    // [SEC-DEL-01] NEW: explicit overflow variant instead of silent i128::MAX clamp.
    Overflow = 5,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Delegation {
    pub limit: i128,
    pub spent: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DelegationDataKey {
    Allowance(Address, Address), // (owner, delegate)
}

#[contract]
pub struct DelegationContract;

#[contractimpl]
impl DelegationContract {
    /// Authorizes `delegate` to spend up to `limit` on behalf of `owner`.
    ///
    /// # Security
    /// - [SEC-DEL-02] Self-delegation is explicitly blocked: an address granting
    ///   itself a delegation would create a trivial privilege-escalation path.
    /// - [SEC-DEL-03] `limit` must be strictly positive; zero or negative limits
    ///   are rejected before any storage write occurs.
    /// - Pre-existing delegations have their `limit` updated but `spent` is
    ///   preserved, preventing a re-grant from resetting accumulated usage.
    pub fn set_delegation(env: Env, owner: Address, delegate: Address, limit: i128) {
        owner.require_auth();

        // [SEC-DEL-02] Self-delegation guard.
        if owner == delegate {
            panic_with_error!(&env, DelegationError::InvalidAddress);
        }
        // [SEC-DEL-03] Positive-limit guard.
        if limit <= 0 {
            panic_with_error!(&env, DelegationError::InvalidAmount);
        }

        let key = DelegationDataKey::Allowance(owner.clone(), delegate.clone());
        // Preserve spent so a limit reset cannot be abused to replay already-
        // consumed allowance.
        let mut delegation: Delegation = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Delegation { limit: 0, spent: 0 });

        delegation.limit = limit;
        env.storage().persistent().set(&key, &delegation);

        env.events().publish(
            (
                soroban_sdk::symbol_short!("delegate"),
                soroban_sdk::symbol_short!("set"),
                owner.clone(),
                delegate.clone(),
            ),
            limit,
        );
    }

    /// Revokes all delegation rights from `delegate`.
    ///
    /// # Security
    /// - [SEC-DEL-04] Removes the key entirely rather than zeroing fields so
    ///   `consume_allowance` cannot race a zero-limit entry.
    pub fn revoke_delegation(env: Env, owner: Address, delegate: Address) {
        owner.require_auth();

        let key = DelegationDataKey::Allowance(owner.clone(), delegate.clone());
        if env.storage().persistent().has(&key) {
            env.storage().persistent().remove(&key);

            env.events().publish(
                (
                    soroban_sdk::symbol_short!("delegate"),
                    soroban_sdk::symbol_short!("revoked"),
                    owner.clone(),
                    delegate.clone(),
                ),
                (),
            );
        }
    }

    /// Records that `delegate` has consumed `amount` of their allowance.
    ///
    /// # Security
    /// - [SEC-DEL-05] `delegate.require_auth()` is the first operation so that
    ///   unauthorized callers cannot probe delegation state.
    /// - [SEC-DEL-01] `checked_add` replaces the previous `unwrap_or(i128::MAX)`
    ///   clamp; an overflow now surfaces as a typed error rather than silently
    ///   capping `new_spent` and potentially bypassing the limit comparison.
    /// - Missing delegation entry returns `Unauthorized` (not `NotFound`) to
    ///   avoid leaking information about whether a delegation exists.
    pub fn consume_allowance(
        env: Env,
        owner: Address,
        delegate: Address,
        amount: i128,
    ) -> Result<(), DelegationError> {
        // [SEC-DEL-05] Authenticate first — no state reads before this point.
        delegate.require_auth();

        if amount <= 0 {
            return Err(DelegationError::InvalidAmount);
        }

        let key = DelegationDataKey::Allowance(owner.clone(), delegate.clone());

        if let Some(mut delegation) = env.storage().persistent().get::<_, Delegation>(&key) {
            // [SEC-DEL-01] Checked addition — surfaced as Overflow, not clamped.
            let new_spent = delegation
                .spent
                .checked_add(amount)
                .ok_or(DelegationError::Overflow)?;

            if new_spent > delegation.limit {
                return Err(DelegationError::AmountTooLarge);
            }

            delegation.spent = new_spent;
            env.storage().persistent().set(&key, &delegation);

            env.events().publish(
                (
                    soroban_sdk::symbol_short!("delegate"),
                    soroban_sdk::symbol_short!("consumed"),
                    owner.clone(),
                    delegate.clone(),
                ),
                amount,
            );

            Ok(())
        } else {
            // [SEC-DEL-05] Return Unauthorized rather than a distinct "not found"
            // error to avoid leaking delegation existence to unauthenticated callers.
            Err(DelegationError::Unauthorized)
        }
    }

    /// Returns the current delegation state, or `None` if no delegation exists.
    pub fn get_delegation(env: Env, owner: Address, delegate: Address) -> Option<Delegation> {
        let key = DelegationDataKey::Allowance(owner, delegate);
        env.storage().persistent().get(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        Address,
        Env,
    };

    #[test]
    fn grant_delegation_creates_allowance() {
        let env = Env::default();

        let owner = Address::generate(&env);
        let delegate = Address::generate(&env);

        env.mock_all_auths();

        DelegationContract::set_delegation(
            env.clone(),
            owner.clone(),
            delegate.clone(),
            1_000,
        );

        let delegation =
            DelegationContract::get_delegation(
                env,
                owner,
                delegate,
            )
            .unwrap();

        assert_eq!(delegation.limit, 1_000);
        assert_eq!(delegation.spent, 0);
    }

    #[test]
    fn revoke_delegation_removes_allowance() {
        let env = Env::default();

        let owner = Address::generate(&env);
        let delegate = Address::generate(&env);

        env.mock_all_auths();

        DelegationContract::set_delegation(
            env.clone(),
            owner.clone(),
            delegate.clone(),
            1_000,
        );

        DelegationContract::revoke_delegation(
            env.clone(),
            owner.clone(),
            delegate.clone(),
        );

        let delegation =
            DelegationContract::get_delegation(
                env,
                owner,
                delegate,
            );

        assert!(delegation.is_none());
    }

    #[test]
    fn over_limit_spend_is_rejected() {
        let env = Env::default();

        let owner = Address::generate(&env);
        let delegate = Address::generate(&env);

        env.mock_all_auths();

        DelegationContract::set_delegation(
            env.clone(),
            owner.clone(),
            delegate.clone(),
            1_000,
        );

        let result =
            DelegationContract::consume_allowance(
                env,
                owner,
                delegate,
                1_500,
            );

        assert_eq!(
            result,
            Err(DelegationError::AmountTooLarge)
        );
    }
}