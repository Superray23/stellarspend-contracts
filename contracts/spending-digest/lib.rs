pub struct SpendingSnapshotInput {
    pub total_spent: u128,
    pub remaining_balance: u128,
    pub active_budgets: Vec<BudgetSnapshot>,
}

pub mod model;
pub mod forecast;
pub mod update;

pub use model::{BudgetSnapshot, BudgetForecast};
pub use forecast::calculate_budget_forecast;
pub use update::update_budget_on_spend;