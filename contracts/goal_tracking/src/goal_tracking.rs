use soroban_sdk::{contracttype, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContributionSource {
    Manual,
    Scheduled,
    Matched,
    Transferred,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalContribution {
    pub id: u64,
    pub amount: i128,
    pub timestamp: u64,
    /// The tracked origin channel configuration for the deposit asset
    pub source: ContributionSource,
    pub reference_note: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavingsGoal {
    pub id: u64,
    pub target_amount: i128,
    pub current_balance: i128,
    pub contributions: Vec<GoalContribution>,
}

pub struct GoalTrackingEngine;

impl GoalTrackingEngine {
    /// Commits a new asset payload to the tracking array while preserving source origin traits.
    pub fn record_contribution(
        env: &Env,
        mut goal: SavingsGoal,
        id: u64,
        amount: i128,
        source: ContributionSource,
        reference_note: String,
    ) -> SavingsGoal {
        let new_contribution = GoalContribution {
            id,
            amount,
            timestamp: env.ledger().timestamp(),
            source,
            reference_note,
        };

        goal.current_balance += amount;
        goal.contributions.push_back(new_contribution);
        goal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Vec, String};

    #[test]
    fn test_record_contributions_with_distinct_sources() {
        let env = Env::default();
        
        let initial_goal = SavingsGoal {
            id: 101,
            target_amount: 5000,
            current_balance: 0,
            contributions: Vec::new(&env),
        };

        // 1. Assert Manual Deposit Source Integration
        let goal_after_manual = GoalTrackingEngine::record_contribution(
            &env,
            initial_goal,
            1,
            500,
            ContributionSource::Manual,
            String::from_str(&env, "OTC Deposit"),
        );
        
        let first_tx = goal_after_manual.contributions.get(0).unwrap();
        assert_eq!(first_tx.source, ContributionSource::Manual);
        assert_eq!(goal_after_manual.current_balance, 500);

        // 2. Assert Matched Tracking Source Integration
        let final_goal = GoalTrackingEngine::record_contribution(
            &env,
            goal_after_manual,
            2,
            500,
            ContributionSource::Matched,
            String::from_str(&env, "Employer Match Payout"),
        );

        let second_tx = final_goal.contributions.get(1).unwrap();
        assert_eq!(second_tx.source, ContributionSource::Matched);
        assert_eq!(final_goal.current_balance, 1000);
    }
}