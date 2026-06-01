use alloc::string::ToString;
use soroban_sdk::{Env, Symbol};

/// Normalizes an asset symbol by converting it to uppercase
pub fn normalize_asset_symbol(env: &Env, symbol: &Symbol) -> Symbol {
    let sym_string = symbol.to_string();
    let upper = sym_string.to_uppercase();
    Symbol::new(env, &upper)
}
