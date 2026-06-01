use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, Symbol,
};

use crate::types::CategorySpend;
use crate::{CategoryAnalytics, CategoryAnalyticsClient};
// handle hash
fn setup_analytics_contract() -> (Env, Address, CategoryAnalyticsClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CategoryAnalytics, ());
    let client = CategoryAnalyticsClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    (env, admin, client)
}

#[test]
fn test_integration_spending_trends() {
    let (env, _admin, client) = setup_analytics_contract();
    let user = Address::generate(&env);
    let category = Symbol::new(&env, "shopping");

    // Year/Month calculation in contract:
    // 2026: 1768608000
    env.ledger().set_timestamp(1768608000);

    client.record_spending(&user, &category, &5000);
    client.record_spending(&user, &category, &3000);

    let metrics = client.get_category_metrics(&user, &category, &2026, &2);
    assert_eq!(metrics.volume, 8000);
    assert_eq!(metrics.count, 2);

    // Advance time to March (approx 30 days)
    env.ledger().set_timestamp(1768608000 + 2592000);
    client.record_spending(&user, &category, &2000);

    let march_metrics = client.get_category_metrics(&user, &category, &2026, &3);
    assert_eq!(march_metrics.volume, 2000);

    let yearly = client.get_yearly_trend(&user, &category, &2026);
    assert_eq!(yearly.volume, 10000);
    assert_eq!(yearly.count, 3);
}

#[test]
fn test_multi_category_batch_aggregation() {
    let (env, _admin, client) = setup_analytics_contract();
    let user = Address::generate(&env);
    let shopping = Symbol::new(&env, "shopping");
    let groceries = Symbol::new(&env, "groceries");
    let transport = Symbol::new(&env, "transport");

    env.ledger().set_timestamp(1768608000);

    let spendings = vec![
        &env,
        CategorySpend {
            category: shopping.clone(),
            amount: 5000,
        },
        CategorySpend {
            category: groceries.clone(),
            amount: 2500,
        },
        CategorySpend {
            category: shopping.clone(),
            amount: 1500,
        },
        CategorySpend {
            category: transport.clone(),
            amount: 900,
        },
    ];

    client.record_spending_batch(&user, &spendings);

    let shopping_metrics = client.get_category_metrics(&user, &shopping, &2026, &2);
    assert_eq!(shopping_metrics.volume, 6500);
    assert_eq!(shopping_metrics.count, 2);

    let groceries_metrics = client.get_category_metrics(&user, &groceries, &2026, &2);
    assert_eq!(groceries_metrics.volume, 2500);
    assert_eq!(groceries_metrics.count, 1);

    let transport_metrics = client.get_category_metrics(&user, &transport, &2026, &2);
    assert_eq!(transport_metrics.volume, 900);
    assert_eq!(transport_metrics.count, 1);

    let shopping_current = client.get_current_spending(&user, &shopping);
    assert_eq!(shopping_current.volume, 6500);
    assert_eq!(shopping_current.count, 2);

    let shopping_yearly = client.get_yearly_trend(&user, &shopping, &2026);
    assert_eq!(shopping_yearly.volume, 6500);
    assert_eq!(shopping_yearly.count, 2);
}
