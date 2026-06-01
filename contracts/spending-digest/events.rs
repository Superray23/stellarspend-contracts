#[derive(Debug, Clone, serde::Serialize)]
pub struct SpendingDigestEvent {
    pub user_id: String,
    pub date: String,

    pub total_spent: u128,
    pub remaining_balance: u128,

    pub active_budgets: Vec<BudgetSnapshot>,

    pub emitted_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetSnapshot {
    pub budget_id: String,
    pub name: String,
    pub limit: u128,
    pub spent: u128,
}