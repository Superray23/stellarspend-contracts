use soroban_sdk::{Env, Vec};

use crate::{
    storage::DataKey,
    types::{Budget, BudgetMergeRecord},
};

pub fn merge_budgets(
    env: &Env,
    target_budget_id: u64,
    source_budget_ids: Vec<u64>,
) {
    let mut target: Budget = env
        .storage()
        .instance()
        .get(&DataKey::Budget(target_budget_id))
        .expect("Target budget not found");

    if target.archived {
        panic!("Target budget archived");
    }

    let mut merged_balance = target.balance;

    for source_id in source_budget_ids.iter() {
        let mut source: Budget = env
            .storage()
            .instance()
            .get(&DataKey::Budget(source_id))
            .expect("Source budget not found");

        if source.archived {
            panic!("Source budget archived");
        }

        merged_balance += source.balance;

        source.archived = true;

        env.storage()
            .instance()
            .set(
                &DataKey::Budget(source_id),
                &source,
            );
    }

    target.balance = merged_balance;

    env.storage()
        .instance()
        .set(
            &DataKey::Budget(target_budget_id),
            &target,
        );

    let record = BudgetMergeRecord {
        target_budget_id,
        source_budget_ids,
        merged_balance,
        timestamp: env.ledger().timestamp(),
    };

    env.storage()
        .instance()
        .set(
            &DataKey::MergeRecord(target_budget_id),
            &record,
        );
}