pub fn emit_daily_digest(
    state: &mut DigestState,
    user_id: String,
    total_spent: u128,
    remaining_balance: u128,
    active_budgets: Vec<BudgetSnapshot>,
    current_date: String,
) -> Option<SpendingDigestEvent> {

    // Idempotency guard: ensure one emission per cycle
    if let Some(last_date) = &state.last_emission_date {
        if last_date == &current_date {
            return None;
        }
    }

    let event = SpendingDigestEvent {
        user_id,
        date: current_date.clone(),

        total_spent,
        remaining_balance,
        active_budgets,

        emitted_at: current_date.clone(),
    };

    state.last_emission_date = Some(current_date);

    Some(event)
}