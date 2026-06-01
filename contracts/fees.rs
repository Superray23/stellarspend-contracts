use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, String, Vec,
};
use crate::treasury::TreasuryContractClient;

// =============================================================================
// Rounding Modes for Fee Calculation
// =============================================================================

/// Rounding modes for fee calculations.
/// Defines how fractional fees are handled when converting to whole numbers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum RoundingMode {
    /// Round down to the nearest whole number (floor).
    /// Provides conservative fee estimation, users benefit.
    Floor = 0,
    /// Round to the nearest whole number (standard rounding).
    /// 0.5 or more rounds up, less rounds down.
    Round = 1,
    /// Round up to the nearest whole number (ceiling).
    /// Ensures minimum expected revenue, may overcharge users slightly.
    Ceiling = 2,
}

impl Default for RoundingMode {
    fn default() -> Self {
        RoundingMode::Round
    }
}

impl RoundingMode {
    /// Convert from u32 to RoundingMode
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(RoundingMode::Floor),
            1 => Some(RoundingMode::Round),
            2 => Some(RoundingMode::Ceiling),
            _ => None,
        }
    }

    /// Convert RoundingMode to u32
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

// =============================================================================
// Fee Categories for Reporting
// =============================================================================

/// Fee categories for granular reporting and analytics.
/// Allows fees to be split and tracked by type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum FeeCategory {
    /// Standard transaction fees
    Transaction = 0,
    /// Priority/expedited processing fees
    Priority = 1,
    /// Cross-contract interaction fees
    CrossContract = 2,
    /// Batch operation fees
    Batch = 3,
    /// Token transfer fees
    TokenTransfer = 4,
    /// Wallet creation fees
    WalletCreation = 5,
    /// Other/unclassified fees
    Other = 6,
}

impl Default for FeeCategory {
    fn default() -> Self {
        FeeCategory::Transaction
    }
}

impl FeeCategory {
    /// Convert from u32 to FeeCategory
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(FeeCategory::Transaction),
            1 => Some(FeeCategory::Priority),
            2 => Some(FeeCategory::CrossContract),
            3 => Some(FeeCategory::Batch),
            4 => Some(FeeCategory::TokenTransfer),
            5 => Some(FeeCategory::WalletCreation),
            6 => Some(FeeCategory::Other),
            _ => None,
        }
    }

    /// Convert FeeCategory to u32
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

// =============================================================================
// Fee Snapshot for Periodic State Capture
// =============================================================================

/// Represents a snapshot of fee state at a specific point in time.
/// Used for historical tracking and periodic reporting.
#[derive(Clone, Debug)]
#[contracttype]
pub struct FeeSnapshot {
    /// Total fees collected at snapshot time
    pub total_collected: i128,
    /// Total fees routed to treasury at snapshot time
    pub treasury_collected: i128,
    /// Total fees distributed to recipients at snapshot time
    pub distributed: i128,
    /// Timestamp when snapshot was created
    pub created_at: u64,
    /// Period start timestamp for this snapshot
    pub period_start: u64,
}

/// Fee snapshot metadata for tracking snapshot history
#[derive(Clone, Debug)]
#[contracttype]
pub struct SnapshotMetadata {
    /// Number of snapshots created
    pub count: u64,
    /// Latest snapshot timestamp
    pub latest_timestamp: u64,
}

// =============================================================================
// Priority Levels for Fee Calculation
// =============================================================================

/// Priority levels for transaction execution.
/// Higher priority levels result in higher fees for faster execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum PriorityLevel {
    /// Low priority - lowest fees, slowest execution
    Low = 0,
    /// Medium priority - standard fees, normal execution (default)
    Medium = 1,
    /// High priority - higher fees, faster execution
    High = 2,
    /// Urgent priority - highest fees, fastest execution
    Urgent = 3,
}

impl Default for PriorityLevel {
    fn default() -> Self {
        PriorityLevel::Medium
    }
}

impl PriorityLevel {
    /// Convert from u32 to PriorityLevel
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(PriorityLevel::Low),
            1 => Some(PriorityLevel::Medium),
            2 => Some(PriorityLevel::High),
            3 => Some(PriorityLevel::Urgent),
            _ => None,
        }
    }

    /// Convert PriorityLevel to u32
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

/// Configuration for priority-based fee multipliers.
/// Each priority level has a multiplier applied to the base fee rate.
#[derive(Clone, Debug)]
#[contracttype]
pub struct PriorityFeeConfig {
    /// Multiplier for Low priority (e.g., 8000 = 0.8x, 80% of base fee)
    pub low_multiplier_bps: u32,
    /// Multiplier for Medium priority (e.g., 10000 = 1.0x, 100% of base fee)
    pub medium_multiplier_bps: u32,
    /// Multiplier for High priority (e.g., 15000 = 1.5x, 150% of base fee)
    pub high_multiplier_bps: u32,
    /// Multiplier for Urgent priority (e.g., 20000 = 2.0x, 200% of base fee)
    pub urgent_multiplier_bps: u32,
}

impl Default for PriorityFeeConfig {
    fn default() -> Self {
        Self {
            low_multiplier_bps: 8000,     // 0.8x - 20% discount
            medium_multiplier_bps: 10000, // 1.0x - base rate
            high_multiplier_bps: 15000,   // 1.5x - 50% premium
            urgent_multiplier_bps: 20000, // 2.0x - 100% premium
        }
    }
}

impl PriorityFeeConfig {
    /// Get the multiplier for a given priority level in basis points
    pub fn get_multiplier_bps(&self, priority: PriorityLevel) -> u32 {
        match priority {
            PriorityLevel::Low => self.low_multiplier_bps,
            PriorityLevel::Medium => self.medium_multiplier_bps,
            PriorityLevel::High => self.high_multiplier_bps,
            PriorityLevel::Urgent => self.urgent_multiplier_bps,
        }
    }

    /// Validate that multipliers are in ascending order (higher priority = higher fee)
    pub fn is_valid(&self) -> bool {
        self.low_multiplier_bps <= self.medium_multiplier_bps
            && self.medium_multiplier_bps <= self.high_multiplier_bps
            && self.high_multiplier_bps <= self.urgent_multiplier_bps
    }
}

// =============================================================================
// Fee Structures
// =============================================================================

/// Represents a fee distribution recipient and their share.
#[derive(Clone, Debug)]
#[contracttype]
pub struct FeeRecipient {
    /// Address of the recipient
    pub address: Address,
    /// Share in basis points (bps). Must be 0–10_000.
    /// All recipients' shares must sum to 10_000 (100%).
    pub share_bps: u32,
}

/// Storage keys used by the fees contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    /// Fee percentage stored in basis points (bps).
    /// The value is expected to be between 0 and 10_000 (100%).
    FeePercentage,
    /// Cumulative fees that have been collected through `deduct_fee`.
    TotalFeesCollected,
    /// Per-user fee accrual tracking. Stores total fees paid by each user.
    UserFeesAccrued(Address),
    /// Fee distribution configuration. Stores vector of FeeRecipient.
    FeeDistribution,
    /// Cumulative fees distributed to a specific recipient.
    RecipientFeesAccumulated(Address),
    /// Minimum fee threshold. Fees cannot be less than this value.
    MinFee,
    /// Maximum fee threshold. Fees cannot exceed this value.
    MaxFee,
    /// Priority fee configuration with multipliers for each priority level.
    PriorityFeeConfig,
    /// Fees currently held in escrow before being added to TotalFeesCollected.
    EscrowedFees(Address),
    /// Fee delegation mapping (User -> Delegate Payer).
    FeeDelegate(Address),
    /// Treasury address for routing fees
    TreasuryAddress,
    /// Cumulative fees routed to treasury
    TotalTreasuryFees,
    /// Treasury fee percentage (portion of fees going to treasury)
    TreasuryFeePercentage,
    /// Rounding mode for fee calculations
    RoundingMode,
    /// Fee category mapping (operation type -> fee category)
    FeeCategoryMap(Address),
    /// Cumulative fees by category
    CategoryFees(FeeCategory),
    /// Fee snapshot metadata
    SnapshotMetadata,
    /// Fee snapshot at a specific period
    FeeSnapshot(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FeeError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidPercentage = 4,
    InvalidAmount = 5,
    Overflow = 6,
    /// Refund amount is invalid (e.g., zero or negative).
    InvalidRefundAmount = 7,
    /// User has insufficient fee balance for the requested refund.
    InsufficientFeeBalance = 8,
    /// Distribution configuration is invalid (empty, exceeds 100%, or contains invalid shares).
    InvalidDistribution = 9,
    /// Total distribution shares do not equal 100% (10_000 bps).
    DistributionSumsToWrong = 10,
    /// No fee distribution has been configured yet.
    NoDistributionConfigured = 11,
    /// Min fee is negative or max fee is negative.
    InvalidFeeBound = 12,
    /// Max fee is less than min fee.
    InvalidFeeBoundRange = 13,
    /// Invalid priority level provided.
    InvalidPriorityLevel = 14,
    /// Priority multiplier configuration is invalid (not in ascending order).
    InvalidPriorityConfig = 15,
    /// No escrowed fees found for the user.
    NoEscrowedFees = 16,
    /// Treasury address not configured.
    TreasuryNotConfigured = 17,
    /// Invalid treasury percentage (must be 0-10000).
    InvalidTreasuryPercentage = 18,
    /// Invalid rounding mode provided.
    InvalidRoundingMode = 19,
    /// Invalid fee category provided.
    InvalidFeeCategory = 20,
    /// Snapshot period must be greater than 0.
    InvalidSnapshotPeriod = 21,
}

/// Events emitted by the fees contract.
pub struct FeeEvents;

impl FeeEvents {
    pub fn fee_deducted(env: &Env, payer: &Address, amount: i128, fee: i128) {
        let topics = (symbol_short!("fee"), symbol_short!("deducted"));
        env.events().publish(
            topics,
            (payer.clone(), amount, fee, env.ledger().timestamp()),
        );
    }

    pub fn config_updated(env: &Env, admin: &Address, percentage_bps: u32) {
        let topics = (symbol_short!("fee"), symbol_short!("cfg_upd"));
        env.events().publish(
            topics,
            (admin.clone(), percentage_bps, env.ledger().timestamp()),
        );
    }

    pub fn fee_refunded(env: &Env, user: &Address, refund_amount: i128, reason: &String) {
        let topics = (symbol_short!("fee"), symbol_short!("refunded"));
        env.events().publish(
            topics,
            (
                user.clone(),
                refund_amount,
                reason.clone(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn distribution_configured(env: &Env, admin: &Address, recipient_count: u32) {
        let topics = (symbol_short!("fee"), symbol_short!("dist_cfg"));
        env.events().publish(
            topics,
            (admin.clone(), recipient_count, env.ledger().timestamp()),
        );
    }

    pub fn fees_distributed(env: &Env, total_distributed: i128, recipient_count: u32) {
        let topics = (symbol_short!("fee"), symbol_short!("dist"));
        env.events().publish(
            topics,
            (total_distributed, recipient_count, env.ledger().timestamp()),
        );
    }

    pub fn fee_bounds_configured(env: &Env, admin: &Address, min_fee: i128, max_fee: i128) {
        let topics = (symbol_short!("fee"), symbol_short!("bnd_cfg"));
        env.events().publish(
            topics,
            (admin.clone(), min_fee, max_fee, env.ledger().timestamp()),
        );
    }

    pub fn priority_config_updated(
        env: &Env,
        admin: &Address,
        low_bps: u32,
        medium_bps: u32,
        high_bps: u32,
        urgent_bps: u32,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("pri_cfg"));
        env.events().publish(
            topics,
            (
                admin.clone(),
                low_bps,
                medium_bps,
                high_bps,
                urgent_bps,
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn fee_deducted_with_priority(
        env: &Env,
        payer: &Address,
        amount: i128,
        fee: i128,
        priority: PriorityLevel,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("ded_pri"));
        env.events().publish(
            topics,
            (
                payer.clone(),
                amount,
                fee,
                priority.to_u32(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn fee_escrowed(env: &Env, user: &Address, amount: i128) {
        let topics = (symbol_short!("fee"), symbol_short!("escrowed"));
        env.events()
            .publish(topics, (user.clone(), amount, env.ledger().timestamp()));
    }

    pub fn fee_delegate_updated(env: &Env, user: &Address, delegate: &Address) {
        let topics = (symbol_short!("fee"), symbol_short!("del_upd"));
        env.events().publish(
            topics,
            (user.clone(), delegate.clone(), env.ledger().timestamp()),
        );
    }

    pub fn treasury_configured(
        env: &Env,
        admin: &Address,
        treasury: &Address,
        percentage_bps: u32,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("treasury"));
        env.events().publish(
            topics,
            (
                admin.clone(),
                treasury.clone(),
                percentage_bps,
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn fees_routed_to_treasury(env: &Env, amount: i128, treasury: &Address) {
        let topics = (symbol_short!("fee"), symbol_short!("to_treas"));
        env.events()
            .publish(topics, (amount, treasury.clone(), env.ledger().timestamp()));
    }

    pub fn rounding_mode_updated(env: &Env, admin: &Address, mode: RoundingMode) {
        let topics = (symbol_short!("fee"), symbol_short!("rounding"));
        env.events().publish(
            topics,
            (admin.clone(), mode.to_u32(), env.ledger().timestamp()),
        );
    }

    pub fn fee_category_configured(env: &Env, user: &Address, category: FeeCategory) {
        let topics = (symbol_short!("fee"), symbol_short!("category"));
        env.events().publish(
            topics,
            (user.clone(), category.to_u32(), env.ledger().timestamp()),
        );
    }

    pub fn category_fees_updated(env: &Env, category: FeeCategory, amount: i128) {
        let topics = (symbol_short!("fee"), symbol_short!("cat_fee"));
        env.events().publish(
            topics,
            (category.to_u32(), amount, env.ledger().timestamp()),
        );
    }

    pub fn snapshot_created(
        env: &Env,
        period_start: u64,
        total_collected: i128,
        treasury_collected: i128,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("snapshot"));
        env.events().publish(
            topics,
            (
                period_start,
                total_collected,
                treasury_collected,
                env.ledger().timestamp(),
            ),
        );
    }
}

/// Internal helpers — not exposed as contract entry points.
impl FeesContract {
    fn require_initialized(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, FeeError::NotInitialized))
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin = Self::require_initialized(env);
        if caller != &admin {
            panic_with_error!(env, FeeError::Unauthorized);
        }
    }

    /// Apply rounding mode to a fee calculation
    fn apply_rounding(fee: i128, mode: RoundingMode) -> i128 {
        match mode {
            RoundingMode::Floor => fee,
            RoundingMode::Round => {
                // For positive values, standard rounding: add 0.5 and floor
                // Since we work with integers, we check if there's a remainder >= 5000
                // (which represents 0.5 in basis points context)
                fee
            }
            RoundingMode::Ceiling => {
                // If there's any remainder, round up
                fee
            }
        }
    }

    /// Get the current rounding mode (defaults to Round)
    fn load_rounding_mode(env: &Env) -> RoundingMode {
        env.storage()
            .instance()
            .get(&DataKey::RoundingMode)
            .unwrap_or(RoundingMode::Round)
    }

    /// Calculate treasury portion of a fee
    fn calculate_treasury_portion(env: &Env, total_fee: i128) -> (i128, i128) {
        let treasury_pct: u32 = env
            .storage()
            .instance()
            .get(&DataKey::TreasuryFeePercentage)
            .unwrap_or(0);

        if treasury_pct == 0 {
            return (0, total_fee);
        }

        let treasury_amount = total_fee
            .checked_mul(treasury_pct as i128)
            .unwrap_or_else(|| panic_with_error!(env, FeeError::Overflow))
            .checked_div(10_000)
            .unwrap_or_else(|| panic_with_error!(env, FeeError::Overflow));

        let remaining = total_fee
            .checked_sub(treasury_amount)
            .unwrap_or_else(|| panic_with_error!(env, FeeError::Overflow));

        (treasury_amount, remaining)
    }

    /// Get the treasury address
    fn get_treasury_address(env: &Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::TreasuryAddress)
    }

    /// Get fee category for a user (defaults to Transaction)
    fn get_user_category(env: &Env, user: &Address) -> FeeCategory {
        env.storage()
            .instance()
            .get(&DataKey::FeeCategoryMap(user.clone()))
            .unwrap_or(FeeCategory::Transaction)
    }

    /// Get cumulative fees for a category
    fn load_category_fees(env: &Env, category: FeeCategory) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::CategoryFees(category))
            .unwrap_or(0)
    }

    /// Update cumulative fees for a category
    fn update_category_fees(env: &Env, category: FeeCategory, amount: i128) {
        let current = Self::load_category_fees(env, category);
        let updated = current
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(env, FeeError::Overflow));
        env.storage()
            .instance()
            .set(&DataKey::CategoryFees(category), &updated);
    }
}

#[contract]
pub struct FeesContract;

#[contractimpl]
impl FeesContract {
    /// Initializes the fees contract with an admin and an initial percentage
    /// (in basis points, 0–10_000). Only callable once.
    ///
    /// # Security
    /// - Guard: `AlreadyInitialized` prevents re-initialization attacks.
    /// - `percentage_bps` is validated ≤ 10_000 before any state is written.
    pub fn initialize(env: Env, admin: Address, percentage_bps: u32) {
        // [SEC-FEES-01] Re-initialization guard: must be checked before any writes.
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, FeeError::AlreadyInitialized);
        }
        // [SEC-FEES-02] Validate percentage before committing state.
        if percentage_bps > 10_000 {
            panic_with_error!(&env, FeeError::InvalidPercentage);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::FeePercentage, &percentage_bps);
        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &0i128);
    }

    /// Updates the fee percentage. Only the current admin may call.
    ///
    /// # Security
    /// - [SEC-FEES-03] `caller.require_auth()` is invoked *before* any storage
    ///   reads so the host can short-circuit unauthorized calls cheaply.
    /// - Admin check uses the centralized `require_admin` helper to avoid
    ///   inconsistent comparisons across call sites.
    pub fn set_percentage(env: Env, caller: Address, percentage_bps: u32) {
        // [SEC-FEES-03] Authenticate before reading sensitive state.
        caller.require_auth();
        Self::require_admin(&env, &caller);

        if percentage_bps > 10_000 {
            panic_with_error!(&env, FeeError::InvalidPercentage);
        }
        env.storage()
            .instance()
            .set(&DataKey::FeePercentage, &percentage_bps);
        FeeEvents::config_updated(&env, &caller, percentage_bps);
    }

    /// Returns the current fee percentage in basis points.
    /// Defaults to 0 when the contract has not yet been initialized.
    pub fn get_percentage(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::FeePercentage)
            .unwrap_or(0)
    }

    /// Calculates the fee for `amount` using the current percentage.
    ///
    /// Applies min/max fee bounds if configured. The final fee will be:
    /// - At least min_fee (if configured)
    /// - At most max_fee (if configured)
    /// - Otherwise, fee_percentage * amount / 10000
    ///
    /// Uses the configured rounding mode for fee calculations.
    ///
    /// # Security
    /// - [SEC-FEES-04] Rejects non-positive amounts to prevent zero-fee bypass.
    /// - [SEC-FEES-05] All arithmetic uses `checked_*` to trap overflow/underflow
    ///   and panics with the typed `Overflow` error instead of silent wrap.
    /// - [SEC-FEES-18] Min/max fee bounds are applied to prevent unbounded fees
    ///   and ensure fees stay within configured ranges.
    pub fn calculate_fee(env: Env, amount: i128) -> i128 {
        // [SEC-FEES-04] Reject non-positive amounts.
        if amount <= 0 {
            panic_with_error!(&env, FeeError::InvalidAmount);
        }
        let pct: u32 = Self::get_percentage(env.clone());

        // Get rounding mode
        let rounding_mode = Self::load_rounding_mode(&env);

        // [SEC-FEES-05] Checked arithmetic throughout.
        let raw_fee = amount
            .checked_mul(pct as i128)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        // Apply rounding mode
        let fee = match rounding_mode {
            RoundingMode::Floor => raw_fee / 10_000,
            RoundingMode::Round => {
                // Add 5000 (0.5 in basis points) before dividing for round half up
                (raw_fee + 5_000) / 10_000
            }
            RoundingMode::Ceiling => {
                // If there's any remainder, round up
                if raw_fee % 10_000 == 0 {
                    raw_fee / 10_000
                } else {
                    raw_fee / 10_000 + 1
                }
            }
        };

        // [SEC-FEES-18] Apply min/max fee bounds.
        let min_fee: i128 = env.storage().instance().get(&DataKey::MinFee).unwrap_or(0);
        let max_fee: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MaxFee)
            .unwrap_or(i128::MAX);

        if fee < min_fee {
            return min_fee;
        }
        if fee > max_fee {
            return max_fee;
        }

        fee
    }

    /// Calculates the fee for `amount` using the current percentage and priority level.
    ///
    /// Priority multipliers adjust the base fee rate:
    /// - Low priority: discounted fee (e.g., 80% of base)
    /// - Medium priority: base fee (100%)
    /// - High priority: premium fee (e.g., 150% of base)
    /// - Urgent priority: highest fee (e.g., 200% of base)
    ///
    /// Applies min/max fee bounds if configured.
    /// Uses the configured rounding mode for fee calculations.
    ///
    /// # Security
    /// - [SEC-FEES-21] Priority configuration must be valid (ascending multipliers).
    /// - [SEC-FEES-22] All arithmetic uses `checked_*` to prevent overflow.
    pub fn calculate_fee_with_priority(env: Env, amount: i128, priority: PriorityLevel) -> i128 {
        // [SEC-FEES-04] Reject non-positive amounts.
        if amount <= 0 {
            panic_with_error!(&env, FeeError::InvalidAmount);
        }

        let base_pct: u32 = Self::get_percentage(env.clone());
        let priority_config: PriorityFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::PriorityFeeConfig)
            .unwrap_or_else(PriorityFeeConfig::default);

        // Get the multiplier for the priority level
        let multiplier_bps = priority_config.get_multiplier_bps(priority);

        // Calculate adjusted fee rate: base_pct * multiplier / 10000
        // This gives us the effective fee rate for the priority level
        let adjusted_pct = (base_pct as u64 * multiplier_bps as u64 / 10_000) as u32;

        // Get rounding mode
        let rounding_mode = Self::load_rounding_mode(&env);

        // [SEC-FEES-05] Checked arithmetic throughout.
        let raw_fee = amount
            .checked_mul(adjusted_pct as i128)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        // Apply rounding mode
        let fee = match rounding_mode {
            RoundingMode::Floor => raw_fee / 10_000,
            RoundingMode::Round => {
                // Add 5000 (0.5 in basis points) before dividing for round half up
                (raw_fee + 5_000) / 10_000
            }
            RoundingMode::Ceiling => {
                // If there's any remainder, round up
                if raw_fee % 10_000 == 0 {
                    raw_fee / 10_000
                } else {
                    raw_fee / 10_000 + 1
                }
            }
        };

        // [SEC-FEES-18] Apply min/max fee bounds.
        let min_fee: i128 = env.storage().instance().get(&DataKey::MinFee).unwrap_or(0);
        let max_fee: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MaxFee)
            .unwrap_or(i128::MAX);

        if fee < min_fee {
            return min_fee;
        }
        if fee > max_fee {
            return max_fee;
        }

        fee
    }

    /// Deducts the configured fee from `amount` with a specified priority level.
    ///
    /// Returns `(net_amount, fee)` and updates the cumulative accounting.
    /// Routes portion to treasury if configured and tracks by category.
    ///
    /// # Security
    /// - [SEC-FEES-06] `payer.require_auth()` is invoked first — no state
    ///   mutations can occur without authorization.
    /// - [SEC-FEES-07] `TotalFeesCollected` accumulation uses `checked_add` so
    ///   a saturated counter triggers `Overflow` rather than wrapping silently.
    /// - Requires the contract to be initialized; `calculate_fee` propagates
    ///   `NotInitialized` via `get_percentage` if called before `initialize`.
    /// - [SEC-FEES-08] Per-user fee tracking is updated with `checked_add` to
    ///   prevent overflow on per-user accumulation.
    pub fn deduct_fee(env: Env, payer: Address, amount: i128) -> (i128, i128) {
        // [SEC-FEES-06] Authenticate before any computation or state change.
        payer.require_auth();

        // Ensure contract is initialized before proceeding.
        Self::require_initialized(&env);

        let fee = Self::calculate_fee(env.clone(), amount);

        // [SEC-FEES-07] Checked subtraction for net amount.
        let net = amount
            .checked_sub(fee)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        // [SEC-FEES-21] Delegation logic: use the configured delegate if it exists.
        let actual_payer = if let Some(delegate) = env
            .storage()
            .instance()
            .get(&DataKey::FeeDelegate(payer.clone()))
        {
            delegate
        } else {
            payer.clone()
        };

        if actual_payer != payer {
            actual_payer.require_auth();
        }

        // Calculate treasury portion and remaining
        let (treasury_amount, remaining) = Self::calculate_treasury_portion(&env, fee);

        // Update treasury total if configured
        if treasury_amount > 0 {
            let mut treasury_total: i128 = env
                .storage()
                .instance()
                .get(&DataKey::TotalTreasuryFees)
                .unwrap_or(0);
            treasury_total = treasury_total
                .checked_add(treasury_amount)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));
            env.storage()
                .instance()
                .set(&DataKey::TotalTreasuryFees, &treasury_total);
        }

        // Track fees by category for the user
        let category = Self::get_user_category(&env, &payer);
        Self::update_category_fees(&env, category, fee);

        let mut total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0);

        // [SEC-FEES-07] Checked addition for running total.
        total = total
            .checked_add(remaining)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &total);

        // [SEC-FEES-08] Update per-user fee accrual tracking.
        let mut user_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::UserFeesAccrued(payer.clone()))
            .unwrap_or(0);

        // [SEC-FEES-08] Checked addition for per-user total.
        user_fees = user_fees
            .checked_add(fee)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::UserFeesAccrued(payer.clone()), &user_fees);

        FeeEvents::fee_deducted(&env, &payer, amount, fee);
        if treasury_amount > 0 {
            if let Some(treasury) = Self::get_treasury_address(&env) {
                FeeEvents::fees_routed_to_treasury(&env, treasury_amount, &treasury);
                let client = TreasuryContractClient::new(&env, &treasury);
                client.credit_fee(&treasury_amount);
            }
        }
        (net, fee)
    }

    /// Deducts the configured fee from `amount` with a specified priority level.
    ///
    /// Returns `(net_amount, fee)` and updates the cumulative accounting.
    /// Higher priority levels result in higher fees for faster execution.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `payer` - The address paying the fee
    /// * `amount` - The amount to calculate fees on
    /// * `priority` - The priority level (Low, Medium, High, Urgent)
    ///
    /// # Returns
    /// Tuple of (net_amount, fee_charged)
    ///
    /// # Security
    /// - [SEC-FEES-23] Same security guarantees as `deduct_fee`.
    /// - [SEC-FEES-24] Priority level is used to adjust fee via configured multipliers.
    pub fn deduct_fee_with_priority(
        env: Env,
        payer: Address,
        amount: i128,
        priority: PriorityLevel,
    ) -> (i128, i128) {
        // [SEC-FEES-06] Authenticate before any computation or state change.
        payer.require_auth();

        // Ensure contract is initialized before proceeding.
        Self::require_initialized(&env);

        let fee = Self::calculate_fee_with_priority(env.clone(), amount, priority);

        // [SEC-FEES-07] Checked subtraction for net amount.
        let net = amount
            .checked_sub(fee)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        // Calculate treasury portion and remaining
        let (treasury_amount, remaining) = Self::calculate_treasury_portion(&env, fee);

        // Update treasury total if configured
        if treasury_amount > 0 {
            let mut treasury_total: i128 = env
                .storage()
                .instance()
                .get(&DataKey::TotalTreasuryFees)
                .unwrap_or(0);
            treasury_total = treasury_total
                .checked_add(treasury_amount)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));
            env.storage()
                .instance()
                .set(&DataKey::TotalTreasuryFees, &treasury_total);
        }

        // Track fees by category for the user
        let category = Self::get_user_category(&env, &payer);
        Self::update_category_fees(&env, category, fee);

        let mut total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0);

        // [SEC-FEES-07] Checked addition for running total.
        total = total
            .checked_add(remaining)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &total);

        // [SEC-FEES-08] Update per-user fee accrual tracking.
        let mut user_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::UserFeesAccrued(payer.clone()))
            .unwrap_or(0);

        // [SEC-FEES-08] Checked addition for per-user total.
        user_fees = user_fees
            .checked_add(fee)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::UserFeesAccrued(payer.clone()), &user_fees);

        FeeEvents::fee_deducted_with_priority(&env, &payer, amount, fee, priority);
        if treasury_amount > 0 {
            if let Some(treasury) = Self::get_treasury_address(&env) {
                FeeEvents::fees_routed_to_treasury(&env, treasury_amount, &treasury);
                let client = TreasuryContractClient::new(&env, &treasury);
                client.credit_fee(&treasury_amount);
            }
        }
        (net, fee)
    }

    /// [ISSUE-206] Deduct fee and hold it in escrow.
    pub fn deduct_fee_to_escrow(env: Env, payer: Address, amount: i128) -> (i128, i128) {
        payer.require_auth();
        Self::require_initialized(&env);

        let fee = Self::calculate_fee(env.clone(), amount);
        let net = amount.checked_sub(fee).expect("Overflow");

        let mut escrowed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowedFees(payer.clone()))
            .unwrap_or(0);
        escrowed = escrowed.checked_add(fee).expect("Overflow");

        env.storage()
            .instance()
            .set(&DataKey::EscrowedFees(payer.clone()), &escrowed);
        FeeEvents::fee_escrowed(&env, &payer, fee);

        (net, fee)
    }

    /// [ISSUE-206] Release escrowed fees to global collection.
    pub fn release_escrow(env: Env, admin: Address, user: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        let escrowed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowedFees(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::NoEscrowedFees));

        if escrowed == 0 {
            panic_with_error!(&env, FeeError::NoEscrowedFees);
        }

        let mut total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0);
        total = total.checked_add(escrowed).expect("Overflow");

        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &total);
        env.storage()
            .instance()
            .remove(&DataKey::EscrowedFees(user));
    }

    /// [ISSUE-203] Set a delegate for fee payments.
    pub fn set_fee_delegate(env: Env, user: Address, delegate: Address) {
        user.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::FeeDelegate(user.clone()), &delegate);
        FeeEvents::fee_delegate_updated(&env, &user, &delegate);
    }

    /// Configure treasury address and percentage for fee routing.
    ///
    /// Routes a portion of collected fees to the treasury address.
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `caller` - The admin address
    /// * `treasury` - The treasury address to receive fees
    /// * `treasury_percentage_bps` - Percentage of fees to route to treasury (0-10000)
    ///
    /// # Security
    /// - Admin authentication required
    /// - Treasury percentage must be 0-10000
    pub fn set_treasury(
        env: Env,
        caller: Address,
        treasury: Address,
        treasury_percentage_bps: u32,
    ) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        if treasury_percentage_bps > 10_000 {
            panic_with_error!(&env, FeeError::InvalidTreasuryPercentage);
        }

        env.storage()
            .instance()
            .set(&DataKey::TreasuryAddress, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::TreasuryFeePercentage, &treasury_percentage_bps);
        FeeEvents::treasury_configured(&env, &caller, &treasury, treasury_percentage_bps);
    }

    /// Returns the treasury address if configured.
    pub fn get_treasury(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::TreasuryAddress)
    }

    /// Returns the treasury fee percentage.
    pub fn get_treasury_percentage(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::TreasuryFeePercentage)
            .unwrap_or(0)
    }

    /// Returns total fees routed to treasury.
    pub fn get_total_treasury_fees(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalTreasuryFees)
            .unwrap_or(0)
    }

    /// Set rounding mode for fee calculations.
    ///
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `caller` - The admin address
    /// * `mode` - The rounding mode (Floor, Round, or Ceiling)
    pub fn set_rounding_mode(env: Env, caller: Address, mode: RoundingMode) {
        caller.require_auth();
        Self::require_admin(&env, &caller);
        env.storage().instance().set(&DataKey::RoundingMode, &mode);
        FeeEvents::rounding_mode_updated(&env, &caller, mode);
    }

    /// Get the current rounding mode.
    pub fn get_rounding_mode(env: Env) -> RoundingMode {
        Self::load_rounding_mode(&env)
    }

    /// Set fee category for a user for tracking purposes.
    ///
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `caller` - The admin address
    /// * `user` - The user address
    /// * `category` - The fee category
    pub fn set_user_fee_category(env: Env, caller: Address, user: Address, category: FeeCategory) {
        caller.require_auth();
        Self::require_admin(&env, &caller);
        env.storage()
            .instance()
            .set(&DataKey::FeeCategoryMap(user.clone()), &category);
        FeeEvents::fee_category_configured(&env, &user, category);
    }

    /// Get fee category for a user.
    pub fn get_user_fee_category(env: Env, user: Address) -> FeeCategory {
        Self::get_user_category(&env, &user)
    }

    /// Get cumulative fees for a specific category.
    pub fn get_category_fees(env: Env, category: FeeCategory) -> i128 {
        Self::load_category_fees(&env, category)
    }

    /// Create a snapshot of current fee state.
    ///
    /// Captures the current totals for reporting and historical tracking.
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `caller` - The admin address
    /// * `period_start` - Start timestamp of the period being snapshotted
    pub fn create_fee_snapshot(env: Env, caller: Address, period_start: u64) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        if period_start == 0 {
            panic_with_error!(&env, FeeError::InvalidSnapshotPeriod);
        }

        let now = env.ledger().timestamp();
        let total_collected = Self::get_total_collected(env.clone());
        let treasury_collected = Self::get_total_treasury_fees(env.clone());

        let snapshot = FeeSnapshot {
            total_collected,
            treasury_collected,
            distributed: total_collected - treasury_collected, // Approximate - remaining goes to distribution
            created_at: now,
            period_start,
        };

        env.storage()
            .persistent()
            .set(&DataKey::FeeSnapshot(period_start), &snapshot);

        // Update metadata
        let mut metadata: SnapshotMetadata = env
            .storage()
            .instance()
            .get(&DataKey::SnapshotMetadata)
            .unwrap_or(SnapshotMetadata {
                count: 0,
                latest_timestamp: 0,
            });
        metadata.count = metadata.count.saturating_add(1);
        metadata.latest_timestamp = now;
        env.storage()
            .instance()
            .set(&DataKey::SnapshotMetadata, &metadata);

        FeeEvents::snapshot_created(&env, period_start, total_collected, treasury_collected);
    }

    /// Get a fee snapshot for a specific period.
    pub fn get_fee_snapshot(env: Env, period_start: u64) -> Option<FeeSnapshot> {
        env.storage()
            .persistent()
            .get(&DataKey::FeeSnapshot(period_start))
    }

    /// Get snapshot metadata.
    pub fn get_snapshot_metadata(env: Env) -> SnapshotMetadata {
        env.storage()
            .instance()
            .get(&DataKey::SnapshotMetadata)
            .unwrap_or(SnapshotMetadata {
                count: 0,
                latest_timestamp: 0,
            })
    }

    /// Returns cumulative fees collected since deployment.
    pub fn get_total_collected(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0)
    }

    /// Returns the total fees accrued by a specific user.
    ///
    /// Returns 0 if the user has not accrued any fees yet.
    ///
    /// # Arguments
    /// * `user` - The address of the user to query
    ///
    /// # Returns
    /// Total fees paid by the user in stroops (smallest unit)
    pub fn get_user_fees_accrued(env: Env, user: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::UserFeesAccrued(user))
            .unwrap_or(0)
    }

    /// Refunds fees for a specific user.
    ///
    /// Only the admin can invoke this function. Validates that the refund amount
    /// does not exceed the user's accumulated fees. Updates both global and per-user
    /// fee balances.
    ///
    /// # Arguments
    /// * `caller` - The address requesting the refund (must be admin)
    /// * `user` - The user to whom fees are refunded
    /// * `refund_amount` - The amount to refund (must be positive)
    /// * `reason` - The reason for the refund (for audit trail)
    ///
    /// # Returns
    /// The refunded amount
    ///
    /// # Security
    /// - [SEC-FEES-09] `caller.require_auth()` is invoked first — admin-only refunds.
    /// - [SEC-FEES-10] `require_admin()` ensures only authorized admins can process
    ///   refunds, preventing unauthorized fee adjustments.
    /// - [SEC-FEES-11] Refund amount is validated as positive before any state mutation.
    /// - [SEC-FEES-12] User's fee balance is checked before refund — prevents negative
    ///   fee balances which would enable fee credit abuse.
    /// - [SEC-FEES-13] Checked arithmetic (`checked_sub`) prevents underflow when
    ///   reducing fee totals.
    pub fn refund_fee(
        env: Env,
        caller: Address,
        user: Address,
        refund_amount: i128,
        reason: String,
    ) -> i128 {
        // [SEC-FEES-09] Authenticate before any computation or state change.
        caller.require_auth();

        // [SEC-FEES-10] Only admin can process refunds.
        Self::require_admin(&env, &caller);

        // [SEC-FEES-11] Validate refund amount is positive.
        if refund_amount <= 0 {
            panic_with_error!(&env, FeeError::InvalidRefundAmount);
        }

        // Ensure contract is initialized before proceeding.
        Self::require_initialized(&env);

        // [SEC-FEES-12] Check user has sufficient fee balance.
        let user_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::UserFeesAccrued(user.clone()))
            .unwrap_or(0);

        if user_fees < refund_amount {
            panic_with_error!(&env, FeeError::InsufficientFeeBalance);
        }

        // [SEC-FEES-13] Deduct from global fee total using checked subtraction.
        let mut total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0);

        total = total
            .checked_sub(refund_amount)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &total);

        // [SEC-FEES-13] Deduct from per-user fee balance using checked subtraction.
        let updated_user_fees = user_fees
            .checked_sub(refund_amount)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

        env.storage()
            .instance()
            .set(&DataKey::UserFeesAccrued(user.clone()), &updated_user_fees);

        FeeEvents::fee_refunded(&env, &user, refund_amount, &reason);
        refund_amount
    }

    /// Sets the fee distribution configuration.
    ///
    /// Defines which recipients receive distributed fees and their respective shares.
    /// Only callable by the admin. Validates that:
    /// - Distribution is not empty
    /// - Each recipient has a valid share (0–10_000 bps)
    /// - All shares sum exactly to 10_000 (100%)
    ///
    /// # Arguments
    /// * `caller` - The address requesting configuration (must be admin)
    /// * `recipients` - Vector of FeeRecipient with address and share_bps
    ///
    /// # Security
    /// - [SEC-FEES-14] `caller.require_auth()` ensures only authorized admins can
    ///   configure distributions.
    /// - [SEC-FEES-15] Comprehensive validation prevents invalid distributions:
    ///   empty lists, invalid shares, or sums != 100%.
    pub fn set_distribution(env: Env, caller: Address, recipients: Vec<FeeRecipient>) {
        // [SEC-FEES-14] Authenticate before any state mutation.
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // [SEC-FEES-15] Validate distribution is not empty.
        if recipients.len() == 0 {
            panic_with_error!(&env, FeeError::InvalidDistribution);
        }

        let mut total_bps: u32 = 0;
        for recipient in recipients.iter() {
            // [SEC-FEES-15] Validate each share is within valid range.
            if recipient.share_bps > 10_000 {
                panic_with_error!(&env, FeeError::InvalidDistribution);
            }
            // [SEC-FEES-15] Accumulate total and check for overflow.
            total_bps = total_bps
                .checked_add(recipient.share_bps)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));
        }

        // [SEC-FEES-15] Ensure total equals exactly 100% (10_000 bps).
        if total_bps != 10_000 {
            panic_with_error!(&env, FeeError::DistributionSumsToWrong);
        }

        env.storage()
            .instance()
            .set(&DataKey::FeeDistribution, &recipients);
        FeeEvents::distribution_configured(&env, &caller, recipients.len() as u32);
    }

    /// Returns the current fee distribution configuration.
    ///
    /// # Returns
    /// Vector of FeeRecipient, or empty vector if no distribution configured
    pub fn get_distribution(env: Env) -> Vec<FeeRecipient> {
        env.storage()
            .instance()
            .get(&DataKey::FeeDistribution)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Distributes accumulated fees to all configured recipients.
    ///
    /// Only callable by the admin. Requires that a valid distribution configuration
    /// has been set. Distributes fees according to each recipient's share percentage.
    ///
    /// # Returns
    /// Total amount distributed
    ///
    /// # Security
    /// - [SEC-FEES-14] `caller.require_auth()` ensures only authorized admins can
    ///   trigger distributions.
    /// - [SEC-FEES-16] Distribution must be configured before distribution can occur.
    /// - [SEC-FEES-17] All per-recipient distributions use checked arithmetic to
    ///   prevent overflow.
    pub fn distribute_fees(env: Env, caller: Address) -> i128 {
        // [SEC-FEES-14] Authenticate before any state mutation.
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // [SEC-FEES-16] Check distribution is configured.
        let recipients: Vec<FeeRecipient> = env
            .storage()
            .instance()
            .get(&DataKey::FeeDistribution)
            .unwrap_or_else(|| panic_with_error!(&env, FeeError::NoDistributionConfigured));

        if recipients.len() == 0 {
            panic_with_error!(&env, FeeError::NoDistributionConfigured);
        }

        // Get current total fees to distribute
        let total_to_distribute: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalFeesCollected)
            .unwrap_or(0);

        // If no fees to distribute, return early
        if total_to_distribute <= 0 {
            return 0;
        }

        let mut total_distributed: i128 = 0;

        // Distribute to each recipient according to their share
        for recipient in recipients.iter() {
            // [SEC-FEES-17] Calculate recipient's share using checked arithmetic.
            let recipient_share: i128 = total_to_distribute
                .checked_mul(recipient.share_bps as i128)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow))
                .checked_div(10_000)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

            // [SEC-FEES-17] Accumulate recipient's fees.
            let mut recipient_fees: i128 = env
                .storage()
                .instance()
                .get(&DataKey::RecipientFeesAccumulated(
                    recipient.address.clone(),
                ))
                .unwrap_or(0);

            recipient_fees = recipient_fees
                .checked_add(recipient_share)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));

            env.storage().instance().set(
                &DataKey::RecipientFeesAccumulated(recipient.address.clone()),
                &recipient_fees,
            );

            total_distributed = total_distributed
                .checked_add(recipient_share)
                .unwrap_or_else(|| panic_with_error!(&env, FeeError::Overflow));
        }

        // Reset total fees collected after distribution
        env.storage()
            .instance()
            .set(&DataKey::TotalFeesCollected, &0i128);

        FeeEvents::fees_distributed(&env, total_distributed, recipients.len() as u32);
        total_distributed
    }

    /// Returns the cumulative fees accumulated by a specific recipient.
    ///
    /// # Arguments
    /// * `recipient` - The recipient address to query
    ///
    /// # Returns
    /// Total fees accumulated for the recipient
    pub fn get_recipient_fees_accumulated(env: Env, recipient: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::RecipientFeesAccumulated(recipient))
            .unwrap_or(0)
    }

    /// Sets the minimum and maximum fee bounds.
    ///
    /// Fees calculated from percentage will be bounded to stay within [min_fee, max_fee].
    /// Only callable by the admin. Validates that:
    /// - Both bounds are non-negative
    /// - max_fee is >= min_fee
    ///
    /// # Arguments
    /// * `caller` - The address requesting configuration (must be admin)
    /// * `min_fee` - Minimum fee threshold (must be >= 0)
    /// * `max_fee` - Maximum fee threshold (must be >= min_fee)
    ///
    /// # Security
    /// - [SEC-FEES-19] `caller.require_auth()` ensures only authorized admins can
    ///   configure fee bounds.
    /// - [SEC-FEES-20] Comprehensive validation prevents invalid bounds:
    ///   negative values or inverted ranges.
    pub fn set_fee_bounds(env: Env, caller: Address, min_fee: i128, max_fee: i128) {
        // [SEC-FEES-19] Authenticate before any state mutation.
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // [SEC-FEES-20] Validate both bounds are non-negative.
        if min_fee < 0 || max_fee < 0 {
            panic_with_error!(&env, FeeError::InvalidFeeBound);
        }

        // [SEC-FEES-20] Validate max >= min.
        if max_fee < min_fee {
            panic_with_error!(&env, FeeError::InvalidFeeBoundRange);
        }

        env.storage().instance().set(&DataKey::MinFee, &min_fee);
        env.storage().instance().set(&DataKey::MaxFee, &max_fee);
        FeeEvents::fee_bounds_configured(&env, &caller, min_fee, max_fee);
    }

    /// Returns the minimum fee threshold.
    ///
    /// # Returns
    /// Minimum fee in stroops, or 0 if not configured
    pub fn get_min_fee(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::MinFee).unwrap_or(0)
    }

    /// Returns the maximum fee threshold.
    ///
    /// # Returns
    /// Maximum fee in stroops, or i128::MAX if not configured
    pub fn get_max_fee(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::MaxFee)
            .unwrap_or(i128::MAX)
    }

    // =========================================================================
    // Priority Fee Configuration
    // =========================================================================

    /// Sets the priority fee multipliers.
    ///
    /// Priority multipliers determine how fees are adjusted based on transaction priority.
    /// Higher priority levels should have higher multipliers for faster execution.
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `caller` - The address requesting configuration (must be admin)
    /// * `low_multiplier_bps` - Multiplier for Low priority (e.g., 8000 = 0.8x)
    /// * `medium_multiplier_bps` - Multiplier for Medium priority (e.g., 10000 = 1.0x)
    /// * `high_multiplier_bps` - Multiplier for High priority (e.g., 15000 = 1.5x)
    /// * `urgent_multiplier_bps` - Multiplier for Urgent priority (e.g., 20000 = 2.0x)
    ///
    /// # Security
    /// - [SEC-FEES-25] `caller.require_auth()` ensures only authorized admins can configure.
    /// - [SEC-FEES-26] Multipliers must be in ascending order (low <= medium <= high <= urgent).
    pub fn set_priority_multipliers(
        env: Env,
        caller: Address,
        low_multiplier_bps: u32,
        medium_multiplier_bps: u32,
        high_multiplier_bps: u32,
        urgent_multiplier_bps: u32,
    ) {
        // [SEC-FEES-25] Authenticate before any state mutation.
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let config = PriorityFeeConfig {
            low_multiplier_bps,
            medium_multiplier_bps,
            high_multiplier_bps,
            urgent_multiplier_bps,
        };

        // [SEC-FEES-26] Validate multipliers are in ascending order.
        if !config.is_valid() {
            panic_with_error!(&env, FeeError::InvalidPriorityConfig);
        }

        env.storage()
            .instance()
            .set(&DataKey::PriorityFeeConfig, &config);

        FeeEvents::priority_config_updated(
            &env,
            &caller,
            low_multiplier_bps,
            medium_multiplier_bps,
            high_multiplier_bps,
            urgent_multiplier_bps,
        );
    }

    /// Returns the current priority fee configuration.
    ///
    /// # Returns
    /// PriorityFeeConfig with multipliers for each priority level
    pub fn get_priority_config(env: Env) -> PriorityFeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::PriorityFeeConfig)
            .unwrap_or_else(PriorityFeeConfig::default)
    }

    /// Returns the multiplier for a specific priority level.
    ///
    /// # Arguments
    /// * `priority` - The priority level to query
    ///
    /// # Returns
    /// Multiplier in basis points (e.g., 15000 = 1.5x)
    pub fn get_priority_multiplier(env: Env, priority: PriorityLevel) -> u32 {
        let config = Self::get_priority_config(env.clone());
        config.get_multiplier_bps(priority)
    }
}
