// Types and events for shared budget management.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};

/// Maximum number of members allowed in a budget.
pub const MAX_BUDGET_MEMBERS: u32 = 20;

/// Maximum number of spending rules allowed in a budget.
pub const MAX_SPENDING_RULES: u32 = 10;


/// Represents a shared budget with multiple members.
#[derive(Clone, Debug)]
#[contracttype]
pub struct Budget {
    /// Unique identifier for the budget
    pub id: u64,
    /// Name of the budget
    pub name: Symbol,
    /// Creator of the budget
    pub creator: Address,
    /// Token type for the budget
    pub token: Address,
    /// Members of the budget
    pub members: Vec<Address>,
    /// Current balance of the budget
    pub balance: i128,
    /// Total amount contributed to the budget
    pub total_contributed: i128,
    /// Spending rules for the budget
    pub spending_rules: Vec<BudgetSpendingRule>,
    /// Whether the budget is active
    pub is_active: bool,
    /// Timestamp when the budget was created
    pub created_at: u64,
}

/// Represents a contribution to a shared budget.
#[derive(Clone, Debug)]
#[contracttype]
pub struct BudgetContribution {
    /// ID of the budget this contribution belongs to
    pub budget_id: u64,
    /// Address of the contributor
    pub contributor: Address,
    /// Amount contributed
    pub amount: i128,
    /// Optional memo for the contribution
    pub memo: Option<Symbol>,
    /// Timestamp of the contribution
    pub timestamp: u64,
}

/// Represents a spending rule for a budget.
#[derive(Clone, Debug)]
#[contracttype]
pub struct BudgetSpendingRule {
    /// Address this rule applies to (0 address means all members)
    pub applicable_to: Address,
    /// Percentage threshold for the rule
    pub percentage_threshold: u32,
    /// Whether approval is required for spending beyond threshold
    pub requires_approval: bool,
    /// Description of the rule
    pub description: Symbol,
}

/// Storage keys for contract state.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Admin address
    Admin,
    /// Budget details by ID
    Budget(u64),
    /// Whether an address is a member of a budget
    BudgetMember(u64, Address),
    /// Contribution details by ID
    Contribution(u64),
    /// Total number of budgets created
    TotalBudgetsCreated,
    /// Total number of contributions processed
    TotalContributionsProcessed,
}

/// Events emitted by the shared budget contract.
pub struct SharedBudgetEvents;

impl SharedBudgetEvents {
    /// Event emitted when a budget is created.
    pub fn budget_created(
        env: &Env,
        budget_id: u64,
        creator: &Address,
        members: &Vec<Address>,
        token: &Address,
    ) {
        let topics = (symbol_short!("budget"), symbol_short!("created"));
        env.events().publish(
            topics,
            (budget_id, creator.clone(), members.clone(), token.clone()),
        );
    }

    /// Event emitted when a contribution is added to a budget.
    pub fn contribution_added(
        env: &Env,
        budget_id: u64,
        contributor: &Address,
        amount: i128,
        memo: Option<Symbol>,
    ) {
        let topics = (symbol_short!("budget"), symbol_short!("contrib"), budget_id);
        env.events()
            .publish(topics, (contributor.clone(), amount, memo));
    }

    /// Event emitted when an allocation fails for a recipient.
    pub fn allocation_failure(
        env: &Env,
        batch_id: u64,
        recipient: &Address,
        amount: i128,
        error_code: u32,
    ) {
        let topics = (symbol_short!("alloc"), symbol_short!("failed"), batch_id);
        env.events()
            .publish(topics, (recipient.clone(), amount, error_code));
    }

    /// Event emitted when allocation batch processing completes.
    pub fn batch_completed(
        env: &Env,
        batch_id: u64,
        successful: u32,
        failed: u32,
        total_allocated: i128,
    ) {
        let topics = (symbol_short!("alloc"), symbol_short!("completed"), batch_id);
        env.events()
            .publish(topics, (successful, failed, total_allocated));
    }

    /// Event emitted when an expense is incurred against a budget.
    pub fn expense_incurred(
        env: &Env,
        budget_id: u64,
        spender: &Address,
        recipient: &Address,
        amount: i128,
    ) {
        let topics = (symbol_short!("budget"), symbol_short!("expense"), budget_id);
        env.events()
            .publish(topics, (spender.clone(), recipient.clone(), amount));
    }

    /// Event emitted when a spending rule is added to a budget.
    pub fn spending_rule_added(env: &Env, budget_id: u64, rule: &BudgetSpendingRule) {
        let topics = (symbol_short!("budget"), symbol_short!("rule"), budget_id);
        env.events().publish(
            topics,
            (
                rule.applicable_to.clone(),
                rule.percentage_threshold,
                rule.requires_approval,
            ),
        );
    }

    /// Event emitted when budget ownership is transferred to a new account.
    pub fn ownership_transferred(
        env: &Env,
        budget_id: u64,
        previous_owner: &Address,
        new_owner: &Address,
    ) {
        let topics = (
            symbol_short!("budget"),
            symbol_short!("xfer_own"),
            budget_id,
        );
        env.events()
            .publish(topics, (previous_owner.clone(), new_owner.clone()));
    }
}

