use soroban_sdk::{contracttype, Env, Map, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetState {
    pub allocated: i128,
    pub spent: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetSnapshot {
    pub category_balances: Map<Symbol, BudgetState>,
}

pub struct TransactionalBudgetManager;

impl TransactionalBudgetManager {
    /// Creates an isolated backup snapshot of the current budget map allocations
    pub fn create_snapshot(balances: Map<Symbol, BudgetState>) -> BudgetSnapshot {
        BudgetSnapshot {
            category_balances: balances.clone(),
        }
    }

    /// Mutates a category budget balance safely. 
    /// Returns `Ok(Map)` on success, or `Err` if a step violates limits.
    pub fn try_spend(
        mut balances: Map<Symbol, BudgetState>,
        category: Symbol,
        amount: i128,
    ) -> Result<Map<Symbol, BudgetState>, &'static str> {
        let mut state = balances.get(category.clone()).unwrap_or(BudgetState {
            allocated: 0,
            spent: 0,
        });

        if state.spent + amount > state.allocated {
            return Err("BUDGET_EXCEEDED");
        }

        state.spent += amount;
        balances.set(category, state);
        Ok(balances)
    }

    /// Explicitly rolls back the runtime balances to a verified previous snapshot state
    pub fn rollback(snapshot: BudgetSnapshot) -> Map<Symbol, BudgetState> {
        snapshot.category_balances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Map, Symbol};

    #[test]
    fn test_atomic_multi_step_spending_rollback() {
        let env = Env::default();
        let cat_ops = Symbol::from_str(&env, "Operations");
        let cat_mkt = Symbol::from_str(&env, "Marketing");

        let mut balances = Map::new(&env);
        balances.set(cat_ops.clone(), BudgetState { allocated: 1000, spent: 200 });
        balances.set(cat_mkt.clone(), BudgetState { allocated: 500, spent: 100 });

        // 1. Take a safe transactional snapshot checkpoint before starting multi-step changes
        let checkpoint = TransactionalBudgetManager::create_snapshot(balances.clone());

        // Step 1: Spend 100 from Operations (Valid)
        let step_1_res = TransactionalBudgetManager::try_spend(balances, cat_ops.clone(), 100);
        assert!(step_1_res.is_ok());
        balances = step_1_res.unwrap();

        // Step 2: Spend 500 from Marketing (Invalid! Exceeds allocation limits)
        let step_2_res = TransactionalBudgetManager::try_spend(balances.clone(), cat_mkt.clone(), 500);
        
        if step_2_res.is_err() {
            // Atomic Enforcement: Revert the entire runtime state back to the pre-transaction checkpoint
            balances = TransactionalBudgetManager::rollback(checkpoint);
        }

        // Verify that partial changes from Step 1 were completely discarded
        let rolled_back_ops = balances.get(cat_ops).unwrap();
        assert_eq!(rolled_back_ops.spent, 200); // Remained at baseline 200, NOT 300
    }
}