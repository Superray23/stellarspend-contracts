#![cfg(test)]

use crate::assets::normalize_asset_symbol;
use soroban_sdk::{Env, Symbol};

#[test]
fn test_normalize_asset_symbol() {
    let env = Env::default();

    // Already normalized
    let sym1 = Symbol::new(&env, "USDC");
    assert_eq!(
        normalize_asset_symbol(&env, &sym1),
        Symbol::new(&env, "USDC")
    );

    // Lowercase
    let sym2 = Symbol::new(&env, "usdc");
    assert_eq!(
        normalize_asset_symbol(&env, &sym2),
        Symbol::new(&env, "USDC")
    );

    // Mixed case
    let sym3 = Symbol::new(&env, "UsDc");
    assert_eq!(
        normalize_asset_symbol(&env, &sym3),
        Symbol::new(&env, "USDC")
    );
}
