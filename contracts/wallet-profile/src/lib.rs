//! # Wallet Profile Contract
//!
//! A Soroban smart contract for managing wallet profiles with
//! nickname update support and validation.
//!
//! ## Features
//!
//! - **Profile Creation**: Create wallet profiles with nicknames
//! - **Nickname Updates**: Update wallet nicknames without recreating records
//! - **Validation**: Rejects empty nicknames
//! - **Data Preservation**: Existing profile data is preserved on update
//! - **Event Emission**: Emits events for profile creation and updates

#![no_std]

mod validation;

use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, Symbol};

use crate::validation::validate_nickname;

/// Error codes for the wallet profile contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ProfileError {
    /// Contract not initialized
    NotInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Nickname is empty or invalid
    InvalidNickname = 3,
    /// Profile not found
    ProfileNotFound = 4,
    /// Profile already exists for this wallet
    ProfileAlreadyExists = 5,
}

impl From<ProfileError> for soroban_sdk::Error {
    fn from(e: ProfileError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}

/// Represents a wallet profile.
#[derive(Clone, Debug)]
#[contracttype]
pub struct WalletProfile {
    /// The wallet owner address
    pub user: Address,
    /// The wallet nickname
    pub nickname: Symbol,
    /// When the profile was created (ledger sequence)
    pub created_at: u64,
    /// When the profile was last updated
    pub updated_at: u64,
    /// Whether the profile is active
    pub is_active: bool,
}

/// Storage keys for the contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Profile(Address),
    TotalProfiles,
}

/// Events emitted by the contract.
pub struct ProfileEvents;

impl ProfileEvents {
    pub fn profile_created(env: &Env, profile: &WalletProfile) {
        let topics = (symbol_short!("profile"), symbol_short!("created"));
        env.events().publish(topics, (profile.user.clone(), profile.nickname.clone()));
    }

    pub fn nickname_updated(env: &Env, user: &Address, old_nickname: Symbol, new_nickname: Symbol) {
        let topics = (symbol_short!("profile"), symbol_short!("nickname"));
        env.events().publish(topics, (user.clone(), old_nickname, new_nickname));
    }
}

#[contract]
pub struct WalletProfileContract;

#[contractimpl]
impl WalletProfileContract {
    /// Initializes the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalProfiles, &0u64);
    }

    /// Creates a new wallet profile for a user.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `user` - The wallet owner (must authorize)
    /// * `nickname` - The wallet nickname
    ///
    /// # Returns
    /// * `WalletProfile` - The created profile
    pub fn create_profile(env: Env, user: Address, nickname: Symbol) -> WalletProfile {
        user.require_auth();

        // Validate nickname
        if validate_nickname(&nickname).is_err() {
            panic_with_error!(&env, ProfileError::InvalidNickname);
        }

        // Check profile doesn't already exist
        if env.storage().persistent().has(&DataKey::Profile(user.clone())) {
            panic_with_error!(&env, ProfileError::ProfileAlreadyExists);
        }

        let current_ledger = env.ledger().sequence() as u64;

        let profile = WalletProfile {
            user: user.clone(),
            nickname: nickname.clone(),
            created_at: current_ledger,
            updated_at: current_ledger,
            is_active: true,
        };

        // Store profile
        env.storage()
            .persistent()
            .set(&DataKey::Profile(user.clone()), &profile);

        // Update total count
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalProfiles)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalProfiles, &(total + 1));

        // Emit event
        ProfileEvents::profile_created(&env, &profile);

        profile
    }

    /// Updates the wallet nickname for an existing profile.
    ///
    /// Preserves all other profile data (created_at, is_active).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `user` - The wallet owner (must authorize)
    /// * `new_nickname` - The new wallet nickname
    ///
    /// # Returns
    /// * `WalletProfile` - The updated profile
    pub fn update_nickname(env: Env, user: Address, new_nickname: Symbol) -> WalletProfile {
        user.require_auth();

        // Validate new nickname
        if validate_nickname(&new_nickname).is_err() {
            panic_with_error!(&env, ProfileError::InvalidNickname);
        }

        // Fetch existing profile
        let mut profile: WalletProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Profile(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, ProfileError::ProfileNotFound));

        let old_nickname = profile.nickname.clone();

        // Update nickname and timestamp
        profile.nickname = new_nickname.clone();
        profile.updated_at = env.ledger().sequence() as u64;

        // Store updated profile
        env.storage()
            .persistent()
            .set(&DataKey::Profile(user.clone()), &profile);

        // Emit event
        ProfileEvents::nickname_updated(&env, &user, old_nickname, new_nickname);

        profile
    }

    /// Retrieves a wallet profile by user address.
    pub fn get_profile(env: Env, user: Address) -> Option<WalletProfile> {
        env.storage().persistent().get(&DataKey::Profile(user))
    }

    /// Returns the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized")
    }

    /// Updates the admin address.
    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&env, &current_admin);

        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    /// Returns the total number of profiles created.
    pub fn get_total_profiles(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalProfiles)
            .unwrap_or(0)
    }

    // Internal helper to verify admin
    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        if *caller != admin {
            panic_with_error!(env, ProfileError::Unauthorized);
        }
    }
}

#[cfg(test)]
mod test;
