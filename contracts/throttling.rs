use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, Map, Vec, U256,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    ThrottleConfig,
    WalletThrottleState(Address),
    WalletTransactionHistory(Address, u64), // wallet_address, timestamp_slot
    GlobalThrottleStats,
    ThrottledWallets,
    TimeWindowData(u64), // timestamp_slot
}

#[derive(Clone)]
#[contracttype]
pub struct ThrottleConfig {
    pub max_transactions_per_window: u32,
    pub window_size_seconds: u64,
    pub block_duration_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub enabled: bool,
    pub exempt_addresses: Vec<Address>,
}

#[derive(Clone)]
#[contracttype]
pub struct WalletThrottleState {
    pub wallet_address: Address,
    pub transaction_count: u32,
    pub window_start: u64,
    pub last_transaction_time: u64,
    pub is_throttled: bool,
    pub throttle_start_time: u64,
    pub violation_count: u32,
    pub total_transactions_all_time: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct ThrottleViolation {
    pub wallet_address: Address,
    pub violation_time: u64,
    pub transaction_count: u32,
    pub window_size: u64,
    pub max_allowed: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct GlobalThrottleStats {
    pub total_transactions_checked: u64,
    pub total_violations: u64,
    pub currently_throttled_wallets: u32,
    pub last_cleanup_time: u64,
    /// Violation rate scaled by 10_000 (e.g. 2500 = 25.00%).
    pub avg_tx_per_window: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct ThrottleResult {
    pub allowed: bool,
    pub reason: ThrottleReason,
    pub remaining_transactions: u32,
    pub window_reset_time: u64,
    pub throttle_end_time: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ThrottleReason {
    Allowed = 0,
    ExceededFrequency = 1,
    CurrentlyThrottled = 2,
    WalletExempt = 3,
    SystemDisabled = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeWindow {
    OneMinute = 60,
    FiveMinutes = 300,
    OneHour = 3600,
    OneDay = 86400,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ThrottleError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidConfig = 4,
    WalletNotFound = 5,
    InvalidTimeWindow = 6,
    StorageError = 7,
    Overflow = 8,
    InvalidAddress = 9,
}

pub struct ThrottleEvents;

impl ThrottleEvents {
    pub fn throttle_triggered(env: &Env, wallet: &Address, violation: &ThrottleViolation) {
        let topics = (symbol_short!("throttle"), symbol_short!("triggered"));
        env.events().publish(
            topics,
            (
                wallet.clone(),
                violation.transaction_count,
                violation.max_allowed,
                violation.window_size,
                violation.violation_time,
            ),
        );
    }

    pub fn throttle_lifted(env: &Env, wallet: &Address, duration: u64) {
        let topics = (symbol_short!("throttle"), symbol_short!("lifted"));
        env.events()
            .publish(topics, (wallet.clone(), duration, env.ledger().timestamp()));
    }

    pub fn transaction_allowed(env: &Env, wallet: &Address, remaining: u32) {
        let topics = (symbol_short!("throttle"), symbol_short!("allowed"));
        env.events().publish(
            topics,
            (wallet.clone(), remaining, env.ledger().timestamp()),
        );
    }

    pub fn config_updated(env: &Env, admin: &Address, config: &ThrottleConfig) {
        let topics = (symbol_short!("throttle"), symbol_short!("cfg_upd"));
        env.events().publish(
            topics,
            (
                admin.clone(),
                config.max_transactions_per_window,
                config.window_size_seconds,
                config.block_duration_seconds,
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn wallet_exempted(env: &Env, admin: &Address, wallet: &Address) {
        let topics = (symbol_short!("throttle"), symbol_short!("exempted"));
        env.events().publish(
            topics,
            (admin.clone(), wallet.clone(), env.ledger().timestamp()),
        );
    }

    pub fn cleanup_performed(env: &Env, cleaned_wallets: u32, freed_space: u64) {
        let topics = (symbol_short!("throttle"), symbol_short!("cleanup"));
        env.events().publish(
            topics,
            (cleaned_wallets, freed_space, env.ledger().timestamp()),
        );
    }

    pub fn violation_recorded(env: &Env, wallet: &Address, violation_count: u32) {
        let topics = (symbol_short!("throttle"), symbol_short!("violation"));
        env.events().publish(
            topics,
            (wallet.clone(), violation_count, env.ledger().timestamp()),
        );
    }
}

pub fn initialize_throttle_contract(env: &Env, admin: Address, config: ThrottleConfig) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(env, ThrottleError::AlreadyInitialized);
    }

    // Validate configuration
    validate_config(&env, &config);

    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage()
        .instance()
        .set(&DataKey::ThrottleConfig, &config);
    env.storage()
        .instance()
        .set(&DataKey::ThrottledWallets, &Vec::<Address>::new(&env));

    let initial_stats = GlobalThrottleStats {
        total_transactions_checked: 0,
        total_violations: 0,
        currently_throttled_wallets: 0,
        last_cleanup_time: env.ledger().timestamp(),
        avg_tx_per_window: 0,
    };
    env.storage()
        .instance()
        .set(&DataKey::GlobalThrottleStats, &initial_stats);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, ThrottleError::NotInitialized))
}

pub fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin = get_admin(env);
    if admin != caller.clone() {
        panic_with_error!(env, ThrottleError::Unauthorized);
    }
}

pub fn check_transaction_throttle(env: &Env, wallet_address: Address) -> ThrottleResult {
    let config = get_throttle_config(env);

    // Check if throttling is enabled
    if !config.enabled {
        return ThrottleResult {
            allowed: true,
            reason: ThrottleReason::SystemDisabled,
            remaining_transactions: u32::MAX,
            window_reset_time: 0,
            throttle_end_time: None,
        };
    }

    // Check if wallet is exempt
    if config.exempt_addresses.contains(&wallet_address) {
        return ThrottleResult {
            allowed: true,
            reason: ThrottleReason::WalletExempt,
            remaining_transactions: u32::MAX,
            window_reset_time: 0,
            throttle_end_time: None,
        };
    }

    let current_time = env.ledger().timestamp();

    // Perform cleanup if needed
    maybe_cleanup_old_data(env, current_time);

    // Get or create wallet state
    let mut wallet_state = get_wallet_throttle_state(env, &wallet_address);

    // Check if wallet is currently throttled
    if wallet_state.is_throttled {
        if current_time < wallet_state.throttle_start_time + config.block_duration_seconds {
            return ThrottleResult {
                allowed: false,
                reason: ThrottleReason::CurrentlyThrottled,
                remaining_transactions: 0,
                window_reset_time: wallet_state.window_start + config.window_size_seconds,
                throttle_end_time: Some(
                    wallet_state.throttle_start_time + config.block_duration_seconds,
                ),
            };
        } else {
            // Throttle period expired, reset state
            wallet_state.is_throttled = false;
            wallet_state.transaction_count = 0;
            wallet_state.window_start = current_time;
            wallet_state.violation_count = 0;

            // Remove from throttled wallets list
            remove_from_throttled_wallets(env, &wallet_address);

            ThrottleEvents::throttle_lifted(env, &wallet_address, config.block_duration_seconds);
        }
    }

    // Check if we need to reset the window
    if current_time >= wallet_state.window_start + config.window_size_seconds {
        wallet_state.transaction_count = 0;
        wallet_state.window_start = current_time;
    }

    // Check if transaction would exceed limit
    if wallet_state.transaction_count >= config.max_transactions_per_window {
        // Trigger throttling
        wallet_state.is_throttled = true;
        wallet_state.throttle_start_time = current_time;
        wallet_state.violation_count = wallet_state
            .violation_count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(env, ThrottleError::Overflow));

        let violation = ThrottleViolation {
            wallet_address: wallet_address.clone(),
            violation_time: current_time,
            transaction_count: wallet_state.transaction_count + 1,
            window_size: config.window_size_seconds,
            max_allowed: config.max_transactions_per_window,
        };

        // Add to throttled wallets list
        add_to_throttled_wallets(env, &wallet_address);

        // Update global stats
        update_global_stats(env, true);

        // Save state
        save_wallet_throttle_state(env, &wallet_address, &wallet_state);

        // Emit events
        ThrottleEvents::throttle_triggered(env, &wallet_address, &violation);
        ThrottleEvents::violation_recorded(env, &wallet_address, wallet_state.violation_count);

        return ThrottleResult {
            allowed: false,
            reason: ThrottleReason::ExceededFrequency,
            remaining_transactions: 0,
            window_reset_time: wallet_state.window_start + config.window_size_seconds,
            throttle_end_time: Some(current_time + config.block_duration_seconds),
        };
    }

    // Transaction is allowed - use checked arithmetic
    wallet_state.transaction_count = wallet_state
        .transaction_count
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, ThrottleError::Overflow));
    wallet_state.last_transaction_time = current_time;
    wallet_state.total_transactions_all_time = wallet_state
        .total_transactions_all_time
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, ThrottleError::Overflow));

    let remaining = config.max_transactions_per_window - wallet_state.transaction_count;

    // Update global stats
    update_global_stats(env, false);

    // Save state
    save_wallet_throttle_state(env, &wallet_address, &wallet_state);

    // Emit event
    ThrottleEvents::transaction_allowed(env, &wallet_address, remaining);

    ThrottleResult {
        allowed: true,
        reason: ThrottleReason::Allowed,
        remaining_transactions: remaining,
        window_reset_time: wallet_state.window_start + config.window_size_seconds,
        throttle_end_time: None,
    }
}

pub fn update_throttle_config(env: &Env, caller: Address, new_config: ThrottleConfig) {
    require_admin(env, &caller);
    validate_config(&env, &new_config);

    env.storage()
        .instance()
        .set(&DataKey::ThrottleConfig, &new_config);
    ThrottleEvents::config_updated(env, &caller, &new_config);
}

pub fn add_exempt_address(env: &Env, caller: Address, wallet_address: Address) {
    require_admin(env, &caller);

    let mut config = get_throttle_config(env);
    if !config.exempt_addresses.contains(&wallet_address) {
        config.exempt_addresses.push_back(wallet_address.clone());
        env.storage()
            .instance()
            .set(&DataKey::ThrottleConfig, &config);
        ThrottleEvents::wallet_exempted(env, &caller, &wallet_address);
    }
}

pub fn remove_exempt_address(env: &Env, caller: Address, wallet_address: Address) {
    require_admin(env, &caller);

    let mut config = get_throttle_config(env);
    let mut found = false;
    let mut new_exempt_list = Vec::<Address>::new(&env);

    for addr in config.exempt_addresses.iter() {
        if addr != wallet_address {
            new_exempt_list.push_back(addr);
        } else {
            found = true;
        }
    }

    if found {
        config.exempt_addresses = new_exempt_list;
        env.storage()
            .instance()
            .set(&DataKey::ThrottleConfig, &config);
    }
}

pub fn get_wallet_throttle_info(env: &Env, wallet_address: Address) -> Option<WalletThrottleState> {
    Some(get_wallet_throttle_state(env, &wallet_address))
}

pub fn get_throttled_wallets(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::ThrottledWallets)
        .unwrap_or_else(|| Vec::new(&env))
}

pub fn get_global_throttle_stats(env: &Env) -> GlobalThrottleStats {
    env.storage()
        .instance()
        .get(&DataKey::GlobalThrottleStats)
        .unwrap_or_else(|| GlobalThrottleStats {
            total_transactions_checked: 0,
            total_violations: 0,
            currently_throttled_wallets: 0,
            last_cleanup_time: 0,
            avg_tx_per_window: 0,
        })
}

pub fn force_cleanup(env: &Env, caller: Address) {
    require_admin(env, &caller);
    let current_time = env.ledger().timestamp();
    cleanup_old_data(env, current_time);
}

pub fn reset_wallet_throttle_state(env: &Env, caller: Address, wallet_address: Address) {
    require_admin(env, &caller);

    let config = get_throttle_config(env);
    let current_time = env.ledger().timestamp();

    let reset_state = WalletThrottleState {
        wallet_address: wallet_address.clone(),
        transaction_count: 0,
        window_start: current_time,
        last_transaction_time: 0,
        is_throttled: false,
        throttle_start_time: 0,
        violation_count: 0,
        total_transactions_all_time: 0,
    };

    save_wallet_throttle_state(env, &wallet_address, &reset_state);
    remove_from_throttled_wallets(env, &wallet_address);
}

// Helper functions

fn validate_config(env: &Env, config: &ThrottleConfig) {
    if config.max_transactions_per_window == 0 {
        panic_with_error!(env, ThrottleError::InvalidConfig);
    }
    if config.window_size_seconds == 0 {
        panic_with_error!(env, ThrottleError::InvalidConfig);
    }
    if config.block_duration_seconds == 0 {
        panic_with_error!(env, ThrottleError::InvalidConfig);
    }
    if config.cleanup_interval_seconds == 0 {
        panic_with_error!(env, ThrottleError::InvalidConfig);
    }
}

fn get_throttle_config(env: &Env) -> ThrottleConfig {
    env.storage()
        .instance()
        .get(&DataKey::ThrottleConfig)
        .unwrap_or_else(|| panic_with_error!(env, ThrottleError::NotInitialized))
}

fn get_wallet_throttle_state(env: &Env, wallet_address: &Address) -> WalletThrottleState {
    env.storage()
        .persistent()
        .get(&DataKey::WalletThrottleState(wallet_address.clone()))
        .unwrap_or_else(|| WalletThrottleState {
            wallet_address: wallet_address.clone(),
            transaction_count: 0,
            window_start: env.ledger().timestamp(),
            last_transaction_time: 0,
            is_throttled: false,
            throttle_start_time: 0,
            violation_count: 0,
            total_transactions_all_time: 0,
        })
}

fn save_wallet_throttle_state(env: &Env, wallet_address: &Address, state: &WalletThrottleState) {
    env.storage()
        .persistent()
        .set(&DataKey::WalletThrottleState(wallet_address.clone()), state);
}

fn add_to_throttled_wallets(env: &Env, wallet_address: &Address) {
    let mut throttled_wallets = get_throttled_wallets(env);
    if !throttled_wallets.contains(wallet_address) {
        throttled_wallets.push_back(wallet_address.clone());
        env.storage()
            .instance()
            .set(&DataKey::ThrottledWallets, &throttled_wallets);
    }
}

fn remove_from_throttled_wallets(env: &Env, wallet_address: &Address) {
    let throttled_wallets = get_throttled_wallets(env);
    let mut new_list = Vec::<Address>::new(&env);

    for addr in throttled_wallets.iter() {
        if addr != *wallet_address {
            new_list.push_back(addr);
        }
    }

    env.storage()
        .instance()
        .set(&DataKey::ThrottledWallets, &new_list);
}

fn update_global_stats(env: &Env, is_violation: bool) {
    let mut stats = get_global_throttle_stats(env);
    stats.total_transactions_checked += 1;

    if is_violation {
        stats.total_violations += 1;
    }

    let throttled_wallets = get_throttled_wallets(env);
    stats.currently_throttled_wallets = throttled_wallets.len() as u32;

    // Update average (simplified calculation)
    if stats.total_transactions_checked > 0 {
        stats.avg_tx_per_window = stats
            .total_violations
            .saturating_mul(10_000)
            / stats.total_transactions_checked;
    }

    env.storage()
        .instance()
        .set(&DataKey::GlobalThrottleStats, &stats);
}

fn maybe_cleanup_old_data(env: &Env, current_time: u64) {
    let config = get_throttle_config(env);
    let stats = get_global_throttle_stats(env);

    if current_time >= stats.last_cleanup_time + config.cleanup_interval_seconds {
        cleanup_old_data(env, current_time);
    }
}

fn cleanup_old_data(env: &Env, current_time: u64) {
    let config = get_throttle_config(env);
    let mut cleaned_wallets = 0u32;

    // This is a simplified cleanup - in production, you'd need a way to iterate
    // through all wallet states and clean up expired ones

    let mut stats = get_global_throttle_stats(env);
    stats.last_cleanup_time = current_time;
    env.storage()
        .instance()
        .set(&DataKey::GlobalThrottleStats, &stats);

    ThrottleEvents::cleanup_performed(env, cleaned_wallets, 0);
}

#[contract]
pub struct ThrottleContract;

#[contractimpl]
impl ThrottleContract {
    pub fn initialize(env: Env, admin: Address, config: ThrottleConfig) {
        initialize_throttle_contract(&env, admin, config);
    }

    pub fn get_admin(env: Env) -> Address {
        get_admin(&env)
    }

    pub fn check_transaction_throttle(env: Env, wallet_address: Address) -> ThrottleResult {
        check_transaction_throttle(&env, wallet_address)
    }

    pub fn update_throttle_config(env: Env, caller: Address, new_config: ThrottleConfig) {
        update_throttle_config(&env, caller, new_config);
    }

    pub fn add_exempt_address(env: Env, caller: Address, wallet_address: Address) {
        add_exempt_address(&env, caller, wallet_address);
    }

    pub fn remove_exempt_address(env: Env, caller: Address, wallet_address: Address) {
        remove_exempt_address(&env, caller, wallet_address);
    }

    pub fn get_wallet_throttle_info(
        env: Env,
        wallet_address: Address,
    ) -> Option<WalletThrottleState> {
        get_wallet_throttle_info(&env, wallet_address)
    }

    pub fn get_throttled_wallets(env: Env) -> Vec<Address> {
        get_throttled_wallets(&env)
    }

    pub fn get_global_throttle_stats(env: Env) -> GlobalThrottleStats {
        get_global_throttle_stats(&env)
    }

    pub fn force_cleanup(env: Env, caller: Address) {
        force_cleanup(&env, caller);
    }

    pub fn reset_wallet_throttle_state(env: Env, caller: Address, wallet_address: Address) {
        reset_wallet_throttle_state(&env, caller, wallet_address);
    }

    pub fn get_throttle_config(env: Env) -> ThrottleConfig {
        get_throttle_config(&env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env, Vec,
    };

    fn setup() -> (Env, Address, ThrottleContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ThrottleContract, ());
        let client = ThrottleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let config = ThrottleConfig {
            max_transactions_per_window: 3,
            window_size_seconds: 60,
            block_duration_seconds: 30,
            cleanup_interval_seconds: 300,
            enabled: true,
            exempt_addresses: Vec::new(&env),
        };
        client.initialize(&admin, &config);
        (env, admin, client)
    }

    #[test]
    fn test_limit_reached_blocks_wallet() {
        let (env, _admin, client) = setup();
        let wallet = Address::generate(&env);

        for _ in 0..3 {
            assert!(client.check_transaction_throttle(&wallet).allowed);
        }

        let blocked = client.check_transaction_throttle(&wallet);
        assert!(!blocked.allowed);
        assert_eq!(blocked.reason, ThrottleReason::ExceededFrequency);
        assert_eq!(blocked.remaining_transactions, 0);
    }

    #[test]
    fn test_window_reset_allows_transactions_again() {
        let (env, _admin, client) = setup();
        let wallet = Address::generate(&env);

        for _ in 0..3 {
            client.check_transaction_throttle(&wallet);
        }
        assert!(!client.check_transaction_throttle(&wallet).allowed);

        env.ledger().with_mut(|li| {
            li.timestamp += 61;
        });

        let after_reset = client.check_transaction_throttle(&wallet);
        assert!(after_reset.allowed);
        assert_eq!(after_reset.remaining_transactions, 2);
    }

    #[test]
    fn test_admin_reset_bypasses_throttle() {
        let (env, admin, client) = setup();
        let wallet = Address::generate(&env);

        for _ in 0..3 {
            client.check_transaction_throttle(&wallet);
        }
        assert!(!client.check_transaction_throttle(&wallet).allowed);

        client.reset_wallet_throttle_state(&admin, &wallet);

        let after_reset = client.check_transaction_throttle(&wallet);
        assert!(after_reset.allowed);
        assert_eq!(after_reset.remaining_transactions, 2);
    }
}
