#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_event_once_per_cycle() {
        let mut state = DigestState::default();

        let input = SpendingSnapshotInput {
            total_spent: 500,
            remaining_balance: 1500,
            active_budgets: vec![],
        };

        let date = "2026-05-29".to_string();

        let first = run_daily_cycle(&mut state, "user1".to_string(), input.clone(), date.clone());
        assert!(first.is_some());

        let second = run_daily_cycle(&mut state, "user1".to_string(), input, date.clone());
        assert!(second.is_none());
    }

    #[test]
    fn emits_correct_values() {
        let mut state = DigestState::default();

        let input = SpendingSnapshotInput {
            total_spent: 200,
            remaining_balance: 800,
            active_budgets: vec![],
        };

        let event = run_daily_cycle(&mut state, "user1".to_string(), input, "2026-05-29".to_string())
            .unwrap();

        assert_eq!(event.total_spent, 200);
        assert_eq!(event.remaining_balance, 800);
    }
}

use crate::model::BudgetSnapshot;
use crate::{calculate_budget_forecast, update_budget_on_spend};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_depletion_correctly() {
        let budget = BudgetSnapshot {
            budget_id: "b1".to_string(),
            total_limit: 1000,
            spent_so_far: 500,
            days_active: 5,
            spend_history: vec![100, 100, 100, 100, 100],
        };

        let forecast = calculate_budget_forecast(&budget, "2026-05-29".to_string());

        assert_eq!(forecast.daily_average_spend, 100.0);
        assert!(forecast.projected_depletion_days.is_some());
    }

    #[test]
    fn updates_budget_after_spend() {
        let mut budget = BudgetSnapshot {
            budget_id: "b1".to_string(),
            total_limit: 1000,
            spent_so_far: 200,
            days_active: 2,
            spend_history: vec![100, 100],
        };

        update_budget_on_spend(&mut budget, 50);

        assert_eq!(budget.spent_so_far, 250);
        assert_eq!(budget.spend_history.len(), 3);
    }
}