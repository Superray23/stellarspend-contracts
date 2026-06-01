#![no_std]
// `Events::publish` is deprecated in soroban-sdk 23.x (pinned by this crate) in
// favour of `#[contractevent]`. We keep the established `publish` style used
// across this repo for these informational upgrade events.
#![allow(deprecated)]

//! Version 2 of the upgradeable contract.
//!
//! Mirrors the multisig + timelock upgrade authorization of the old contract so
//! that the protection persists across upgrades, and adds `handle_upgrade` to
//! migrate state created by the v1 contract.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    BytesN, Env, Vec,
};

/// Default timelock delay applied during migration if none is set: 48 hours.
const DEFAULT_TIMELOCK_DELAY: u64 = 48 * 60 * 60;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    NewAdmin,
    Version,
    Signers,
    Threshold,
    TimelockDelay,
    Pending,
}

/// A pending, not-yet-executed upgrade proposal.
#[contracttype]
#[derive(Clone)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub new_version: u32,
    pub proposer: Address,
    pub scheduled_at: u64,
    pub execute_at: u64,
    pub approvals: Vec<Address>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeError {
    NotAuthorized = 1,
    InvalidVersion = 2,
    InvalidThreshold = 3,
    DuplicateSigner = 4,
    NoPendingUpgrade = 5,
    PendingUpgradeExists = 6,
    AlreadyApproved = 7,
    ThresholdNotMet = 8,
    TimelockNotElapsed = 9,
    AdminMissing = 10,
    EmptySigners = 11,
}

#[contract]
pub struct UpgradeableContract;

#[contractimpl]
impl UpgradeableContract {
    pub fn __constructor(e: Env, admin: Address) {
        e.storage().instance().set(&DataKey::Admin, &admin);
        e.storage().instance().set(&DataKey::NewAdmin, &admin);
        e.storage().instance().set(&DataKey::Version, &2u32);

        let mut signers = Vec::new(&e);
        signers.push_back(admin);
        e.storage().instance().set(&DataKey::Signers, &signers);
        e.storage().instance().set(&DataKey::Threshold, &1u32);
        e.storage()
            .instance()
            .set(&DataKey::TimelockDelay, &DEFAULT_TIMELOCK_DELAY);
    }

    /// Migrate state from the v1 contract. Initializes the NewAdmin key and the
    /// multisig configuration if they were not carried over from v1.
    pub fn handle_upgrade(e: Env) {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, UpgradeError::AdminMissing));
        admin.require_auth();

        if !e.storage().instance().has(&DataKey::NewAdmin) {
            e.storage().instance().set(&DataKey::NewAdmin, &admin);
        }

        // Migrate multisig config if a v1 contract upgraded without it.
        if !e.storage().instance().has(&DataKey::Signers) {
            let mut signers = Vec::new(&e);
            signers.push_back(admin.clone());
            e.storage().instance().set(&DataKey::Signers, &signers);
            e.storage().instance().set(&DataKey::Threshold, &1u32);
        }
        if !e.storage().instance().has(&DataKey::TimelockDelay) {
            e.storage()
                .instance()
                .set(&DataKey::TimelockDelay, &DEFAULT_TIMELOCK_DELAY);
        }

        e.storage().instance().set(&DataKey::Version, &2u32);
    }

    pub fn version(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::Version).unwrap_or(2)
    }

    pub fn new_v2_fn() -> u32 {
        1010101
    }

    // ---------------------------------------------------------------------
    // Configuration (admin only)
    // ---------------------------------------------------------------------

    pub fn set_upgrade_signers(e: Env, signers: Vec<Address>, threshold: u32) {
        Self::require_admin(&e);
        Self::validate_signer_config(&e, &signers, threshold);

        e.storage().instance().set(&DataKey::Signers, &signers);
        e.storage().instance().set(&DataKey::Threshold, &threshold);

        e.events()
            .publish((symbol_short!("upg_cfg"),), (signers.len(), threshold));
    }

    pub fn set_timelock_delay(e: Env, delay: u64) {
        Self::require_admin(&e);
        e.storage().instance().set(&DataKey::TimelockDelay, &delay);

        e.events().publish((symbol_short!("upg_tl"),), delay);
    }

    // ---------------------------------------------------------------------
    // Views
    // ---------------------------------------------------------------------

    pub fn get_signers(e: Env) -> Vec<Address> {
        e.storage()
            .instance()
            .get(&DataKey::Signers)
            .unwrap_or_else(|| Vec::new(&e))
    }

    pub fn get_threshold(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::Threshold).unwrap_or(0)
    }

    pub fn get_timelock_delay(e: Env) -> u64 {
        e.storage()
            .instance()
            .get(&DataKey::TimelockDelay)
            .unwrap_or(0)
    }

    pub fn get_pending_upgrade(e: Env) -> Option<PendingUpgrade> {
        e.storage().instance().get(&DataKey::Pending)
    }

    pub fn upgrade_approval_count(e: Env) -> u32 {
        match Self::get_pending_upgrade(e) {
            Some(p) => p.approvals.len(),
            None => 0,
        }
    }

    pub fn is_upgrade_ready(e: Env) -> bool {
        let threshold = Self::get_threshold(e.clone());
        match Self::get_pending_upgrade(e.clone()) {
            Some(p) => p.approvals.len() >= threshold && e.ledger().timestamp() >= p.execute_at,
            None => false,
        }
    }

    // ---------------------------------------------------------------------
    // Upgrade lifecycle (multisig + timelock)
    // ---------------------------------------------------------------------

    pub fn schedule_upgrade(e: Env, signer: Address, new_wasm_hash: BytesN<32>, new_version: u32) {
        signer.require_auth();
        Self::require_signer(&e, &signer);

        if e.storage().instance().has(&DataKey::Pending) {
            panic_with_error!(&e, UpgradeError::PendingUpgradeExists);
        }

        let current_version = Self::version(e.clone());
        if new_version <= current_version {
            panic_with_error!(&e, UpgradeError::InvalidVersion);
        }

        if !e.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&e, UpgradeError::AdminMissing);
        }
        if !e.storage().instance().has(&DataKey::NewAdmin) {
            panic_with_error!(&e, UpgradeError::AdminMissing);
        }

        let now = e.ledger().timestamp();
        let delay = Self::get_timelock_delay(e.clone());
        let execute_at = now.saturating_add(delay);

        let mut approvals = Vec::new(&e);
        approvals.push_back(signer.clone());

        let pending = PendingUpgrade {
            wasm_hash: new_wasm_hash.clone(),
            new_version,
            proposer: signer.clone(),
            scheduled_at: now,
            execute_at,
            approvals,
        };
        e.storage().instance().set(&DataKey::Pending, &pending);

        e.events().publish(
            (symbol_short!("upg_sched"), signer),
            (new_wasm_hash, new_version, execute_at),
        );
    }

    pub fn approve_upgrade(e: Env, signer: Address) {
        signer.require_auth();
        Self::require_signer(&e, &signer);

        let mut pending: PendingUpgrade = e
            .storage()
            .instance()
            .get(&DataKey::Pending)
            .unwrap_or_else(|| panic_with_error!(&e, UpgradeError::NoPendingUpgrade));

        for approver in pending.approvals.iter() {
            if approver == signer {
                panic_with_error!(&e, UpgradeError::AlreadyApproved);
            }
        }

        pending.approvals.push_back(signer.clone());
        let count = pending.approvals.len();
        e.storage().instance().set(&DataKey::Pending, &pending);

        e.events().publish(
            (symbol_short!("upg_apprv"), signer),
            (count, Self::get_threshold(e.clone())),
        );
    }

    pub fn cancel_upgrade(e: Env, signer: Address) {
        signer.require_auth();
        if !Self::is_signer(&e, &signer) && signer != Self::get_admin(&e) {
            panic_with_error!(&e, UpgradeError::NotAuthorized);
        }

        if !e.storage().instance().has(&DataKey::Pending) {
            panic_with_error!(&e, UpgradeError::NoPendingUpgrade);
        }
        e.storage().instance().remove(&DataKey::Pending);

        e.events().publish((symbol_short!("upg_cncl"), signer), ());
    }

    pub fn execute_upgrade(e: Env, executor: Address) {
        executor.require_auth();
        Self::require_signer(&e, &executor);

        let pending: PendingUpgrade = e
            .storage()
            .instance()
            .get(&DataKey::Pending)
            .unwrap_or_else(|| panic_with_error!(&e, UpgradeError::NoPendingUpgrade));

        let threshold = Self::get_threshold(e.clone());
        if pending.approvals.len() < threshold {
            panic_with_error!(&e, UpgradeError::ThresholdNotMet);
        }

        if e.ledger().timestamp() < pending.execute_at {
            panic_with_error!(&e, UpgradeError::TimelockNotElapsed);
        }

        let current_version = Self::version(e.clone());
        if pending.new_version <= current_version {
            panic_with_error!(&e, UpgradeError::InvalidVersion);
        }
        if !e.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&e, UpgradeError::AdminMissing);
        }
        if !e.storage().instance().has(&DataKey::NewAdmin) {
            panic_with_error!(&e, UpgradeError::AdminMissing);
        }

        e.storage()
            .instance()
            .set(&DataKey::Version, &pending.new_version);

        e.deployer()
            .update_current_contract_wasm(pending.wasm_hash.clone());

        e.storage().instance().remove(&DataKey::Pending);

        e.events().publish(
            (
                symbol_short!("upgrade"),
                current_version,
                pending.new_version,
            ),
            pending.wasm_hash,
        );
    }

    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    fn get_admin(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::NewAdmin)
            .or_else(|| e.storage().instance().get(&DataKey::Admin))
            .unwrap_or_else(|| panic_with_error!(e, UpgradeError::AdminMissing))
    }

    fn require_admin(e: &Env) {
        let admin = Self::get_admin(e);
        admin.require_auth();
    }

    fn is_signer(e: &Env, who: &Address) -> bool {
        let signers: Vec<Address> = e
            .storage()
            .instance()
            .get(&DataKey::Signers)
            .unwrap_or_else(|| Vec::new(e));
        signers.iter().any(|s| &s == who)
    }

    fn require_signer(e: &Env, who: &Address) {
        if !Self::is_signer(e, who) {
            panic_with_error!(e, UpgradeError::NotAuthorized);
        }
    }

    fn validate_signer_config(e: &Env, signers: &Vec<Address>, threshold: u32) {
        let count = signers.len();
        if count == 0 {
            panic_with_error!(e, UpgradeError::EmptySigners);
        }
        if threshold == 0 || threshold > count {
            panic_with_error!(e, UpgradeError::InvalidThreshold);
        }
        for i in 0..count {
            let a = signers.get(i).unwrap();
            for j in (i + 1)..count {
                let b = signers.get(j).unwrap();
                if a == b {
                    panic_with_error!(e, UpgradeError::DuplicateSigner);
                }
            }
        }
    }
}
