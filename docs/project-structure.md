# Project Directory Structure

This document outlines the codebase layout and project structure for the `stellarspend-contracts` repository, clarifying the relationship between top-level contract files, sub-crates, and the test suite.

---

## Workspace Layout Overview

The repository is structured as a Cargo workspace containing two distinct types of Rust code layouts:
1. **Loose Top-Level Contract Files** (under `contracts/`) compiled directly via integration tests.
2. **Standard Workspace Sub-Crates** (in subdirectories of `contracts/`) compiled as standalone WASM contracts.

```
stellarspend-contracts/
├── Cargo.toml (Workspace Root Cargo config)
├── docs/ (Documentation files)
├── tests/ (Workspace-level integration tests)
├── contracts/
│   ├── lib.rs
│   ├── errors.rs
│   ├── transactions.rs
│   ├── ... (other loose .rs files)
│   │
│   ├── balance/ (Workspace member crate)
│   ├── escrow/ (Workspace member crate)
│   ├── stellarspend-contracts/ (Workspace member crate)
│   ├── stellarspend-fee/ (Workspace member crate - formerly contracts/src)
│   └── ... (other sub-crates)
```

---

## 1. Loose Top-Level Contract Files (`contracts/*.rs`)

Files directly inside `contracts/` (such as `contracts/lib.rs`, `contracts/errors.rs`, `contracts/transactions.rs`, etc.) are **not** organized as standard standalone Cargo crates. They do not have a `Cargo.toml` in their parent folder.

Instead:
* They are included in the workspace root package `stellarspend-contracts-tests`.
* They are compiled and tested directly by the integration tests located in `tests/` (e.g., `tests/error_tests.rs` and `tests/multisig_tests.rs`) using path attributes:
  ```rust
  #[path = "../contracts/errors.rs"]
  mod errors;

  #[path = "../contracts/lib.rs"]
  mod lib;
  ```
* This structure allows code sharing and mock testing of contract logic without declaring them as separate crates.

---

## 2. Workspace Sub-Crates (`contracts/<crate-name>/`)

Subdirectories under `contracts/` containing a `Cargo.toml` are individual Cargo packages. They are registered as workspace members in the root `Cargo.toml` and are compiled to standalone WASM targets for deployment on Soroban.

Examples:
* `contracts/balance`
* `contracts/transaction-analytics`
* `contracts/stellarspend-fee`

---

## 3. Misplaced Sub-Crate: `contracts/src/` Renamed to `contracts/stellarspend-fee/`

Previously, a sub-crate named `stellarspend-fee` was located in `contracts/src/`. This directory structure was confusing because:
1. `src/` is conventionally the folder *containing* source files for a crate, not the parent directory of a crate itself.
2. The folder sat alongside the loose `.rs` files under `contracts/`, obscuring its relationship to `contracts/lib.rs` and the other sub-crates.
3. It was not registered as a member in the workspace `Cargo.toml`.

### Resolution
* The `contracts/src/` directory has been renamed to **`contracts/stellarspend-fee/`**.
* The crate is now registered as a workspace member in the root `Cargo.toml` under `"contracts/stellarspend-fee"`.
* References to its files from other crates (e.g., in `contracts/stellarspend-contracts/src/lib.rs`) have been updated from `../../src/` to `../../stellarspend-fee/`.
