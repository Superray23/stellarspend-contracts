use crate::SharedDataKey;
use soroban_sdk::{Address, Env};

// ========== ADMIN ROLE CHECK (Issue #337) ==========

/// Check if the given address is the admin
pub fn is_admin(env: &Env, caller: &Address) -> bool {
    let admin: Address = env.storage().instance().get(&SharedDataKey::Admin).unwrap();
    caller == &admin
}

/// Require that the caller is admin, otherwise panic
pub fn require_admin(env: &Env, caller: &Address) {
    assert!(is_admin(env, caller), "not authorized: admin only");
}

/// Generic version of require_admin that uses a provided error type
pub fn require_admin_with_error<E>(env: &Env, caller: &Address, admin: &Address, error: E)
where
    E: Into<soroban_sdk::Error>,
{
    if caller != admin {
        env.panic_with_error(error.into());
    }
    caller.require_auth();
}
