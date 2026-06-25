# `user` vs `users` Crates — Distinction and Ownership

## Summary

Two crates exist under `contracts/` with overlapping names and similar surface areas:
`contracts/user` and `contracts/users`. This document clarifies their roles, explains why
both exist, and establishes which one owns user-related logic going forward.

---

## `contracts/users` — Canonical implementation ✅

**This is the authoritative crate for all user-related logic.**

| Feature | Status |
|---|---|
| Admin initialisation | ✅ |
| `register_user` (idempotent, returns `bool`) | ✅ |
| User count | ✅ |
| Deactivate user | ✅ |
| Profile update (currency, nickname) | ✅ |
| Last-login timestamp | ✅ |
| On-chain events | ✅ |
| Dedicated `storage` module | ✅ |
| Inline tests (`mod test`) | ✅ |

### Key files
- `contracts/users/src/lib.rs` — contract entry point and public API
- `contracts/users/src/storage.rs` — all persistent storage helpers
- `contracts/users/src/test.rs` — unit and integration tests

**New features, bug-fixes, and cross-contract integrations should target this crate.**

---

## `contracts/user` — Legacy stub ⚠️ (deprecated)

This crate is a minimal, early-stage stub that predates the full `users` implementation.

| Feature | Status |
|---|---|
| `register_user` (panics on duplicate, no return value) | ✅ (limited) |
| `user_exists` | ✅ |
| Admin initialisation | ❌ |
| Events | ❌ |
| Storage module | ❌ |
| Tests | ❌ |

It uses a bare string constant (`"USERS"`) as a storage key and holds no profile data.

**This crate is deprecated.** It is retained only for historical reference and must not
receive new features or be depended upon by other contracts.

---

## Decision

| Concern | Owner |
|---|---|
| Canonical user registry | `contracts/users` |
| User profile data | `contracts/users` |
| Cross-contract user checks | `contracts/users` |
| Legacy reference only | `contracts/user` |

### Why not delete `contracts/user` immediately?

Deletion is out of scope for this issue (see issue #712 acceptance criteria: _no change to
storage layout_). The stub crate is kept to avoid breaking any downstream reference that
may still point to it. A follow-up issue should be raised to remove it once all dependents
have been confirmed clear.

---

## How to test

```bash
# Canonical crate — must pass
cargo test -p users

# Legacy stub — must pass (existing behaviour preserved)
cargo test -p user
```
