use soroban_sdk::{String, Vec};
use crate::types::ConversionRate;

pub fn convert_amount(amount: i128, numerator: i128, denominator: i128) -> i128 {
    if amount == 0 {
        return 0;
    }
    if denominator == 0 {
        panic!("division by zero in conversion");
    }
    amount
        .checked_mul(numerator)
        .expect("overflow in conversion")
        .checked_div(denominator)
        .expect("division by zero in conversion")
}

pub fn normalize_balances(
    balances: Vec<(String, i128)>,
    rates: Vec<ConversionRate>,
    base_currency: String,
) -> i128 {
    let mut total: i128 = 0;

    for item in balances.iter() {
        let (currency, amount) = item;
        if currency == base_currency {
            total = total.checked_add(amount).expect("overflow in normalization");
        } else {
            let mut rate_found = false;
            for rate in rates.iter() {
                if rate.from_currency == currency && rate.to_currency == base_currency {
                    let converted = convert_amount(amount, rate.rate_numerator, rate.rate_denominator);
                    total = total.checked_add(converted).expect("overflow in normalization");
                    rate_found = true;
                    break;
                }
            }
            if !rate_found {
                panic!("no rate found for currency");
            }
        }
    }
    total
}
