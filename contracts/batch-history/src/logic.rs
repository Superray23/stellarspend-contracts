use crate::types::UserHistory;
use soroban_sdk::{
    contracttype,
    symbol_short,
    Address,
    Env,
    Vec,
};

/// Maximum number of users allowed in a single batch request.
pub const MAX_BATCH_SIZE: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchHistoryRetrievedEvent {
    pub requested_users: u32,
    pub returned_records: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserHistoryRetrievedEvent {
    pub user: Address,
}

pub fn get_batch_history(
    env: &Env,
    users: Vec<Address>,
) -> Vec<UserHistory> {
    // Early return for empty batches.
    if users.is_empty() {
        return Vec::new(env);
    }

    // Optional protection against excessive resource consumption.
    if users.len() > MAX_BATCH_SIZE {
        panic!("batch size exceeds maximum allowed");
    }

    let mut results = Vec::new(env);

    for user in users.iter() {
        // Future optimization:
        // Attempt cache lookup first.
        //
        // let history = env
        //     .storage()
        //     .temporary()
        //     .get::<_, UserHistory>(&user)
        //     .unwrap_or_else(|| load_history_from_storage(...));

        let history = UserHistory {
            user: user.clone(),
            transactions: Vec::new(env),
        };

        // Emit structured event for indexers.
        env.events().publish(
            (
                symbol_short!("history"),
                symbol_short!("user"),
            ),
            UserHistoryRetrievedEvent {
                user: user.clone(),
            },
        );

        results.push_back(history);
    }

    // Emit aggregate batch event.
    env.events().publish(
        (
            symbol_short!("history"),
            symbol_short!("batch"),
        ),
        BatchHistoryRetrievedEvent {
            requested_users: users.len(),
            returned_records: results.len(),
        },
    );

    results
}