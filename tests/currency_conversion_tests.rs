#![cfg(test)]

use soroban_sdk::{Env, String, Vec};
use currency_conversion::{CurrencyConversionContract, CurrencyConversionContractClient, ConversionRate};

#[test]
fn convert_known_rate() {
    let env = Env::default();
    let contract_id = env.register(CurrencyConversionContract, ());
    let client = CurrencyConversionContractClient::new(&env, &contract_id);

    let rate = ConversionRate {
        from_currency: String::from_str(&env, "XLM"),
        to_currency: String::from_str(&env, "USD"),
        rate_numerator: 105,
        rate_denominator: 100,
    };

    let result = client.convert(&1000, &rate);
    assert_eq!(result, 1050);
}

#[test]
fn convert_zero_amount() {
    let env = Env::default();
    let contract_id = env.register(CurrencyConversionContract, ());
    let client = CurrencyConversionContractClient::new(&env, &contract_id);

    let rate = ConversionRate {
        from_currency: String::from_str(&env, "XLM"),
        to_currency: String::from_str(&env, "USD"),
        rate_numerator: 105,
        rate_denominator: 100,
    };

    let result = client.convert(&0, &rate);
    assert_eq!(result, 0);
}

#[test]
fn normalize_two_currencies() {
    let env = Env::default();
    let contract_id = env.register(CurrencyConversionContract, ());
    let client = CurrencyConversionContractClient::new(&env, &contract_id);

    let mut balances = Vec::new(&env);
    balances.push_back((String::from_str(&env, "XLM"), 1000));
    balances.push_back((String::from_str(&env, "USD"), 500));

    let mut rates = Vec::new(&env);
    rates.push_back(ConversionRate {
        from_currency: String::from_str(&env, "XLM"),
        to_currency: String::from_str(&env, "USD"),
        rate_numerator: 105,
        rate_denominator: 100,
    });

    let result = client.normalize(&balances, &rates, &String::from_str(&env, "USD"));
    assert_eq!(result, 1550);
}

#[test]
fn normalize_single_base_currency() {
    let env = Env::default();
    let contract_id = env.register(CurrencyConversionContract, ());
    let client = CurrencyConversionContractClient::new(&env, &contract_id);

    let mut balances = Vec::new(&env);
    balances.push_back((String::from_str(&env, "USD"), 500));

    let rates = Vec::new(&env);

    let result = client.normalize(&balances, &rates, &String::from_str(&env, "USD"));
    assert_eq!(result, 500);
}

#[test]
fn convert_same_numerator_denominator() {
    let env = Env::default();
    let contract_id = env.register(CurrencyConversionContract, ());
    let client = CurrencyConversionContractClient::new(&env, &contract_id);

    let rate = ConversionRate {
        from_currency: String::from_str(&env, "XLM"),
        to_currency: String::from_str(&env, "USD"),
        rate_numerator: 100,
        rate_denominator: 100,
    };

    let result = client.convert(&1000, &rate);
    assert_eq!(result, 1000);
}
