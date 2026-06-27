use soroban_sdk::{contracttype, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleAction {
    Allow,
    Deny,
    RequireApproval,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendingRule {
    pub id: String,
    pub name: String,
    pub action: RuleAction,
    /// Higher numerical values represent higher precedence (e.g., priority 100 overrides priority 1)
    pub priority: u32,
    pub max_amount: i128,
}

pub struct SpendingRuleProcessor;

impl SpendingRuleProcessor {
    /// Evaluates conflicting spending rules and returns the highest priority deterministic matching rule.
    pub fn evaluate_rules(rules: Vec<SpendingRule>, amount: i128) -> Option<SpendingRule> {
        if rules.is_empty() {
            return None;
        }

        // Filter and collect rules triggered by the target transaction amount criteria
        let mut applicable_rules: std::vec::Vec<SpendingRule> = rules
            .iter()
            .filter(|rule| amount > rule.max_amount)
            .collect();

        if applicable_rules.is_empty() {
            return None;
        }

        // Sort deterministically:
        // 1. Primary: Descending Order of Priority Score (b.priority.cmp(&a.priority))
        // 2. Secondary (Tie-Breaker): Alphabetical/Lexicographical ordering of rule IDs
        applicable_rules.sort_by(|a, b| {
            match b.priority.cmp(&a.priority) {
                std::cmp::Ordering::Equal => {
                    // Convert Soroban SDK Strings to comparable vectors or standard types safely
                    a.id.private_cmp(&b.id)
                }
                other => other,
            }
        });

        // Return a copy of the highest priority matching rule safely
        Some(applicable_rules[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_evaluate_highest_priority_first() {
        let env = Env::default();

        let rule_low = SpendingRule {
            id: String::from_str(&env, "rule-low"),
            name: String::from_str(&env, "Low Priority Deny"),
            action: RuleAction::Deny,
            priority: 10,
            max_amount: 100,
        };

        let rule_high = SpendingRule {
            id: String::from_str(&env, "rule-high"),
            name: String::from_str(&env, "High Priority Approval"),
            action: RuleAction::RequireApproval,
            priority: 90,
            max_amount: 100,
        };

        let mut rules = Vec::new(&env);
        rules.push_back(rule_low);
        rules.push_back(rule_high);

        let result = SpendingRuleProcessor::evaluate_rules(rules, 150).unwrap();
        assert_eq!(result.id, String::from_str(&env, "rule-high"));
        assert_eq!(result.action, RuleAction::RequireApproval);
    }

    #[test]
    fn test_deterministic_tie_breaker_resolution() {
        let env = Env::default();

        // Both rules share priority 50. Sorting fallback checks the ID string lexicographically.
        let rule_bravo = SpendingRule {
            id: String::from_str(&env, "rule-bravo"),
            name: String::from_str(&env, "Bravo Rule"),
            action: RuleAction::Deny,
            priority: 50,
            max_amount: 200,
        };

        let rule_alpha = SpendingRule {
            id: String::from_str(&env, "rule-alpha"),
            name: String::from_str(&env, "Alpha Rule"),
            action: RuleAction::Allow,
            priority: 50,
            max_amount: 200,
        };

        let mut rules = Vec::new(&env);
        rules.push_back(rule_bravo);
        rules.push_back(rule_alpha);

        let result = SpendingRuleProcessor::evaluate_rules(rules, 300).unwrap();
        // "rule-alpha" comes before "rule-bravo" lexicographically
        assert_eq!(result.id, String::from_str(&env, "rule-alpha"));
        assert_eq!(result.action, RuleAction::Allow);
    }
}