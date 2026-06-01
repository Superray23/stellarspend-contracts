use soroban_sdk::{Env, Address};

use crate::recurring::types::RecurringExpense;

pub fn execute_recurring_expense(
    env: &Env,
    token: &Address,
    contract: &Address,
    expense: &mut RecurringExpense,
    current_time: u64,
) -> Result<(), &'static str> {
    if !expense.active {
        return Ok(());
    }

    if current_time < expense.next_due {
        return Ok(());
    }

    // Attempt deduction
    let transfer_result = token.try_transfer(
        contract,
        &expense.owner,
        &expense.amount,
    );

    match transfer_result {
        Ok(_) => {
            // schedule next run
            expense.next_due = current_time + (expense.interval_days as u64 * 86400);
            Ok(())
        }

        Err(_) => {
            // safe failure handling: deactivate or retry later
            expense.active = false;
            Ok(())
        }
    }
}