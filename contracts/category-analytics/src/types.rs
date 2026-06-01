use soroban_sdk::{contracttype, Address, Symbol};

/// Spending metrics for a category
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CategorySpending {
    pub count: u32,
    pub volume: i128,
}

/// Spending entry used for multi-category batch aggregation
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CategorySpend {
    pub category: Symbol,
    pub amount: i128,
}

/// Historical analytics record for a user, category, and month
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonthlyAnalytics {
    pub user: Address,
    pub category: Symbol,
    pub year: u32,
    pub month: u32,
    pub volume: i128,
    pub count: u32,
    pub last_updated: u64,
}

/// Storage keys for the contract
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    // (year, month, user, category) -> MonthlyAnalytics
    MonthlyAnalytics(u32, u32, Address, Symbol),
    // (user, category) -> CategorySpending (current aggregations)
    CurrentSpending(Address, Symbol),
    // Total users tracked
    TotalTrackedUsers,
}
