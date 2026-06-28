use soroban_sdk::{contracttype, Address};

/// How often an allowance is distributed.
///
/// Seconds per period:
/// - `Daily`   → 24 × 60 × 60 = 86 400 s
/// - `Weekly`  → 7 × 24 × 60 × 60 = 604 800 s
/// - `Monthly` → 30 × 24 × 60 × 60 = 2 592 000 s
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frequency {
    /// Once-off — no automatic recurrence.
    Once,
    /// Repeats every 24 hours (86 400 seconds). Issue #832.
    Daily,
    /// Repeats every 7 days (604 800 seconds).
    Weekly,
    /// Repeats every 30 days (2 592 000 seconds).
    Monthly,
}

impl Frequency {
    /// Returns the interval in seconds, or `None` for `Once`.
    pub fn interval_seconds(&self) -> Option<u64> {
        match self {
            Frequency::Once => None,
            Frequency::Daily => Some(86_400),
            Frequency::Weekly => Some(604_800),
            Frequency::Monthly => Some(2_592_000),
        }
    }
}

/// A recurring (or one-time) spending allowance record.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Allowance {
    /// The address that created and funds the allowance.
    pub owner: Address,
    /// The address entitled to spend / claim the allowance.
    pub recipient: Address,
    /// Token contract address used for distributions.
    pub token: Address,
    /// Amount transferred on each distribution.
    pub amount: i128,
    /// Recurrence schedule.
    pub frequency: Frequency,
    /// Ledger timestamp of the next allowed distribution.
    pub next_distribution: u64,
    /// Total number of successful distributions so far.
    pub distribution_count: u64,
    /// Whether the allowance is still active.
    pub active: bool,
    /// Whether the allowance is temporarily paused (issue #833).
    pub paused: bool,
    /// Ledger timestamp after which the allowance expires and distributions
    /// stop automatically (issue #839). `0` means it never expires.
    pub end_date: u64,
}

/// Persistent storage keys for the allowances contract.
#[contracttype]
pub enum DataKey {
    /// Total number of allowances created (monotonic counter → unique IDs).
    AllowanceCount,
    /// Per-allowance record keyed by ID.
    Allowance(u64),
    /// Index: list of allowance IDs owned by an address.
    OwnerAllowances(Address),
    /// Index: list of allowance IDs a recipient is entitled to.
    RecipientAllowances(Address),
}

/// Error codes returned by the allowances contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AllowanceError {
    NotFound = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
    InvalidInterval = 4,
    AlreadyInactive = 5,
    TooEarlyToDistribute = 6,
    InsufficientBalance = 7,
    /// Allowance is already paused (#833)
    AlreadyPaused = 8,
    /// Allowance is not paused (#833)
    NotPaused = 9,
    /// Allowance is paused — distribution blocked (#833)
    Paused = 10,
    /// Allowance has passed its end date and is expired (#839)
    Expired = 11,
    /// Expiration timestamp must be in the future (or 0 to clear) (#839)
    InvalidExpiration = 12,
}

impl From<AllowanceError> for soroban_sdk::Error {
    fn from(e: AllowanceError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}
