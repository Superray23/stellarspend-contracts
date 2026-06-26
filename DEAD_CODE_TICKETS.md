# Dead Code Tickets — Clippy Unused-Code Sweep

Generated: 2026-06-26  
Tool: static analysis (cargo not available in CI environment at time of sweep)

---

## Summary

| Category | Count |
|---|---|
| Orphan `.rs` files in compiled crates (never declared via `mod`) | 39 |
| Orphan loose `contracts/*.rs` files (workspace non-member) | 39 |
| Unused import fixes applied | 1 |
| **Total findings** | **79** |

---

## Applied Fixes

| File | Change |
|---|---|
| `contracts/notification/src/lib.rs` | Removed unused `Address` import from `soroban_sdk` group |

---

## Ticket: `contracts/` directory is not a workspace member

**Crate path:** `contracts/` (root)  
**Severity:** High — ~39 loose `.rs` files are never compiled  
**Affected files:** all `contracts/*.rs` except those declared in `contracts/lib.rs`

The `contracts/` directory is not listed in the workspace `Cargo.toml` `[workspace] members`. Its `lib.rs` only declares 5 modules (`delegation`, `errors`, `fees`, `multisig_savings_withdrawal`, `multisig_savings_withdrawal_utils`). All other `.rs` files in this directory are dead and invisible to the compiler.

**Recommended action:** Decide which files are candidates to become their own crates (most already have crate equivalents), then either delete the loose files or create a `contracts/Cargo.toml` and declare the remaining modules. Priority: audit for duplication with existing workspace crates.

### Orphan loose files in `contracts/`

| File | Notes |
|---|---|
| `contracts/account_status.rs` | Superseded by `contracts/access-control` crate |
| `contracts/archive.rs` | References `history.rs` via `#[path]`; both are orphaned |
| `contracts/batch.rs` | Likely superseded by `batch-payment`, `batch-transfer`, `batch-rewards` crates |
| `contracts/budget.rs` | Superseded by `contracts/budget` crate |
| `contracts/compliance.rs` | No crate equivalent found; evaluate for inclusion or deletion |
| `contracts/conditional_payment.rs` | No crate equivalent; large file (~26 KB); ticket for crate extraction |
| `contracts/contract.rs` | Stub CosmWasm entry-point fragment; not Soroban-compatible |
| `contracts/conversion.rs` | Superseded by `contracts/currency-conversion` crate |
| `contracts/dependencies.rs` | Contains only type aliases for external crates; likely dead |
| `contracts/fraud.rs` | No crate equivalent; evaluate for extraction |
| `contracts/gas-optimization.rs` | Reference/benchmark code; not a deployable contract |
| `contracts/governance.rs` | No crate equivalent; evaluate for extraction |
| `contracts/history.rs` | Referenced by `archive.rs` via `#[path]`; both orphaned |
| `contracts/memo.rs` | Superseded by `contracts/transaction-memo` crate |
| `contracts/metadata.rs` | Uses non-Soroban `drip_sdk` framework; incompatible |
| `contracts/msg.rs` | CosmWasm `ExecuteMsg` fragment; not Soroban-compatible |
| `contracts/multisig.rs` | Referenced by `transactions.rs` via `#[path]`; whole group orphaned |
| `contracts/multisig_savings_integration.rs` | No crate equivalent; evaluate for extraction |
| `contracts/overdraft.rs` | No crate equivalent; large file (~20 KB); ticket for crate extraction |
| `contracts/prediction.rs` | Uses old Soroban API (`env.storage().get/set`); outdated |
| `contracts/preference.rs` | No crate equivalent; large file (~18 KB); ticket for crate extraction |
| `contracts/priority.rs` | No crate equivalent; evaluate for extraction |
| `contracts/rate_limit.rs` | No crate equivalent; evaluate for extraction |
| `contracts/recurring_savings.rs` | No crate equivalent; large file (~18 KB); ticket for crate extraction |
| `contracts/refunds.rs` | No crate equivalent; large file (~15 KB); ticket for crate extraction |
| `contracts/rewards.rs` | Superseded by `contracts/batch-rewards` crate |
| `contracts/savings.rs` | Superseded by `contracts/savings-goals` and `contracts/savings` crates |
| `contracts/simulation.rs` | Not Soroban-based (references `crate::transactions`); incompatible |
| `contracts/snapshots.rs` | No crate equivalent; evaluate for extraction |
| `contracts/state.rs` | CosmWasm `cw_storage_plus` code; not Soroban-compatible |
| `contracts/streak_reward.rs` | No crate equivalent; large file (~20 KB); ticket for crate extraction |
| `contracts/throttling.rs` | No crate equivalent; large file (~20 KB); ticket for crate extraction |
| `contracts/timelock.rs` | Referenced by `transactions.rs` via `#[path]`; whole group orphaned |
| `contracts/token.rs` | No crate equivalent; large file (~22 KB); ticket for crate extraction |
| `contracts/transaction_metadata.rs` | No crate equivalent; evaluate for extraction |
| `contracts/transactions.rs` | Uses `#[path]` to load `multisig.rs` and `timelock.rs`; all three orphaned |
| `contracts/utils.rs` | Empty file (0 bytes) |
| `contracts/wallet.rs` | No crate equivalent; large file (~15 KB); ticket for crate extraction |
| `contracts/wallet_linking.rs` | No crate equivalent; evaluate for extraction |

---

## Ticket: `contracts/batch-rewards` — orphan src files

**Crate:** `contracts/batch-rewards`  
**Declared modules in `src/lib.rs`:** `types`, `validation`, `test`

| Orphan file | Recommendation |
|---|---|
| `contracts/batch-rewards/src/decay.rs` | Contains fee decay logic; evaluate whether to declare as `mod decay` in lib.rs or delete |
| `contracts/batch-rewards/src/gas.rs` | Duplicate of `contracts/batch-rewards/utils.rs` (outside `src/`); delete one, declare the other |

**Note:** `contracts/batch-rewards/utils.rs` and `contracts/batch-rewards/cross.rs` are also outside `src/` and not declared anywhere — they are loose sibling files with no crate path. Add to the delete/extract backlog.

---

## Ticket: `contracts/budget` — orphan src files

**Crate:** `contracts/budget`  
**Declared modules in `src/lib.rs`:** `storage`, `types`, `test`

| Orphan file | Recommendation |
|---|---|
| `contracts/budget/src/budge.rs` | Typo filename; appears to be a stub; delete |
| `contracts/budget/src/lib_feature.rs` | Large (~18 KB) feature module; likely a split-out draft of `lib.rs`; merge or declare as `mod lib_feature` |
| `contracts/budget/src/lib_main.rs` | Large (~52 KB) main module; appears to be an alternate `lib.rs`; consolidate |
| `contracts/budget/src/libs.rs` | Helper module; evaluate for declaration or merge |
| `contracts/budget/src/merge.rs` | Merge utility; evaluate for declaration |
| `contracts/budget/src/pause.rs` | Pause logic; evaluate for declaration as `mod pause` |

---

## Ticket: `contracts/escrow` — orphan src files

**Crate:** `contracts/escrow`  
**Declared modules in `src/lib.rs`:** `types`, `validation`, `test`

| Orphan file | Recommendation |
|---|---|
| `contracts/escrow/src/gas.rs` | Gas optimization helpers; declare as `mod gas` or delete |

---

## Ticket: `contracts/events` — orphan src files

**Crate:** `contracts/events`  
**Declared modules in `src/lib.rs`:** `events` (via `use events::`)

| Orphan file | Recommendation |
|---|---|
| `contracts/events/src/calculate_fee.rs` | Fee calculation helper; declare or delete |
| `contracts/events/src/common.rs` | Empty file (0 bytes); delete |
| `contracts/events/src/gas.rs` | Gas event helpers; declare or delete |

---

## Ticket: `contracts/fee` — orphan src files

**Crate:** `contracts/fee`  
**Declared modules in `src/lib.rs`:** `auth`, `decay`, `escrow`, `events`, `fee_validation`, `reconciliation`, `storage`, `utils`, `validation`

| Orphan file | Recommendation |
|---|---|
| `contracts/fee/src/bps_converter.rs` | BPS conversion helpers; functionality overlaps `utils.rs`; merge or delete |
| `contracts/fee/src/debug_log.rs` | Debug logging stub; `log_fee_event` is a no-op; delete |
| `contracts/fee/src/fee_percentage.rs` | Overlaps `bps_converter.rs`; consolidate and delete |
| `contracts/fee/src/history_test.rs` | Test file not declared; add `#[cfg(test)] mod history_test;` or merge into `test.rs` |
| `contracts/fee/src/max_fee.rs` | Thin wrapper around `storage::get_max_fee`; inline or delete |
| `contracts/fee/src/min_fee.rs` | Thin wrapper around `storage::get_min_fee`; inline or delete |
| `contracts/fee/src/negative_check.rs` | `check_not_negative` duplicates `shared::utils::validate_amount`; delete, use shared |
| `contracts/fee/src/safe_sub.rs` | `safe_sub` duplicates stdlib `checked_sub`; delete |

---

## Ticket: `contracts/goals` — orphan src files

**Crate:** `contracts/goals` (not in workspace `Cargo.toml`)  
**Status:** Entire crate is outside the workspace; none of its files are compiled

| Orphan file | Recommendation |
|---|---|
| `contracts/goals/src/goal.rs` | Add crate to workspace or delete |
| `contracts/goals/src/storage.rs` | Add crate to workspace or delete |
| `contracts/goals/src/tests.rs` | Add crate to workspace or delete |

---

## Ticket: `contracts/multi-currency-wallet` — orphan src files

**Crate:** `contracts/multi-currency-wallet`  
**Declared modules in `src/lib.rs`:** `types`, `validation`, `test`

| Orphan file | Recommendation |
|---|---|
| `contracts/multi-currency-wallet/src/auth.rs` | Auth helper; declare as `mod auth` in lib.rs |

---

## Ticket: `contracts/notification` — orphan src files

**Crate:** `contracts/notification` (not in workspace `Cargo.toml`)

| Orphan file | Recommendation |
|---|---|
| `contracts/notification/src/budget_notifier.rs` | References `crate::notifications::events` which doesn't exist; broken path |
| `contracts/notification/src/events.rs` | Not declared in lib.rs |

**Note:** `notification/src/lib.rs` also contains a `use` statement embedded inside an `impl` block (line ~60), which is a syntax error. The file cannot compile. File needs structural repair before it can be added to the workspace.

---

## Ticket: `contracts/savings` — orphan src files

**Crate:** `contracts/savings` (not in workspace `Cargo.toml`)  
**Status:** Entire crate is outside the workspace; none of its files are compiled

| Orphan file | Recommendation |
|---|---|
| `contracts/savings/src/limits.rs` | Add crate to workspace or delete |
| `contracts/savings/src/rewards.rs` | Add crate to workspace or delete |
| `contracts/savings/src/storage.rs` | Add crate to workspace or delete |
| `contracts/savings/src/tests.rs` | Empty file; delete |
| `contracts/savings/src/types.rs` | Add crate to workspace or delete |

---

## Ticket: `contracts/shared` — orphan src files

**Crate:** `contracts/shared`  
**Declared modules in `src/lib.rs`:** `assets`, `auth`, `errors`, `sanitizer`, `utils`, `validation`

| Orphan file | Recommendation |
|---|---|
| `contracts/shared/src/events.rs` | Not declared; contains `emit_event` helper; declare as `mod events` if needed, else delete |

---

## Ticket: `contracts/spending-limits` — orphan src files

**Crate:** `contracts/spending-limits`  
**Declared modules in `src/lib.rs`:** `cross_contract`, `types`, `validation`, `test`

| Orphan file | Recommendation |
|---|---|
| `contracts/spending-limits/src/errors.rs` | Error types not imported; declare as `mod errors` or merge into `types.rs` |
| `contracts/spending-limits/src/storage.rs` | Storage helpers not imported; declare as `mod storage` or inline |

---

## Ticket: `contracts/src` — orphan src files

**Crate path:** `contracts/src` (not a workspace member)  
**Status:** The `contracts/src/` directory has its own `Cargo.toml` but is not in workspace `members`

| Orphan file | Recommendation |
|---|---|
| `contracts/src/analytics_events.rs` | Not declared; add crate to workspace or delete |
| `contracts/src/events.rs` | Not declared; add crate to workspace or delete |

---

## Ticket: `contracts/transact` — orphan src files

**Crate:** `contracts/transact` (not in workspace `Cargo.toml`)  
**Status:** Entire crate is outside the workspace

| Orphan file | Recommendation |
|---|---|
| `contracts/transact/src/libss.rs` | Not declared; add crate to workspace or delete |
| `contracts/transact/src/trans.rs` | Not declared; add crate to workspace or delete |

---

## Ticket: `contracts/wallet-status` — orphan src files

**Crate:** `contracts/wallet-status` (not in workspace `Cargo.toml`)

| Orphan file | Recommendation |
|---|---|
| `contracts/wallet-status/src/test.rs` | Not declared in lib.rs; declare as `#[cfg(test)] mod test;` |

---

## Ticket: Crates missing from workspace `Cargo.toml`

The following crates have `Cargo.toml` files but are absent from the workspace `[members]` list, meaning they are never built or tested by `cargo build/test --workspace`:

| Crate path | Notes |
|---|---|
| `contracts/src` | Has Cargo.toml; large fee.rs (~58 KB); likely old implementation |
| `contracts/goals` | Has Cargo.toml; small crate |
| `contracts/savings` | Has Cargo.toml; split-out savings logic |
| `contracts/notification` | Has Cargo.toml; has syntax error in lib.rs |
| `contracts/transact` | Has Cargo.toml; thin wrapper crate |
| `contracts/wallet-status` | Has Cargo.toml; small crate |
| `contracts/activity-feed` | Has src/ but no Cargo.toml in directory listing |
| `contracts/treasury` | Has src/ but no Cargo.toml |
| `contracts/transaction` | Has src/ but no Cargo.toml |
| `contracts/user` | Has src/ but no Cargo.toml |

**Recommended action:** Audit each crate; add to workspace or remove from repository.

---

## CI Note

Cargo is not installed in the dev container. The CI `clippy` step in `.github/workflows/contract-ci.yml` runs against the workspace members listed above. Orphan files outside declared `mod` trees produce **zero clippy warnings** — they are simply never seen by the compiler. To surface these as errors, add a CI step:

```bash
# Detect undeclared .rs files in src/ directories
for dir in $(find contracts -name 'src' -type d); do
  lib="$dir/lib.rs"
  [ -f "$lib" ] || continue
  declared=$(grep -oP '(?<=^mod |^pub mod )\w+' "$lib")
  for f in "$dir"/*.rs; do
    base=$(basename "$f" .rs)
    [ "$base" = "lib" ] && continue
    echo "$declared" | grep -qw "$base" || echo "UNDECLARED: $f"
  done
done
```
