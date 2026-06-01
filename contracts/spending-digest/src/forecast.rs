use crate::model::{BudgetSnapshot, BudgetForecast};

pub fn calculate_budget_forecast(
    budget: &BudgetSnapshot,
    current_date: String,
) -> BudgetForecast {

    let days = budget.days_active.max(1) as f64;

    let total_spent = budget.spent_so_far as f64;
    let remaining = (budget.total_limit as f64) - total_spent;

    let daily_avg = total_spent / days;

    let projected_days = if daily_avg > 0.0 {
        Some((remaining / daily_avg).ceil() as u32)
    } else {
        None
    };

    let projected_date = projected_days.map(|d| {
        format!("{}+{}d", current_date, d)
    });

    let confidence = if budget.spend_history.len() >= 7 {
        0.85
    } else if budget.spend_history.len() >= 3 {
        0.65
    } else {
        0.40
    };

    BudgetForecast {
        budget_id: budget.budget_id.clone(),
        daily_average_spend: daily_avg,
        projected_depletion_days: projected_days,
        projected_depletion_date: projected_date,
        confidence_score: confidence,
    }
}