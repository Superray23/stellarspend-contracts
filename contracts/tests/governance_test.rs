#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

#[path = "../governance.rs"]
mod governance;

use governance::{GovernanceContract, GovernanceContractClient};

#[test]
fn initialize_and_update_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let contract_id = env.register(GovernanceContract, ());
    let client = GovernanceContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1);
    assert_eq!(client.get_admin(), admin);

    client.update_admin(&admin, &new_admin);
    assert_eq!(client.get_admin(), new_admin);
}
