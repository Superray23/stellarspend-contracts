pub fn run_daily_cycle(
    state: &mut DigestState,
    user_id: String,
    input: SpendingSnapshotInput,
    current_date: String,
) -> Option<SpendingDigestEvent> {

    emit_daily_digest(
        state,
        user_id,
        input.total_spent,
        input.remaining_balance,
        input.active_budgets,
        current_date,
    )
}