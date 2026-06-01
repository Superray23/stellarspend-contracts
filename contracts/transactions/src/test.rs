use crate::{
    Transaction, TransactionError, TransactionStatus, TransactionsContract,
    TransactionsContractClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, Map, String, Symbol, Vec,
};

#[test]
fn test_initialize_and_get_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);
    assert_eq!(client.get_admin(), Some(admin.clone()));
}

#[test]
#[should_panic]
fn test_initialize_duplicate_fails() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);
    let admin2 = Address::generate(&env);
    client.initialize(&admin2);
}

#[test]
fn test_create_transaction() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Test transaction");
    let memo = String::from_str(&env, "Payment memo");
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "groceries"));
    tags.push_back(String::from_str(&env, "monthly"));
    let tx_type = Symbol::new(&env, "expense");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    let transaction = client.get_transaction(&tx_id).unwrap();
    assert_eq!(transaction.id, tx_id);
    assert_eq!(transaction.from, from);
    assert_eq!(transaction.to, to);
    assert_eq!(transaction.amount, amount);
    assert_eq!(transaction.note, note);
    assert_eq!(transaction.memo, memo);
    assert_eq!(transaction.tags.len(), 2);
    assert_eq!(
        transaction.tags.get(0),
        Some(String::from_str(&env, "groceries"))
    );
    assert_eq!(
        transaction.tags.get(1),
        Some(String::from_str(&env, "monthly"))
    );
    assert!(transaction.timestamp > 0);
    assert_eq!(transaction.status, TransactionStatus::Completed);
    assert_eq!(transaction.tx_type, tx_type);
    assert_eq!(transaction.is_public, is_public);
}

#[test]
#[should_panic]
fn test_create_transaction_invalid_amount_zero() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "Invalid amount test");
    let zero_amount: i128 = 0;
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    client.create_transaction(
        &from,
        &to,
        &zero_amount,
        &note,
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
}

#[test]
#[should_panic]
fn test_create_transaction_invalid_amount_negative() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "Invalid amount test");
    let negative_amount: i128 = -100;
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    client.create_transaction(
        &from,
        &to,
        &negative_amount,
        &note,
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
}

#[test]
fn test_update_transaction_note() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let original_note = String::from_str(&env, "Original note");
    let updated_note = String::from_str(&env, "Updated note");
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from,
        &to,
        &amount,
        &original_note,
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    let transaction = client.get_transaction(&tx_id).unwrap();
    assert_eq!(transaction.note, original_note);

    let success = client.update_transaction_note(&tx_id, &from, &updated_note);
    assert!(success);

    let updated_transaction = client.get_transaction(&tx_id).unwrap();
    assert_eq!(updated_transaction.note, updated_note);
}

#[test]
fn test_update_transaction_amount() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let updated_amount: i128 = 1500;
    let note = String::from_str(&env, "Amount update");
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    let success = client.update_transaction_amount(&tx_id, &from, &updated_amount);
    assert!(success);

    let transaction = client.get_transaction(&tx_id).unwrap();
    assert_eq!(transaction.amount, updated_amount);
}

#[test]
#[should_panic]
fn test_transaction_limit_per_user() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "Limit test");
    let one: i128 = 1;
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    for _ in 0..1000 {
        client.create_transaction(&from, &to, &one, &note, &memo, &tags, &tx_type, &is_public);
    }

    client.create_transaction(&from, &to, &one, &note, &memo, &tags, &tx_type, &is_public);
}

#[test]
fn test_get_transaction_timestamp() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Timestamp test");
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    let timestamp = client.get_transaction_timestamp(&tx_id);
    assert!(timestamp.is_some());
    assert!(timestamp.unwrap() > 0);

    let fake_id = Symbol::new(&env, "fake_id");
    let fake_timestamp = client.get_transaction_timestamp(&fake_id);
    assert!(fake_timestamp.is_none());
}

#[test]
fn test_get_user_transactions() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    client.create_transaction(
        &user1,
        &recipient,
        &1000,
        &String::from_str(&env, "User1 transaction 1"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    client.create_transaction(
        &user1,
        &recipient,
        &2000,
        &String::from_str(&env, "User1 transaction 2"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    client.create_transaction(
        &user2,
        &recipient,
        &3000,
        &String::from_str(&env, "User2 transaction"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    let user1_txs = client.get_user_transactions(&user1);
    assert_eq!(user1_txs.len(), 2);

    let user2_txs = client.get_user_transactions(&user2);
    assert_eq!(user2_txs.len(), 1);

    let non_existent_user = Address::generate(&env);
    let empty_txs = client.get_user_transactions(&non_existent_user);
    assert_eq!(empty_txs.len(), 0);
}

#[test]
fn test_clear_user_transactions() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx1_id = client.create_transaction(
        &user,
        &recipient,
        &1000,
        &String::from_str(&env, "Transaction 1"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    let tx2_id = client.create_transaction(
        &user,
        &recipient,
        &2000,
        &String::from_str(&env, "Transaction 2"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    let user_txs = client.get_user_transactions(&user);
    assert_eq!(user_txs.len(), 2);

    let success = client.clear_user_transactions(&user);
    assert!(success);

    let empty_txs = client.get_user_transactions(&user);
    assert_eq!(empty_txs.len(), 0);

    let tx1 = client.get_transaction(&tx1_id);
    assert!(tx1.is_none());
    let tx2 = client.get_transaction(&tx2_id);
    assert!(tx2.is_none());
}

#[test]
fn test_transaction_counter_increments() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx1_id = client.create_transaction(
        &from,
        &to,
        &1000,
        &String::from_str(&env, "Transaction 1"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    let tx2_id = client.create_transaction(
        &from,
        &to,
        &2000,
        &String::from_str(&env, "Transaction 2"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    let tx3_id = client.create_transaction(
        &from,
        &to,
        &3000,
        &String::from_str(&env, "Transaction 3"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    assert_ne!(tx1_id, tx2_id);
    assert_ne!(tx2_id, tx3_id);
    assert_ne!(tx1_id, tx3_id);

    assert!(client.get_transaction(&tx1_id).is_some());
    assert!(client.get_transaction(&tx2_id).is_some());
    assert!(client.get_transaction(&tx3_id).is_some());
}

#[test]
fn test_transaction_exists() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Existence test");
    let memo = String::from_str(&env, "Existence test memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    assert!(client.transaction_exists(&tx_id));

    let fake_id = Symbol::new(&env, "not_here");
    assert!(!client.transaction_exists(&fake_id));
}

#[test]
fn test_create_transaction_stores_creation_timestamp() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    env.ledger().set_timestamp(1_700_000_123);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from,
        &to,
        &500,
        &String::from_str(&env, "timestamped"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    let tx = client.get_transaction(&tx_id).unwrap();
    assert_eq!(tx.timestamp, 1_700_000_123);
    assert_eq!(
        client.get_transaction_timestamp(&tx_id),
        Some(1_700_000_123)
    );
}

#[test]
fn test_get_transaction_memo() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Test transaction");
    let memo = String::from_str(&env, "Important payment memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    // Test get_transaction_memo function
    let retrieved_memo = client.get_transaction_memo(&tx_id).unwrap();
    assert_eq!(retrieved_memo, memo);
}

#[test]
fn test_get_transaction_memo_nonexistent() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let fake_id = Symbol::new(&env, "not_here");

    // Test get_transaction_memo for non-existent transaction
    let memo = client.get_transaction_memo(&fake_id);
    assert!(memo.is_none());
}

#[test]
fn test_delete_transaction_admin_can_remove_record() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Transaction to delete");
    let memo = String::from_str(&env, "Delete memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );
    assert!(client.transaction_exists(&tx_id));

    let success = client.delete_transaction(&admin, &tx_id);
    assert!(success);
    assert!(!client.transaction_exists(&tx_id));
    assert!(client.get_transaction(&tx_id).is_none());
    assert_eq!(client.get_user_transactions(&from).len(), 0);
}

#[test]
#[should_panic]
fn test_delete_transaction_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 1000;
    let note = String::from_str(&env, "Transaction to delete");
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    let tx_id = client.create_transaction(
        &from, &to, &amount, &note, &memo, &tags, &tx_type, &is_public, &Map::new(&env),
    );

    let caller = Address::generate(&env);
    client.delete_transaction(&caller, &tx_id);
}

#[test]
fn test_get_all_transactions() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let memo = String::from_str(&env, "");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    // Create some transactions
    let tx1_id = client.create_transaction(
        &user1,
        &recipient,
        &1000,
        &String::from_str(&env, "Transaction 1"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    let tx2_id = client.create_transaction(
        &user2,
        &recipient,
        &2000,
        &String::from_str(&env, "Transaction 2"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );
    let tx3_id = client.create_transaction(
        &user1,
        &recipient,
        &3000,
        &String::from_str(&env, "Transaction 3"),
        &memo,
        &tags,
        &tx_type,
        &is_public, &Map::new(&env),
    );

    let all_txs = client.get_all_transactions();
    assert_eq!(all_txs.len(), 3);

    // Check that all transactions are present
    let ids: Vec<Symbol> = all_txs.iter().map(|tx| tx.id.clone()).collect();
    assert!(ids.contains(&tx1_id));
    assert!(ids.contains(&tx2_id));
    assert!(ids.contains(&tx3_id));
}

#[test]
fn test_get_all_transactions_empty() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let all_txs = client.get_all_transactions();
    assert_eq!(all_txs.len(), 0);
}

#[test]
fn test_get_transactions_paginated_basic() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "tx");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    for _ in 0..5 {
        client.create_transaction(&from, &to, &100, &note, &memo, &tags, &tx_type, &is_public);
    }

    // fetch all 5
    let page = client.get_transactions_paginated(&0, &10);
    assert_eq!(page.len(), 5);
}

#[test]
fn test_get_transactions_paginated_offset_and_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "tx");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    for i in 1_i128..=10 {
        client.create_transaction(
            &from,
            &to,
            &(i * 10),
            &note,
            &memo,
            &tags,
            &tx_type,
            &is_public, &Map::new(&env),
        );
    }

    // page 1: offset=0, limit=3 → first 3
    let page1 = client.get_transactions_paginated(&0, &3);
    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap().amount, 10);
    assert_eq!(page1.get(2).unwrap().amount, 30);

    // page 2: offset=3, limit=3 → next 3
    let page2 = client.get_transactions_paginated(&3, &3);
    assert_eq!(page2.len(), 3);
    assert_eq!(page2.get(0).unwrap().amount, 40);

    // last page: offset=9, limit=5 → only 1 remaining
    let last = client.get_transactions_paginated(&9, &5);
    assert_eq!(last.len(), 1);
    assert_eq!(last.get(0).unwrap().amount, 100);
}

#[test]
fn test_get_transactions_paginated_offset_beyond_total() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "tx");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;
    client.create_transaction(&from, &to, &100, &note, &memo, &tags, &tx_type, &is_public);

    let page = client.get_transactions_paginated(&10, &5);
    assert_eq!(page.len(), 0);
}

#[test]
fn test_get_transactions_paginated_limit_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "tx");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;
    client.create_transaction(&from, &to, &100, &note, &memo, &tags, &tx_type, &is_public);

    let page = client.get_transactions_paginated(&0, &0);
    assert_eq!(page.len(), 0);
}

#[test]
fn test_get_transactions_paginated_limit_capped_at_100() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "tx");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");
    let is_public = false;

    for _ in 0..120 {
        client.create_transaction(&from, &to, &1, &note, &memo, &tags, &tx_type, &is_public);
    }

    // requesting 200 should be capped to 100
    let page = client.get_transactions_paginated(&0, &200);
    assert_eq!(page.len(), 100);
}

#[test]
fn test_get_user_transactions_filtered_by_tx_type() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let user = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "typed");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let income = Symbol::new(&env, "income");
    let expense = Symbol::new(&env, "expense");
    let is_public = false;

    let income_tx_1 =
        client.create_transaction(&user, &to, &100, &note, &memo, &tags, &income, &is_public);
    let expense_tx =
        client.create_transaction(&user, &to, &50, &note, &memo, &tags, &expense, &is_public);
    let income_tx_2 =
        client.create_transaction(&user, &to, &75, &note, &memo, &tags, &income, &is_public);

    let income_txs = client.get_user_transactions_filtered(&user, &income);
    assert_eq!(income_txs.len(), 2);
    assert_eq!(income_txs.get(0).unwrap().id, income_tx_1);
    assert_eq!(income_txs.get(1).unwrap().id, income_tx_2);

    let expense_txs = client.get_user_transactions_filtered(&user, &expense);
    assert_eq!(expense_txs.len(), 1);
    assert_eq!(expense_txs.get(0).unwrap().id, expense_tx);
}

#[test]
fn test_create_transaction_with_visibility_flag() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let amount: i128 = 500;
    let note = String::from_str(&env, "Visible transaction");
    let memo = String::from_str(&env, "visible memo");
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "public"));
    let tx_type = Symbol::new(&env, "income");

    // 1. Create a public transaction
    let tx_public_id =
        client.create_transaction(&from, &to, &amount, &note, &memo, &tags, &tx_type, &true);
    let tx_public = client.get_transaction(&tx_public_id).unwrap();
    assert_eq!(tx_public.is_public, true);

    // 2. Create a private transaction
    let tx_private_id =
        client.create_transaction(&from, &to, &amount, &note, &memo, &tags, &tx_type, &false);
    let tx_private = client.get_transaction(&tx_private_id).unwrap();
    assert_eq!(tx_private.is_public, false);
}

#[test]
fn test_get_total_expense() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let user = Address::generate(&env);
    let to = Address::generate(&env);
    let note = String::from_str(&env, "note");
    let memo = String::from_str(&env, "memo");
    let tags = Vec::new(&env);
    let income = Symbol::new(&env, "income");
    let expense = Symbol::new(&env, "expense");
    let is_public = false;

    // No transactions yet
    assert_eq!(client.get_total_expense(), 0);

    client.create_transaction(&user, &to, &200, &note, &memo, &tags, &income, &is_public);
    client.create_transaction(&user, &to, &50, &note, &memo, &tags, &expense, &is_public);
    client.create_transaction(&user, &to, &30, &note, &memo, &tags, &expense, &is_public);

    // Only expense amounts should be summed
    assert_eq!(client.get_total_expense(), 80);
}

#[test]
fn test_create_transaction_with_metadata() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");

    let mut metadata = Map::new(&env);
    metadata.set(Symbol::new(&env, "category"), String::from_str(&env, "food"));
    metadata.set(Symbol::new(&env, "ref"), String::from_str(&env, "INV-001"));

    let tx_id = client.create_transaction(
        &from,
        &to,
        &500,
        &String::from_str(&env, "note"),
        &String::from_str(&env, "memo"),
        &tags,
        &tx_type,
        &false,
        &metadata,
    );

    let tx = client.get_transaction(&tx_id).unwrap();
    assert_eq!(tx.metadata.get(Symbol::new(&env, "category")), Some(String::from_str(&env, "food")));
    assert_eq!(tx.metadata.get(Symbol::new(&env, "ref")), Some(String::from_str(&env, "INV-001")));
}

#[test]
fn test_set_and_get_metadata() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");

    let tx_id = client.create_transaction(
        &from,
        &to,
        &100,
        &String::from_str(&env, "note"),
        &String::from_str(&env, "memo"),
        &tags,
        &tx_type,
        &false,
        &Map::new(&env),
    );

    let mut metadata = Map::new(&env);
    metadata.set(Symbol::new(&env, "source"), String::from_str(&env, "mobile"));

    let success = client.set_metadata(&tx_id, &from, &metadata);
    assert!(success);

    let retrieved = client.get_metadata(&tx_id).unwrap();
    assert_eq!(retrieved.get(Symbol::new(&env, "source")), Some(String::from_str(&env, "mobile")));
}

#[test]
fn test_set_metadata_non_owner_fails() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let tags = Vec::new(&env);
    let tx_type = Symbol::new(&env, "transfer");

    let tx_id = client.create_transaction(
        &from,
        &to,
        &100,
        &String::from_str(&env, "note"),
        &String::from_str(&env, "memo"),
        &tags,
        &tx_type,
        &false,
        &Map::new(&env),
    );

    let stranger = Address::generate(&env);
    let mut metadata = Map::new(&env);
    metadata.set(Symbol::new(&env, "key"), String::from_str(&env, "val"));

    let success = client.set_metadata(&tx_id, &stranger, &metadata);
    assert!(!success);
}

#[test]
fn test_get_metadata_nonexistent_returns_none() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TransactionsContract, ());
    let client = TransactionsContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let fake_id = Symbol::new(&env, "ghost");
    assert!(client.get_metadata(&fake_id).is_none());
}
