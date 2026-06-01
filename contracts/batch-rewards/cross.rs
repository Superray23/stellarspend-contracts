use soroban_sdk::{
    contracterror, contracttype, panic_with_error, symbol_short, Address, Env, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Signers,
    Threshold,
    HighValueThreshold,
    NextTxId,
    PendingTx(u64),
    Approval(u64, Address),
    ApprovalCount(u64),
    Balance(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct PendingTx {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub payload: Symbol,
    pub asset: Option<Address>,
    pub created_at: u64,
    pub executed: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MultiSigError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidThreshold = 4,
    DuplicateSigner = 5,
    InvalidAmount = 6,
    PendingTxNotFound = 7,
    UnauthorizedSigner = 8,
    DuplicateApproval = 9,
    AlreadyExecuted = 10,
    InsufficientBalance = 11,
    MultisigNotConfigured = 12,
    Overflow = 13,
}

pub struct MultisigEvents;

impl MultisigEvents {
    pub fn pending_created(env: &Env, tx: &PendingTx) {
        let topics = (symbol_short!("tx"), symbol_short!("pending"), tx.id);
        env.events().publish(
            topics,
            (tx.from.clone(), tx.to.clone(), tx.amount, tx.asset.clone()),
        );
    }

    pub fn approval_recorded(
        env: &Env,
        tx_id: u64,
        signer: &Address,
        approvals_count: u32,
        threshold: u32,
    ) {
        let topics = (symbol_short!("approve"), symbol_short!("record"), tx_id);
        env.events()
            .publish(topics, (signer.clone(), approvals_count, threshold));
    }

    pub fn transaction_executed(env: &Env, tx: &PendingTx, executor: &Address) {
        let topics = (symbol_short!("tx"), symbol_short!("executed"), tx.id);
        env.events().publish(
            topics,
            (
                executor.clone(),
                tx.from.clone(),
                tx.to.clone(),
                tx.amount,
                tx.asset.clone(),
            ),
        );
    }
}

pub fn initialize_state(env: &Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(env, MultiSigError::AlreadyInitialized);
    }

    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage()
        .instance()
        .set(&DataKey::Signers, &Vec::<Address>::new(env));
    env.storage().instance().set(&DataKey::Threshold, &0u32);
    env.storage()
        .instance()
        .set(&DataKey::HighValueThreshold, &i128::MAX);
    env.storage().instance().set(&DataKey::NextTxId, &0u64);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::NotInitialized))
}

pub fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin = get_admin(env);
    if admin != caller.clone() {
        panic_with_error!(env, MultiSigError::Unauthorized);
    }
}

pub fn set_signers(env: &Env, caller: Address, signers: Vec<Address>, threshold: u32) {
    require_admin(env, &caller);
    validate_signer_config(env, &signers, threshold);

    env.storage().instance().set(&DataKey::Signers, &signers);
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

pub fn set_high_value_threshold(env: &Env, caller: Address, amount: i128) {
    require_admin(env, &caller);

    if amount < 0 {
        panic_with_error!(env, MultiSigError::InvalidAmount);
    }

    env.storage()
        .instance()
        .set(&DataKey::HighValueThreshold, &amount);
}

pub fn get_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Signers)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::Threshold)
        .unwrap_or(0)
}

pub fn get_high_value_threshold(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::HighValueThreshold)
        .unwrap_or(i128::MAX)
}

pub fn ensure_multisig_configured(env: &Env) {
    let signers = get_signers(env);
    let threshold = get_threshold(env);

    if signers.len() == 0 || threshold == 0 || threshold > signers.len() {
        panic_with_error!(env, MultiSigError::MultisigNotConfigured);
    }
}

pub fn require_signer(env: &Env, signer: &Address) {
    if !is_signer(env, signer) {
        panic_with_error!(env, MultiSigError::UnauthorizedSigner);
    }
}

pub fn is_signer(env: &Env, signer: &Address) -> bool {
    let signers = get_signers(env);
    for configured in signers.iter() {
        if configured == signer.clone() {
            return true;
        }
    }
    false
}

pub fn next_tx_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextTxId)
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::Overflow));

    env.storage().instance().set(&DataKey::NextTxId, &next);
    next
}

pub fn has_approval(env: &Env, tx_id: u64, signer: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Approval(tx_id, signer.clone()))
}

pub fn get_approval_count(env: &Env, tx_id: u64) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ApprovalCount(tx_id))
        .unwrap_or(0)
}

pub fn record_approval(env: &Env, tx_id: u64, signer: &Address) -> u32 {
    if has_approval(env, tx_id, signer) {
        panic_with_error!(env, MultiSigError::DuplicateApproval);
    }

    env.storage()
        .persistent()
        .set(&DataKey::Approval(tx_id, signer.clone()), &true);

    let current = get_approval_count(env, tx_id);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::Overflow));

    env.storage()
        .persistent()
        .set(&DataKey::ApprovalCount(tx_id), &next);

    next
}

fn validate_signer_config(env: &Env, signers: &Vec<Address>, threshold: u32) {
    let signer_count = signers.len();

    if signer_count == 0 || threshold == 0 || threshold > signer_count {
        panic_with_error!(env, MultiSigError::InvalidThreshold);
    }

    for i in 0..signer_count {
        let signer_i = signers
            .get(i)
            .unwrap_or_else(|| panic_with_error!(env, MultiSigError::InvalidThreshold));

        for j in (i + 1)..signer_count {
            let signer_j = signers
                .get(j)
                .unwrap_or_else(|| panic_with_error!(env, MultiSigError::InvalidThreshold));
            if signer_i == signer_j {
                panic_with_error!(env, MultiSigError::DuplicateSigner);
            }
        }
    }
}


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
