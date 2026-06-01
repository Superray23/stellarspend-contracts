#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};
//pub struct BatchHistoryContract;
mod logic;
mod types;

#[cfg(test)]
mod test;

use crate::types::UserHistory;

#[contract]
pub struct BatchHistoryContract;

#[contractimpl]
impl BatchHistoryContract {
    pub fn retrieve_histories(
        env: Env,
        requester: Address,
        users: Vec<Address>,
    ) -> Vec<UserHistory> {
        // Requirement: Validate user/requester
        requester.require_auth();

        logic::get_batch_history(env, users)
    }
}
