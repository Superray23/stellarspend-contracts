use soroban_sdk::{Env};

use crate::recurring::types::RecurringExpense;

pub fn schedule_next(expense: &mut RecurringExpense, current_time: u64) {
    expense.next_due = current_time + (expense.interval_days as u64 * 86400);
}