use soroban_sdk::Env;

use soroban_sdk::Address;

use crate::penalty::config::{get_penalty_percent, get_treasury};
use crate::treasury::TreasuryContractClient;

pub fn apply_penalty_withdrawal(
    env: &Env,
    token: &Address,
    contract: &Address,
    user: &Address,
    amount: i128,
) {
    let penalty_percent = get_penalty_percent(env);

    let penalty: i128 = amount * penalty_percent as i128 / 100;
    let withdrawable: i128 = amount - penalty;

    let treasury = get_treasury(env);

    // transfer to user
    token.transfer(contract, user, &withdrawable);

    // send penalty to treasury
    token.transfer(contract, &treasury, &penalty);

    // record penalty in treasury accounting
    let client = TreasuryContractClient::new(env, &treasury);
    client.credit_penalty(&penalty);
}