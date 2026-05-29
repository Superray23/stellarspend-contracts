//! # Budget Contract
//!
//! A Soroban smart contract for managing user budgets with validation,
//! deletion cooldown protection, and event emission.
//!
//! ## Features
//!
//! - **Budget Updates**: Update single user budgets with validation
//! - **Deletion Cooldown**: 24-hour deletion delay with cancellation option
//! - **Validation**: Prevents negative or zero allocations
//! - **Event Emission**: Tracks budget updates and deletions
//! - **Atomic Operations**: Ensures reliable state changes

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env};

/// Error codes for the budget contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BudgetError {
    /// Contract not initialized
    NotInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Invalid budget amount (negative or zero)
    InvalidAmount = 3,
    /// User not found
    UserNotFound = 4,
    /// Deletion cooldown has not elapsed
    DeletionCooldownNotElapsed = 5,
    /// No pending deletion found
    NoPendingDeletion = 6,
}

impl From<BudgetError> for soroban_sdk::Error {
    fn from(e: BudgetError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}

/// Deletion cooldown period in seconds (24 hours).
pub const DELETION_COOLDOWN_SECONDS: u64 = 86_400;

/// Budget record for a user with multi-asset support.
#[derive(Clone, Debug)]
#[contracttype]
pub struct BudgetRecord {
    pub user: Address,
    pub amount: i128,
    /// Asset contract ID (native XLM if None)
    pub asset: Option<Address>,
    pub last_updated: u64,
}

/// Pending deletion record with cooldown expiry timestamp.
#[derive(Clone, Debug)]
#[contracttype]
pub struct PendingDeletion {
    /// The user whose budget is pending deletion
    pub user: Address,
    /// Ledger timestamp after which deletion is allowed
    pub cooldown_expiry: u64,
}

/// Storage keys for the contract.
#[derive(Clone, Debug)]
#[contracttype]
pub enum DataKey {
    /// Admin address
    Admin,
    /// Budget by user address (default/native asset)
    Budget(Address),
    /// Budget by (user, asset) pair for multi-asset support
    BudgetAsset(Address, Address),
    /// List of assets owned by a user
    UserAssets(Address),
    /// Total amount allocated across all budgets
    TotalAllocated,
    /// Pending deletion cooldown by user address
    PendingDeletion(Address),
}

#[contract]
pub struct BudgetContract;

#[contractimpl]
impl BudgetContract {
    /// Initializes the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalAllocated, &0i128);
    }

    /// Updates a single user's budget with optional multi-asset support.
    ///
    /// When `asset` is None, the budget applies to the native asset (XLM).
    /// When `asset` is Some, the budget is tied to that specific token contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address calling the function
    /// * `user` - The user address to update budget for
    /// * `amount` - The new budget amount
    /// * `asset` - Optional asset contract ID for multi-asset budgets
    pub fn update_budget(
        env: Env,
        admin: Address,
        user: Address,
        amount: i128,
        asset: Option<Address>,
    ) {
        // Verify admin authority
        admin.require_auth();
        Self::require_admin(&env, &admin);

        // Validate amount
        if amount <= 0 {
            panic_with_error!(&env, BudgetError::InvalidAmount);
        }

        let current_time = env.ledger().timestamp();

        // Get current total allocated
        let mut total_allocated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAllocated)
            .unwrap_or(0);

        // Check if user exists and get old amount
        if let Some(old_record) = env
            .storage()
            .persistent()
            .get::<DataKey, BudgetRecord>(&DataKey::Budget(user.clone()))
        {
            // Subtract old amount from total
            total_allocated = total_allocated.checked_sub(old_record.amount).unwrap_or(0);
        }

        // Add new amount to total
        total_allocated = total_allocated.checked_add(amount).unwrap_or(i128::MAX);

        // Create new budget record
        let record = BudgetRecord {
            user: user.clone(),
            amount,
            asset: asset.clone(),
            last_updated: current_time,
        };

        // Store the updated budget (use asset-specific key when applicable)
        if let Some(ref asset_addr) = asset {
            env.storage()
                .persistent()
                .set(
                    &DataKey::BudgetAsset(user.clone(), asset_addr.clone()),
                    &record,
                );
            // Track asset in user's asset list
            let mut user_assets: Vec<Address> = env
                .storage()
                .persistent()
                .get(&DataKey::UserAssets(user.clone()))
                .unwrap_or(Vec::new(&env));
            if !user_assets.contains(asset_addr) {
                user_assets.push_back(asset_addr.clone());
                env.storage()
                    .persistent()
                    .set(&DataKey::UserAssets(user.clone()), &user_assets);
            }
        } else {
            env.storage()
                .persistent()
                .set(&DataKey::Budget(user.clone()), &record);
        }

        // Update total allocated
        env.storage()
            .instance()
            .set(&DataKey::TotalAllocated, &total_allocated);

        // Emit update event
        env.events().publish(
            (symbol_short!("budget"), symbol_short!("updated")),
            (user, amount, current_time),
        );
    }

    /// Schedules a budget for deletion with a 24-hour cooldown.
    ///
    /// After scheduling, the budget remains active until the cooldown
    /// expires and `execute_deletion` is called.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must authorize)
    /// * `user` - The user whose budget is scheduled for deletion
    pub fn schedule_deletion(env: Env, admin: Address, user: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        // Verify the budget exists
        if env
            .storage()
            .persistent()
            .get::<DataKey, BudgetRecord>(&DataKey::Budget(user.clone()))
            .is_none()
        {
            panic_with_error!(&env, BudgetError::UserNotFound);
        }

        let current_time = env.ledger().timestamp();
        let cooldown_expiry = current_time.checked_add(DELETION_COOLDOWN_SECONDS).unwrap_or(u64::MAX);

        let pending = PendingDeletion {
            user: user.clone(),
            cooldown_expiry,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingDeletion(user.clone()), &pending);

        // Emit deletion scheduled event
        env.events().publish(
            (symbol_short!("budget"), symbol_short!("del_sched")),
            (user, cooldown_expiry),
        );
    }

    /// Cancels a pending budget deletion.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must authorize)
    /// * `user` - The user whose deletion is being cancelled
    pub fn cancel_deletion(env: Env, admin: Address, user: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        // Verify a pending deletion exists
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingDeletion(user.clone()))
        {
            panic_with_error!(&env, BudgetError::NoPendingDeletion);
        }

        // Remove the pending deletion
        env.storage()
            .persistent()
            .remove(&DataKey::PendingDeletion(user.clone()));

        // Emit cancellation event
        env.events().publish(
            (symbol_short!("budget"), symbol_short!("del_canc")),
            user,
        );
    }

    /// Executes a scheduled budget deletion after the cooldown period has elapsed.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must authorize)
    /// * `user` - The user whose budget is being deleted
    pub fn execute_deletion(env: Env, admin: Address, user: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        // Verify a pending deletion exists and cooldown has elapsed
        let pending: PendingDeletion = env
            .storage()
            .persistent()
            .get(&DataKey::PendingDeletion(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, BudgetError::NoPendingDeletion));

        let current_time = env.ledger().timestamp();
        if current_time < pending.cooldown_expiry {
            panic_with_error!(&env, BudgetError::DeletionCooldownNotElapsed);
        }

        // Get the budget amount to adjust total (check both native and per-asset)
        let mut old_amount = env
            .storage()
            .persistent()
            .get::<DataKey, BudgetRecord>(&DataKey::Budget(user.clone()))
            .map(|r| r.amount)
            .unwrap_or(0);

        // Also clean up multi-asset budgets for the user
        let user_assets: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::UserAssets(user.clone()))
            .unwrap_or(Vec::new(&env));
        for asset in user_assets.iter() {
            if let Some(record) = env
                .storage()
                .persistent()
                .get::<DataKey, BudgetRecord>(&DataKey::BudgetAsset(
                    user.clone(),
                    asset.clone(),
                ))
            {
                old_amount = old_amount.checked_add(record.amount).unwrap_or(old_amount);
            }
            env.storage()
                .persistent()
                .remove(&DataKey::BudgetAsset(user.clone(), asset.clone()));
        }
        env.storage()
            .persistent()
            .remove(&DataKey::UserAssets(user.clone()));

        // Remove the budget
        env.storage()
            .persistent()
            .remove(&DataKey::Budget(user.clone()));

        // Remove the pending deletion
        env.storage()
            .persistent()
            .remove(&DataKey::PendingDeletion(user.clone()));

        // Update total allocated
        let total_allocated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAllocated)
            .unwrap_or(0);
        let new_total = total_allocated.checked_sub(old_amount).unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalAllocated, &new_total);

        // Emit deletion event
        env.events().publish(
            (symbol_short!("budget"), symbol_short!("deleted")),
            (user, current_time),
        );
    }

    /// Returns the pending deletion for a user, if one exists.
    pub fn get_pending_deletion(env: Env, user: Address) -> Option<PendingDeletion> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingDeletion(user))
    }

    /// Retrieves the budget for a specific user (default/native asset).
    pub fn get_budget(env: Env, user: Address) -> Option<BudgetRecord> {
        env.storage().persistent().get(&DataKey::Budget(user))
    }

    /// Retrieves the budget for a specific user and asset.
    pub fn get_budget_by_asset(env: Env, user: Address, asset: Address) -> Option<BudgetRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::BudgetAsset(user, asset))
    }

    /// Returns all asset contract IDs for a user's multi-asset budgets.
    pub fn get_user_assets(env: Env, user: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::UserAssets(user))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized")
    }

    /// Returns the total allocated budget amount.
    pub fn get_total_allocated(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalAllocated)
            .unwrap_or(0)
    }

    /// Internal helper to verify admin authority.
    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if *caller != admin {
            panic_with_error!(env, BudgetError::Unauthorized);
        }
    }
}
