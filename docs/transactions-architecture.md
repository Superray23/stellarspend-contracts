# Transaction Modules Architecture

This document clarifies the responsibility boundaries, scope, and relationships between the various transaction-related modules and crates in the `stellarspend-contracts` workspace.

## Responsibility Matrix

| Module / Crate Path | Crate Name | Primary Responsibility | Key Functionality |
| :--- | :--- | :--- | :--- |
| `contracts/transactions.rs` | *(Part of root tests)* | **Simulated Ledger & Multi-Sig/Timelock Execution** | Internal balance ledger, multi-signature transaction approval for high-value transactions, timelocked scheduling/execution/cancellation. |
| `contracts/transactions/` | `transactions` | **Transaction Log Database & Metadata Registry** | Creating and querying transaction records, managing transaction metadata maps, user-facing notes, tags, pagination, and status tracking. |
| `contracts/transaction/` | `transaction` | **Mock Transaction Registry with User Limits** | Simple transaction storage enforcing a hardcoded limit of maximum 5 transactions per user, export filtering. |
| `contracts/transactional/` | `transactional` | **Sequential Transaction Ledger with Averages** | Sequential transaction insertion, duplicate ID prevention, average transaction amount calculation, and first-entry retrieval. |
| `contracts/transaction-analytics/` | `transaction-analytics` | **Batch Analytics, Bundling, Fees & Refunds** | O(n) single-pass batch metric generation, transaction bundling, refund processing, audit logging, and contract fee calculation/deduction. |
| `contracts/transaction-validation/` | `transaction-validation` | **Timestamp & Payload Validation** | Timestamp freshness verification and entry-point validation for transaction flows. |
| `contracts/transaction-memo/` | `transaction-memo` | **Size-Bounded Memo Storage** | Length-validated memo text, reference, and payload storage to optimize on-chain storage costs. |

---

## Detailed Module Breakdown

### 1. `contracts/transactions.rs`
* **Type:** File-based contract (loaded via `#[path]` in tests).
* **Role:** Acts as an execution coordinator that simulates an internal ledger. It manages user balances directly and wraps transfers in multi-signature and timelock checks.
* **Key Entities:**
  * `PendingTx`: Holds details of a transaction awaiting multi-sig sign-off.
  * `TimelockedTx`: Holds details of a transaction scheduled for future execution.
* **Ownership Boundary:** Owns the state of internal balances and execution/approval workflows (multi-sig threshold, timelock queue). Does not store rich metadata or categorizations.

### 2. `contracts/transactions/`
* **Type:** Workspace member crate (`transactions`).
* **Role:** Acts as the primary database for transaction history. It handles rich metadata (tags, text notes, arbitrary key-value pairs) and provides robust query APIs.
* **Key Entities:**
  * `Transaction`: The most comprehensive transaction record struct (includes notes, memo, tags, is_public, metadata map, and status).
  * `TransactionStatus`: Enum (`Pending`, `Completed`, `Failed`, `Refunded`).
* **Ownership Boundary:** Owns transaction history logs, user-supplied annotations, categorization tags, and query pagination. It does not perform transfer logic or balance adjustments.

### 3. `contracts/transaction/`
* **Type:** Crate (`transaction`).
* **Role:** A lightweight/mock transaction registry with a strict per-user transaction limit of 5.
* **Key Entities:**
  * `Transaction`: Uses `Symbol` for sender/receiver and `u64` for transaction ID.
* **Ownership Boundary:** Owns the enforcement of user transaction frequency limits (`MAX_TX_PER_USER = 5`) and export helpers.

### 4. `contracts/transactional/`
* **Type:** Workspace member crate (`transactional`).
* **Role:** A sequential ledger tracking transaction insertion order via an incremental counter.
* **Key Entities:**
  * `Transaction`: A minimal 4-field struct (`id`, `amount`, `sender`, `receiver`).
* **Ownership Boundary:** Owns the sequential counter mapping and computation of workspace-wide average transaction amounts.

### 5. `contracts/transaction-analytics/`
* **Type:** Workspace member crate (`transaction-analytics`).
* **Role:** Performs computationally intensive batch analytics, bundling, and auditing operations.
* **Key Entities:**
  * `BatchMetrics`: Aggregated volume, averages, min/max, and unique addresses.
  * `BundledTransaction` / `BundleResult`: Grouping transactions for single-run validations.
  * `RefundRequest` / `RefundResult`: Batch refund processing and metrics.
* **Ownership Boundary:** Owns analytics processing, batch calculations, fee distribution/deduction math, and auditing logs. It consumes transaction lists generated or submitted by external callers.

### 6. `contracts/transaction-validation/`
* **Type:** Workspace member crate (`transaction-validation`).
* **Role:** Entry-point validator verifying transaction timestamps against current ledger boundaries.
* **Ownership Boundary:** Owns timestamp age check rules.

### 7. `contracts/transaction-memo/`
* **Type:** Crate (`transaction-memo`).
* **Role:** Validates and stores user memos, ensuring they fit within strict byte limits to protect contract storage.
* **Key Entities:**
  * `TransactionMemo`: Stores the transaction ID, type, reference, and text content.
* **Ownership Boundary:** Owns memo size and length validations (e.g. total payload sizes).

---

## Architectural Issues & Flagged Duplications

While there is no identical duplicate code (functions copied verbatim), there is significant **conceptual overlap** and **structural redundancy**:

1. **Multiple `Transaction` Structs**:
   There are four separate definitions of a `Transaction` struct across the codebase:
   * `contracts/transactions/src/storage.rs`: Rich struct using `Address` for actors, containing metadata maps, tags, and status.
   * `contracts/transaction-analytics/src/types.rs`: Minimal struct using `Address` for actors, with a `category` field.
   * `contracts/transaction/src/lib.rs`: Minimal struct using `Symbol` for actors, with a `u64` ID and `export` flag.
   * `contracts/transactional/src/lib.rs`: Minimal struct using `Symbol` for actors, with `Symbol` ID.
   
   *Recommendation:* Define a single canonical `Transaction` type in a shared utility crate (e.g., `contracts/shared`) or unify the crates to use a consistent model.

2. **Redundant Transaction Registries**:
   The crates `transactions`, `transaction`, and `transactional` all implement custom transaction store/read registry functionality on-chain.
   * `transactions` acts as the production historical registry.
   * `transaction` acts as a limit-enforcing registry.
   * `transactional` acts as a sequential/averaging registry.
   
   *Recommendation:* Consolidate registry storage into the main `transactions` crate, implementing limits and sequence counters as features/modules within it, rather than deploying three separate registry contracts.

3. **Overlapping High-Value Logic**:
   * `contracts/transactions.rs` implements `high_value_threshold` to route large transfers to multi-sig approvals.
   * `contracts/transaction-analytics/src/analytics.rs` implements `find_high_value_transactions` to flag high-value transfers in processed batches.
   
   *Recommendation:* Keep execution enforcement in the transaction execution layer (`transactions.rs`) and query-based analysis in the analytics layer, but share the threshold configurations if applicable.
