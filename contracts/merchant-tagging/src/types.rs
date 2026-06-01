//! Types for the merchant-tagging contract.

use soroban_sdk::{contracttype, Address, String, Symbol};

/// Maximum length of a merchant name (characters).
pub const MAX_MERCHANT_NAME_LEN: u32 = 64;

/// Maximum length of a merchant category tag (characters).
pub const MAX_TAG_LEN: u32 = 32;

/// Maximum number of tags per merchant.
pub const MAX_TAGS_PER_MERCHANT: u32 = 10;

/// Maximum number of transactions returned in a single query.
pub const MAX_QUERY_RESULTS: u32 = 100;

// ── Storage keys ──────────────────────────────────────────────────────────────

/// Storage key variants for the merchant-tagging contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract administrator.
    Admin,
    /// Merchant record keyed by merchant ID.
    Merchant(Symbol),
    /// List of all registered merchant IDs.
    MerchantIndex,
    /// Merchant tag attached to a specific transaction.
    /// Key: (transaction_id, merchant_id)
    TransactionMerchant(u64, Symbol),
    /// Index: all transaction IDs tagged with a given merchant.
    MerchantTransactions(Symbol),
    /// Aggregate spend analytics per merchant.
    MerchantAnalytics(Symbol),
    /// Running total of transactions processed.
    TotalTagged,
}

// ── Core types ────────────────────────────────────────────────────────────────

/// A registered merchant with metadata and category tags.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Merchant {
    /// Unique merchant identifier (short symbol, e.g. `symbol_short!("STARBUCKS")`).
    pub id: Symbol,
    /// Human-readable merchant name.
    pub name: String,
    /// Category tags (e.g. "food", "retail", "travel"). Max `MAX_TAGS_PER_MERCHANT`.
    pub tags: soroban_sdk::Vec<Symbol>,
    /// Address of the merchant account on Stellar (optional).
    pub address: Option<Address>,
    /// Ledger timestamp when the merchant was registered.
    pub registered_at: u64,
    /// Whether this merchant record is active.
    pub active: bool,
}

/// Metadata attached to a single transaction linking it to a merchant.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionMerchantTag {
    /// Transaction identifier.
    pub tx_id: u64,
    /// Merchant this transaction is attributed to.
    pub merchant_id: Symbol,
    /// Amount of the transaction in stroops.
    pub amount: i128,
    /// Asset code (e.g. "XLM", "USDC").
    pub asset: Symbol,
    /// Ledger timestamp of the transaction.
    pub timestamp: u64,
    /// Optional free-text note.
    pub note: String,
}

/// Aggregate analytics for a single merchant.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantAnalytics {
    /// Merchant identifier.
    pub merchant_id: Symbol,
    /// Total number of tagged transactions.
    pub tx_count: u32,
    /// Total spend volume in stroops.
    pub total_volume: i128,
    /// Ledger timestamp of the most recent transaction.
    pub last_tx_at: u64,
}
