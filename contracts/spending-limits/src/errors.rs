use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BudgetError {
    BudgetExceeded = 1,
    BudgetNotFound = 2,
    Unauthorized = 3,
    BudgetAlreadyPaused = 4,
    BudgetAlreadyActive = 5,
}