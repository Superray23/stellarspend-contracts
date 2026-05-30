use soroban_sdk::Env;

use crate::types::ContributionPeriod;

pub fn current_bucket(
    env: &Env,
    period: &ContributionPeriod,
) -> u64 {
    let timestamp = env.ledger().timestamp();

    match period {
        ContributionPeriod::Daily => timestamp / 86_400,
        ContributionPeriod::Weekly => timestamp / 604_800,
        ContributionPeriod::Monthly => timestamp / 2_592_000,
    }
}