#![no_std]
use soroban_sdk::{contract, contractimpl, log, Address, Env, Symbol, Vec};

pub mod events;
#[cfg(test)]
#[path = "../tests/integration.rs"]
mod integration;
#[cfg(test)]
mod test;
pub mod types;

use crate::events::emit_spending_updated;
use crate::types::{CategorySpend, CategorySpending, DataKey, MonthlyAnalytics};

#[contract]
pub struct CategoryAnalytics;

#[contractimpl]
impl CategoryAnalytics {
    /// Initializes the contract with an admin address
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Records spending for a user and category.
    /// Updates both current spending aggregations and monthly history.
    pub fn record_spending(env: Env, user: Address, category: Symbol, amount: i128) {
        user.require_auth();
        record_category_spending(&env, &user, category.clone(), amount);

        log!(
            &env,
            "recorded spending: user={}, category={}, amount={}",
            user,
            category,
            amount
        );
    }

    /// Records a batch of spending entries for a user across one or more categories.
    pub fn record_spending_batch(env: Env, user: Address, spendings: Vec<CategorySpend>) {
        user.require_auth();

        if spendings.len() == 0 {
            panic!("batch must not be empty");
        }

        for spending in spendings.iter() {
            record_category_spending(&env, &user, spending.category, spending.amount);
        }
    }

    /// Retrieves current aggregate spending for a user and category.
    pub fn get_current_spending(env: Env, user: Address, category: Symbol) -> CategorySpending {
        let key = DataKey::CurrentSpending(user, category);
        env.storage()
            .instance()
            .get(&key)
            .unwrap_or(CategorySpending {
                count: 0,
                volume: 0,
            })
    }

    /// Retrieves analytics for a user and category in a specific month
    pub fn get_category_metrics(
        env: Env,
        user: Address,
        category: Symbol,
        year: u32,
        month: u32,
    ) -> MonthlyAnalytics {
        let key = DataKey::MonthlyAnalytics(year, month, user.clone(), category.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(MonthlyAnalytics {
                user,
                category,
                year,
                month,
                volume: 0,
                count: 0,
                last_updated: 0,
            })
    }

    /// Aggregates yearly trend for a user and category
    pub fn get_yearly_trend(
        env: Env,
        user: Address,
        category: Symbol,
        year: u32,
    ) -> CategorySpending {
        let mut total_volume: i128 = 0;
        let mut total_count: u32 = 0;

        for month in 1..=12 {
            let analytics = Self::get_category_metrics(
                env.clone(),
                user.clone(),
                category.clone(),
                year,
                month,
            );
            total_volume = total_volume
                .checked_add(analytics.volume)
                .expect("volume overflow");
            total_count += analytics.count;
        }

        CategorySpending {
            count: total_count,
            volume: total_volume,
        }
    }
}

fn record_category_spending(env: &Env, user: &Address, category: Symbol, amount: i128) {
    if amount <= 0 {
        panic!("amount must be positive");
    }

    let current_key = DataKey::CurrentSpending(user.clone(), category.clone());
    let mut current = env
        .storage()
        .instance()
        .get(&current_key)
        .unwrap_or(CategorySpending {
            count: 0,
            volume: 0,
        });

    current.count += 1;
    current.volume = current.volume.checked_add(amount).expect("volume overflow");
    env.storage().instance().set(&current_key, &current);

    let ledger_timestamp = env.ledger().timestamp();
    let (year, month) = get_year_month(ledger_timestamp);

    let monthly_key = DataKey::MonthlyAnalytics(year, month, user.clone(), category.clone());
    let mut monthly = env
        .storage()
        .persistent()
        .get(&monthly_key)
        .unwrap_or(MonthlyAnalytics {
            user: user.clone(),
            category: category.clone(),
            year,
            month,
            volume: 0,
            count: 0,
            last_updated: ledger_timestamp,
        });

    monthly.volume = monthly.volume.checked_add(amount).expect("volume overflow");
    monthly.count += 1;
    monthly.last_updated = ledger_timestamp;

    env.storage().persistent().set(&monthly_key, &monthly);

    emit_spending_updated(env, user.clone(), category, amount);
}

/// Helper function to estimate year and month from timestamp
/// Note: This is a simplified calculation for simulation
fn get_year_month(timestamp: u64) -> (u32, u32) {
    let _seconds_in_day = 86400;
    let seconds_in_year = 31536000;
    let seconds_in_month = 2592000; // Average month (30 days)

    let year = 1970 + (timestamp / seconds_in_year) as u32;
    let month = 1 + ((timestamp % seconds_in_year) / seconds_in_month) as u32;

    (year, month % 13) // Ensure month is 1-12
}
