use crate::model::BudgetSnapshot;

pub fn update_budget_on_spend(
    budget: &mut BudgetSnapshot,
    new_spend: u128,
) {
    budget.spent_so_far += new_spend;
    budget.spend_history.push(new_spend);

    budget.days_active += 1;
}