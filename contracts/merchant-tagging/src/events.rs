//! Event emitters for the merchant-tagging contract.

use soroban_sdk::{symbol_short, Address, Env, Symbol};

/// Emitted when a new merchant is registered.
pub fn emit_merchant_registered(env: &Env, merchant_id: &Symbol, registrar: &Address) {
    env.events().publish(
        (symbol_short!("merchant"), symbol_short!("regd")),
        (merchant_id.clone(), registrar.clone()),
    );
}

/// Emitted when a merchant record is updated.
pub fn emit_merchant_updated(env: &Env, merchant_id: &Symbol, updater: &Address) {
    env.events().publish(
        (symbol_short!("merchant"), symbol_short!("updated")),
        (merchant_id.clone(), updater.clone()),
    );
}

/// Emitted when a merchant is deactivated.
pub fn emit_merchant_deactivated(env: &Env, merchant_id: &Symbol, caller: &Address) {
    env.events().publish(
        (symbol_short!("merchant"), symbol_short!("deact")),
        (merchant_id.clone(), caller.clone()),
    );
}

/// Emitted when a transaction is tagged with a merchant.
pub fn emit_transaction_tagged(
    env: &Env,
    tx_id: u64,
    merchant_id: &Symbol,
    amount: i128,
    asset: &Symbol,
) {
    env.events().publish(
        (symbol_short!("merchant"), symbol_short!("tx_tagged")),
        (tx_id, merchant_id.clone(), amount, asset.clone()),
    );
}

/// Emitted when a transaction tag is removed.
pub fn emit_tag_removed(env: &Env, tx_id: u64, merchant_id: &Symbol) {
    env.events().publish(
        (symbol_short!("merchant"), symbol_short!("tag_rm")),
        (tx_id, merchant_id.clone()),
    );
}
