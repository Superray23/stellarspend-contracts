#[derive(Debug, Clone)]
pub struct BudgetSnapshot {
    pub budget_id: String,
    pub total_limit: u128,
    pub spent_so_far: u128,

    pub days_active: u32,
    pub spend_history: Vec<u128>,
}

#[derive(Debug, Clone)]
pub struct BudgetForecast {
    pub budget_id: String,

    pub daily_average_spend: f64,
    pub projected_depletion_days: Option<u32>,
    pub projected_depletion_date: Option<String>,

    pub confidence_score: f64,
}