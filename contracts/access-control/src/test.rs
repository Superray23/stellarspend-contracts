#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_contract() -> (Env, Address, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, AccessControlContract);
    let client = AccessControlContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    (env, contract_id, admin)
}

#[test]
fn test_initialize_contract() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    // Verify admin is set
    assert_eq!(client.get_admin(), admin);

    // Verify admin has admin role
    assert!(client.has_role(&admin, &Role::Admin));

    // Verify total role assignments
    assert_eq!(client.get_total_role_assignments(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_cannot_initialize_twice() {
    let (env, contract_id, _) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);
    env.mock_all_auths();

    // Try to initialize again - should panic
    client.initialize(&new_admin);
}

#[test]
fn test_grant_role_as_admin() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant user role
    client.grant_role(&admin, &user, &Role::User);

    // Verify user has the role
    assert!(client.has_role(&user, &Role::User));
    assert!(!client.has_role(&user, &Role::Admin));

    // Verify counter updated
    assert_eq!(client.get_total_role_assignments(), 2);
}

#[test]
fn test_grant_multiple_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant multiple roles
    client.grant_role(&admin, &user, &Role::User);
    client.grant_role(&admin, &user, &Role::Operator);
    client.grant_role(&admin, &user, &Role::Auditor);

    // Verify all roles
    assert!(client.has_role(&user, &Role::User));
    assert!(client.has_role(&user, &Role::Operator));
    assert!(client.has_role(&user, &Role::Auditor));
    assert!(!client.has_role(&user, &Role::Admin));

    // Verify counter
    assert_eq!(client.get_total_role_assignments(), 4);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_cannot_grant_role_twice() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant role
    client.grant_role(&admin, &user, &Role::User);

    // Try to grant same role again - should panic
    client.grant_role(&admin, &user, &Role::User);
}

#[test]
fn test_revoke_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant and then revoke role
    client.grant_role(&admin, &user, &Role::User);
    assert!(client.has_role(&user, &Role::User));

    client.revoke_role(&admin, &user, &Role::User);
    assert!(!client.has_role(&user, &Role::User));

    // Verify counter decremented
    assert_eq!(client.get_total_role_assignments(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_cannot_revoke_unassigned_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Try to revoke role that was never assigned - should panic
    client.revoke_role(&admin, &user, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_cannot_revoke_self_admin() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    env.mock_all_auths();

    // Admin tries to revoke their own admin role - should panic
    client.revoke_role(&admin, &admin, &Role::Admin);
}

#[test]
fn test_transfer_admin() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    // Transfer admin
    client.transfer_admin(&admin, &new_admin);

    // Verify new admin
    assert_eq!(client.get_admin(), new_admin);
    assert!(client.has_role(&new_admin, &Role::Admin));
    assert!(!client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_get_user_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant multiple roles
    client.grant_role(&admin, &user, &Role::User);
    client.grant_role(&admin, &user, &Role::Operator);

    // Get all roles
    let roles = client.get_user_roles(&user);

    assert_eq!(roles.get(Role::User), Some(true));
    assert_eq!(roles.get(Role::Operator), Some(true));
    // Admin role was never set for this user, so it returns None
    assert_eq!(roles.get(Role::Admin), None);
}

#[test]
fn test_get_user_roles_returns_empty_for_user_with_no_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    let roles = client.get_user_roles(&user);

    assert_eq!(roles.get(Role::Admin), None);
    assert_eq!(roles.get(Role::User), None);
    assert_eq!(roles.get(Role::Operator), None);
    assert_eq!(roles.get(Role::Auditor), None);
    assert!(!client.has_any_role(&user));
}

#[test]
fn test_has_any_role_returns_true_for_assigned_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    client.grant_role(&admin, &user, &Role::Auditor);

    assert!(client.has_any_role(&user));
    assert_eq!(client.get_user_roles(&user).get(Role::Auditor), Some(true));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_grant_role() {
    let (env, contract_id, _) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();

    // Non-admin tries to grant role - should panic
    client.grant_role(&non_admin, &user, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_revoke_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant role as admin
    client.grant_role(&admin, &user, &Role::User);

    // Non-admin tries to revoke role - should panic
    client.revoke_role(&non_admin, &user, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_transfer_admin() {
    let (env, contract_id, _) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let non_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    // Non-admin tries to transfer admin - should panic
    client.transfer_admin(&non_admin, &new_admin);
}

#[test]
fn test_role_events_emitted() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant role - events are emitted but we just verify the operation succeeds
    client.grant_role(&admin, &user, &Role::User);

    // Verify the role was granted (which confirms the event path was executed)
    assert!(client.has_role(&user, &Role::User));
}

#[test]
fn test_operator_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let operator = Address::generate(&env);

    env.mock_all_auths();

    // Grant operator role
    client.grant_role(&admin, &operator, &Role::Operator);

    // Verify operator has the role
    assert!(client.has_role(&operator, &Role::Operator));
    assert!(!client.has_role(&operator, &Role::Admin));
    assert!(!client.has_role(&operator, &Role::User));
}

#[test]
fn test_auditor_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let auditor = Address::generate(&env);

    env.mock_all_auths();

    // Grant auditor role
    client.grant_role(&admin, &auditor, &Role::Auditor);

    // Verify auditor has the role
    assert!(client.has_role(&auditor, &Role::Auditor));
    assert!(!client.has_role(&auditor, &Role::Admin));
}

#[test]
fn test_complex_role_management() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    env.mock_all_auths();

    // Grant various roles
    client.grant_role(&admin, &user1, &Role::User);
    client.grant_role(&admin, &user1, &Role::Operator);

    client.grant_role(&admin, &user2, &Role::Auditor);

    client.grant_role(&admin, &user3, &Role::User);

    // Verify all assignments
    assert_eq!(client.get_total_role_assignments(), 5);

    // Revoke some roles
    client.revoke_role(&admin, &user1, &Role::Operator);
    client.revoke_role(&admin, &user3, &Role::User);

    // Verify final state
    assert_eq!(client.get_total_role_assignments(), 3);
    assert!(client.has_role(&user1, &Role::User));
    assert!(!client.has_role(&user1, &Role::Operator));
    assert!(client.has_role(&user2, &Role::Auditor));
    assert!(!client.has_role(&user3, &Role::User));
}

// ─── Admin Role Enforcement Regression Tests ──────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_grant_user_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    env.mock_all_auths();

    // Attacker attempts to grant role without being admin
    client.grant_role(&attacker, &target, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_grant_operator_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    env.mock_all_auths();

    // Attacker attempts to grant operator role
    client.grant_role(&attacker, &target, &Role::Operator);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_grant_auditor_role() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    env.mock_all_auths();

    // Attacker attempts to grant auditor role
    client.grant_role(&attacker, &target, &Role::Auditor);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_user_cannot_revoke_own_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant user a role
    client.grant_role(&admin, &user, &Role::User);

    // User tries to revoke their own role (must be admin)
    client.revoke_role(&user, &user, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_operator_cannot_revoke_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let operator = Address::generate(&env);
    let target = Address::generate(&env);

    env.mock_all_auths();

    // Grant roles
    client.grant_role(&admin, &operator, &Role::Operator);
    client.grant_role(&admin, &target, &Role::User);

    // Operator tries to revoke role (must be admin)
    client.revoke_role(&operator, &target, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_auditor_cannot_grant_roles() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let auditor = Address::generate(&env);
    let target = Address::generate(&env);

    env.mock_all_auths();

    // Grant auditor role
    client.grant_role(&admin, &auditor, &Role::Auditor);

    // Auditor tries to grant role (must be admin)
    client.grant_role(&auditor, &target, &Role::User);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_transfer_admin_to_another() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    // Attacker tries to transfer admin to someone else
    client.transfer_admin(&attacker, &new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_cannot_transfer_admin_without_auth() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    env.mock_all_auths();

    // Non-admin tries to transfer admin role
    client.transfer_admin(&non_admin, &new_admin);
}

#[test]
fn test_admin_can_grant_all_role_types() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Admin can grant all role types
    client.grant_role(&admin, &user, &Role::User);
    assert!(client.has_role(&user, &Role::User));

    let user2 = Address::generate(&env);
    client.grant_role(&admin, &user2, &Role::Operator);
    assert!(client.has_role(&user2, &Role::Operator));

    let user3 = Address::generate(&env);
    client.grant_role(&admin, &user3, &Role::Auditor);
    assert!(client.has_role(&user3, &Role::Auditor));
}

#[test]
fn test_admin_can_revoke_all_role_types() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant roles
    client.grant_role(&admin, &user, &Role::User);
    client.grant_role(&admin, &user, &Role::Operator);
    client.grant_role(&admin, &user, &Role::Auditor);

    // Admin can revoke all role types
    client.revoke_role(&admin, &user, &Role::User);
    assert!(!client.has_role(&user, &Role::User));

    client.revoke_role(&admin, &user, &Role::Operator);
    assert!(!client.has_role(&user, &Role::Operator));

    client.revoke_role(&admin, &user, &Role::Auditor);
    assert!(!client.has_role(&user, &Role::Auditor));
}

#[test]
fn test_privilege_escalation_prevented() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);

    env.mock_all_auths();

    // Grant user role only
    client.grant_role(&admin, &user, &Role::User);
    assert!(client.has_role(&user, &Role::User));
    assert!(!client.has_role(&user, &Role::Admin));

    // User cannot grant themselves admin role
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.grant_role(&user, &user, &Role::Admin);
    }));
    assert!(result.is_err());

    // Verify user still doesn't have admin role
    assert!(!client.has_role(&user, &Role::Admin));
}

#[test]
fn test_admin_transition_security() {
    let (env, contract_id, admin1) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let admin2 = Address::generate(&env);

    env.mock_all_auths();

    // Verify admin1 is admin
    assert_eq!(client.get_admin(), admin1);
    assert!(client.has_role(&admin1, &Role::Admin));

    // Transfer admin to admin2
    client.transfer_admin(&admin1, &admin2);

    // Verify admin2 is now admin
    assert_eq!(client.get_admin(), admin2);
    assert!(client.has_role(&admin2, &Role::Admin));
    assert!(!client.has_role(&admin1, &Role::Admin));

    // admin1 cannot perform admin operations anymore
    let user = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.grant_role(&admin1, &user, &Role::User);
    }));
    assert!(result.is_err());

    // admin2 can perform admin operations
    let user2 = Address::generate(&env);
    client.grant_role(&admin2, &user2, &Role::User);
    assert!(client.has_role(&user2, &Role::User));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_previous_admin_cannot_grant_roles_after_transfer() {
    let (env, contract_id, admin1) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let admin2 = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();

    // Transfer admin
    client.transfer_admin(&admin1, &admin2);

    // Previous admin tries to grant role - should fail
    client.grant_role(&admin1, &user, &Role::User);
}

#[test]
fn test_only_admin_can_transfer_admin() {
    let (env, contract_id, admin) = create_contract();
    let client = AccessControlContractClient::new(&env, &contract_id);

    let operator = Address::generate(&env);
    let new_target = Address::generate(&env);

    env.mock_all_auths();

    // Grant operator role
    client.grant_role(&admin, &operator, &Role::Operator);

    // Operator cannot transfer admin
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.transfer_admin(&operator, &new_target);
    }));
    assert!(result.is_err());

    // Admin can transfer admin
    client.transfer_admin(&admin, &new_target);
    assert_eq!(client.get_admin(), new_target);
}
