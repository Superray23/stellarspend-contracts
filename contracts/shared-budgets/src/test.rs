// Integration tests for the Shared Budget Contract.

#![cfg(test)]

use crate::{
    BudgetSpendingRule, BudgetUtilizationBand, SharedBudgetContract, SharedBudgetContractClient,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, Symbol, Vec,
};

/// Creates a test environment with the contract deployed and initialized.
fn setup_test_env() -> (
    Env,
    Address,
    Address,
    token::Client<'static>,
    SharedBudgetContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.sequence_number = 12345;
    });

    // Deploy token contract (simulating XLM StellarAssetContract)
    let issuer = Address::generate(&env);
    let stellar_asset = env.register_stellar_asset_contract_v2(issuer);
    let token_id: Address = stellar_asset.address();
    let token_client = token::Client::new(&env, &token_id);

    // Deploy shared budget contract
    let contract_id = env.register(SharedBudgetContract, ());
    let client = SharedBudgetContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, admin, token_id, token_client, client)
}

fn mint_balance(env: &Env, token: &Address, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token).mint(recipient, &amount);
}

// Initialization Tests

#[test]
fn test_initialize_contract() {
    let (_env, admin, _token, _token_client, client) = setup_test_env();

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_total_budgets_created(), 0);
    assert_eq!(client.get_total_contribs_processed(), 0);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_cannot_initialize_twice() {
    let (env, _admin, _token, _token_client, client) = setup_test_env();

    let new_admin = Address::generate(&env);
    client.initialize(&new_admin);
}

// Budget Creation Tests

#[test]
fn test_create_budget() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());

    let budget_name = Symbol::new(&env, "family_budget");

    let mut spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);
    let rule = BudgetSpendingRule {
        applicable_to: member1.clone(),
        percentage_threshold: 10,
        requires_approval: false,
        description: Symbol::new(&env, "small_purchases"),
    };
    spending_rules.push_back(rule);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    assert!(budget_id > 0);

    let budget = client.get_budget(&budget_id);
    assert_eq!(budget.name, budget_name);
    assert_eq!(budget.creator, creator);
    assert_eq!(budget.token, token);
    assert_eq!(budget.balance, 0);
    assert_eq!(budget.total_contributed, 0);
    assert_eq!(budget.is_active, true);
    assert!(client.is_budget_member(&budget_id, &member1));
    assert!(client.is_budget_member(&budget_id, &member2));
}

#[test]
fn test_contribute_to_budget() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);
    let member1 = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "vacation_fund");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Initially budget should have 0 balance
    let budget_before = client.get_budget(&budget_id);
    assert_eq!(budget_before.balance, 0);

    // Contribute to the budget
    let contribution_amount = 100_000_000; // 10 XLM
    mint_balance(&env, &token, &contributor, contribution_amount);
    client.contribute_to_budget(&contributor, &budget_id, &contribution_amount, &None);

    // Check that budget balance increased
    let budget_after = client.get_budget(&budget_id);
    assert_eq!(budget_after.balance, contribution_amount);
    assert_eq!(budget_after.total_contributed, contribution_amount);

    // Check that total contributions processed increased
    assert_eq!(client.get_total_contribs_processed(), 1);
}

#[test]
fn test_contribution_with_memo() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);
    let budget_name = Symbol::new(&env, "memo_budget");
    let budget_id = client.create_budget(
        &creator,
        &budget_name,
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    let amount = 100_000_000;
    let memo = Some(Symbol::new(&env, "lunch_contribution"));
    mint_balance(&env, &token, &contributor, amount);
    client.contribute_to_budget(&contributor, &budget_id, &amount, &memo);

    let contribution_id = client.get_total_contribs_processed();
    let contribution = client.get_contribution(&contribution_id);

    assert_eq!(contribution.amount, amount);
    assert_eq!(contribution.memo, memo);
    assert_eq!(contribution.contributor, contributor);
}

#[test]
fn test_spend_from_budget() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "project_fund");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Contribute to budget first
    let contribution_amount = 100_000_000; // 10 XLM
    mint_balance(&env, &token, &member1, contribution_amount);
    client.contribute_to_budget(&member1, &budget_id, &contribution_amount, &None);

    // Spend from budget
    let expense_amount = 50_000_000; // 5 XLM
    client.spend_from_budget(&member1, &budget_id, &recipient, &expense_amount);

    // Check that budget balance decreased
    let budget_after = client.get_budget(&budget_id);
    assert_eq!(budget_after.balance, contribution_amount - expense_amount);
}

#[test]
fn test_add_member_to_budget() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let existing_member = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(existing_member.clone());

    let budget_name = Symbol::new(&env, "team_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Add new member
    let new_member = Address::generate(&env);
    client.add_member_to_budget(&creator, &budget_id, &new_member);

    // Check that new member is now part of budget
    assert!(client.is_budget_member(&budget_id, &new_member));
}

#[test]
fn test_add_spending_rule() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "controlled_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Add spending rule
    let new_rule = BudgetSpendingRule {
        applicable_to: member1.clone(),
        percentage_threshold: 20,
        requires_approval: true,
        description: Symbol::new(&env, "approval_required"),
    };

    client.add_spending_rule(&creator, &budget_id, &new_rule);

    // Verify the rule was added by attempting to get the budget and check its rules
    let budget = client.get_budget(&budget_id);
    assert!(budget.spending_rules.len() > 0);
}

#[test]
fn test_budget_events_emitted() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "event_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Check that events were emitted
    let events = env.events().all();
    assert!(events.len() >= 1); // At least budget creation event

    // Check that events were emitted (simplified check)
    assert!(!events.is_empty());
}

#[test]
fn test_budget_stats_accumulate() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    assert_eq!(client.get_total_budgets_created(), 0);

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "stats_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    assert_eq!(client.get_total_budgets_created(), 1);

    // Create another budget
    let budget_name2 = Symbol::new(&env, "stats_budget2");
    client.create_budget(&creator, &budget_name2, &members, &token, &spending_rules);

    assert_eq!(client.get_total_budgets_created(), 2);
}

#[test]
fn test_budget_utilization_band_queries() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member.clone());

    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "util_budget"),
        &members,
        &token,
        &Vec::new(&env),
    );

    mint_balance(&env, &token, &member, 100_000_000_i128);
    client.contribute_to_budget(&member, &budget_id, &100_000_000_i128, &None);
    assert_eq!(
        client.get_budget_utilization_band(&budget_id),
        BudgetUtilizationBand::Low
    );

    client.spend_from_budget(&member, &budget_id, &recipient, &30_000_000_i128);
    assert_eq!(
        client.get_budget_utilization_band(&budget_id),
        BudgetUtilizationBand::Moderate
    );

    client.spend_from_budget(&member, &budget_id, &recipient, &30_000_000_i128);
    assert_eq!(
        client.get_budget_utilization_band(&budget_id),
        BudgetUtilizationBand::High
    );

    client.spend_from_budget(&member, &budget_id, &recipient, &25_000_000_i128);
    let summary = client.get_budget_utilization_summary(&budget_id);

    assert_eq!(summary.utilization_percent, 85);
    assert_eq!(summary.total_spent, 85_000_000);
    assert_eq!(summary.remaining_balance, 15_000_000);
    assert_eq!(summary.utilization_band, BudgetUtilizationBand::Critical);
}

#[test]
fn test_archive_inactive_budget_after_retention() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let admin = client.get_admin();
    client.set_archive_retention_period(&admin, &3_600);

    let creator = Address::generate(&env);
    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "archivable"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    client.deactivate_budget(&creator, &budget_id);
    env.ledger().with_mut(|li| {
        li.timestamp = 3_601;
    });

    assert_eq!(client.archive_inactive_budgets(&admin, &10), 1);
    assert!(client.try_get_budget(&budget_id).is_err());

    let archived = client.get_archived_budget(&budget_id).unwrap();
    assert_eq!(archived.budget.id, budget_id);
    assert_eq!(archived.budget.is_active, false);
    assert_eq!(archived.deactivated_at, 0);
    assert_eq!(archived.archived_at, 3_601);
    assert_eq!(client.get_contributions(&budget_id).len(), 0);
}

// Error Tests

#[test]
fn test_spend_without_sufficient_funds() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "empty_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Try to spend without contributing anything
    let expense_amount = 50_000_000; // 5 XLM
    let result = client.try_spend_from_budget(&member1, &budget_id, &recipient, &expense_amount);
    assert!(result.is_err());
}

#[test]
fn test_non_member_cannot_spend() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let non_member = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member1.clone());

    let budget_name = Symbol::new(&env, "restricted_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);

    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    // Contribute to budget first
    let contribution_amount = 100_000_000; // 10 XLM
    mint_balance(&env, &token, &member1, contribution_amount);
    client.contribute_to_budget(&member1, &budget_id, &contribution_amount, &None);

    // Non-member tries to spend (should fail)
    let expense_amount = 50_000_000; // 5 XLM
    let result = client.try_spend_from_budget(&non_member, &budget_id, &recipient, &expense_amount);
    assert!(result.is_err());
}

#[test]
#[should_panic]
fn test_unauthorized_admin_function() {
    let (env, admin, token, _token_client, client) = setup_test_env();

    let unauthorized_user = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.set_admin(&unauthorized_user, &new_admin);
}

#[test]
fn test_transfer_budget_ownership() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let member = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member.clone());

    let budget_name = Symbol::new(&env, "transfer_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);
    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    client.transfer_budget_ownership(&creator, &budget_id, &new_owner);

    let budget = client.get_budget(&budget_id);
    assert_eq!(budget.creator, new_owner);

    let owner_role = client.get_member_role(&budget_id, &new_owner);
    assert_eq!(owner_role, Symbol::new(&env, "OWNER"));

    let previous_owner_role = client.get_member_role(&budget_id, &creator);
    assert_eq!(previous_owner_role, Symbol::new(&env, "NONE"));
}

#[test]
#[should_panic]
fn test_previous_owner_cannot_manage_budget_after_transfer() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let member = Address::generate(&env);
    let added_member = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member.clone());

    let budget_name = Symbol::new(&env, "post_transfer_budget");
    let spending_rules: Vec<BudgetSpendingRule> = Vec::new(&env);
    let budget_id = client.create_budget(&creator, &budget_name, &members, &token, &spending_rules);

    client.transfer_budget_ownership(&creator, &budget_id, &new_owner);

    // Previous owner no longer has owner-level control.
    client.add_member_to_budget(&creator, &budget_id, &added_member);
}

#[test]
#[should_panic]
fn test_unauthorized_budget_ownership_transfer() {
    let (env, _admin, token, _token_client, client) = setup_test_env();

    let creator = Address::generate(&env);
    let impostor = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let budget_name = Symbol::new(&env, "secure_budget");
    let budget_id = client.create_budget(
        &creator,
        &budget_name,
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    client.transfer_budget_ownership(&impostor, &budget_id, &new_owner);
}

#[test]
fn test_get_contributions_empty() {
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "empty_budget"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    let contribs = client.get_contributions(&budget_id);
    assert_eq!(contribs.len(), 0);
}

#[test]
fn test_get_contributions_multiple() {
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let member = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member.clone());

    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "multi_budget"),
        &members,
        &token,
        &Vec::new(&env),
    );

    mint_balance(&env, &token, &member, 300_000_000_i128);
    client.contribute_to_budget(&member, &budget_id, &100_000_000_i128, &None);
    client.contribute_to_budget(
        &member,
        &budget_id,
        &200_000_000_i128,
        &Some(Symbol::new(&env, "second")),
    );

    let contribs = client.get_contributions(&budget_id);
    assert_eq!(contribs.len(), 2);
    assert_eq!(contribs.get(0).unwrap().amount, 100_000_000);
    assert_eq!(contribs.get(0).unwrap().contributor, member);
    assert_eq!(contribs.get(1).unwrap().amount, 200_000_000);
    assert_eq!(
        contribs.get(1).unwrap().memo,
        Some(Symbol::new(&env, "second"))
    );
}

#[test]
fn test_get_contributions_isolated_per_budget() {
    // Contributions from budget A must not appear in budget B
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);

    let budget_a = client.create_budget(
        &creator,
        &Symbol::new(&env, "budget_a"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );
    let budget_b = client.create_budget(
        &creator,
        &Symbol::new(&env, "budget_b"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    mint_balance(&env, &token, &contributor, 50_000_000_i128);
    client.contribute_to_budget(&contributor, &budget_a, &50_000_000_i128, &None);

    assert_eq!(client.get_contributions(&budget_a).len(), 1);
    assert_eq!(client.get_contributions(&budget_b).len(), 0);
}

#[test]
fn test_get_contributions_paginated_empty() {
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "empty_budget_paginated"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    let contribs = client.get_contributions_paginated(&budget_id, &0, &10);
    assert_eq!(contribs.len(), 0);
}

#[test]
fn test_get_contributions_paginated_multiple_pages() {
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let member = Address::generate(&env);

    let mut members: Vec<Address> = Vec::new(&env);
    members.push_back(member.clone());

    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "multi_page_budget"),
        &members,
        &token,
        &Vec::new(&env),
    );

    mint_balance(&env, &token, &member, 600_000_000_i128);
    client.contribute_to_budget(&member, &budget_id, &100_000_000_i128, &None);
    client.contribute_to_budget(
        &member,
        &budget_id,
        &200_000_000_i128,
        &Some(Symbol::new(&env, "second")),
    );
    client.contribute_to_budget(
        &member,
        &budget_id,
        &300_000_000_i128,
        &Some(Symbol::new(&env, "third")),
    );

    let page1 = client.get_contributions_paginated(&budget_id, &0, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().amount, 100_000_000);
    assert_eq!(page1.get(1).unwrap().amount, 200_000_000);

    let page2 = client.get_contributions_paginated(&budget_id, &2, &2);
    assert_eq!(page2.len(), 1);
    assert_eq!(page2.get(0).unwrap().amount, 300_000_000);
}

#[test]
fn test_get_contributions_paginated_deterministic_order() {
    let (env, _admin, token, _token_client, client) = setup_test_env();
    let creator = Address::generate(&env);
    let member = Address::generate(&env);

    let budget_id = client.create_budget(
        &creator,
        &Symbol::new(&env, "deterministic_budget"),
        &Vec::new(&env),
        &token,
        &Vec::new(&env),
    );

    mint_balance(&env, &token, &member, 60_000_000_i128);
    client.contribute_to_budget(&member, &budget_id, &10_000_000_i128, &None);
    client.contribute_to_budget(&member, &budget_id, &20_000_000_i128, &None);
    client.contribute_to_budget(&member, &budget_id, &30_000_000_i128, &None);

    let first_page = client.get_contributions_paginated(&budget_id, &0, &2);
    let second_page = client.get_contributions_paginated(&budget_id, &2, &2);

    assert_eq!(first_page.get(0).unwrap().amount, 10_000_000);
    assert_eq!(first_page.get(1).unwrap().amount, 20_000_000);
    assert_eq!(second_page.get(0).unwrap().amount, 30_000_000);
}

#[test]
#[should_panic]
fn test_get_contributions_nonexistent_budget() {
    let (_env, _admin, _token, _token_client, client) = setup_test_env();
    client.get_contributions(&9999_u64);
}
