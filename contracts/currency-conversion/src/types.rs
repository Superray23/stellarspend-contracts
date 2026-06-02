use soroban_sdk::contracttype;

#[contracttype]
pub enum DataKey {
    Rate(soroban_sdk::String),
}

#[contracttype]
#[derive(Clone)]
pub struct ConversionRate {
    pub from_currency: soroban_sdk::String,
    pub to_currency: soroban_sdk::String,
    pub rate_numerator: i128,
    pub rate_denominator: i128,
}
