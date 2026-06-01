//! Data types and events for batch transaction analytics.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct FeeRecipientShare {
    pub recipient: Address,
    pub share_bps: u32,
}

pub const MAX_BATCH_SIZE: u32 = 100;
pub const MAX_PAGE_SIZE: u32 = 100;

#[derive(Clone, Debug)]
#[contracttype]
pub struct Transaction {
    pub tx_id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub category: Symbol,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct AuditLog {
    pub actor: Address,
    pub operation: Symbol,
    pub timestamp: u64,
    pub status: Symbol,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct BatchMetrics {
    pub tx_count: u32,
    pub total_volume: i128,
    pub avg_amount: i128,
    pub min_amount: i128,
    pub max_amount: i128,
    pub unique_senders: u32,
    pub unique_recipients: u32,
    pub total_fees: i128,
    pub processed_at: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CategoryMetrics {
    pub category: Symbol,
    pub tx_count: u32,
    pub total_volume: i128,
    pub total_fees: i128,
    pub volume_percentage_bps: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BundledTransaction {
    pub transaction: Transaction,
    pub memo: Option<Symbol>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct ValidationResult {
    pub tx_id: u64,
    pub is_valid: bool,
    pub error: Symbol,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BundleResult {
    pub bundle_id: u64,
    pub total_count: u32,
    pub valid_count: u32,
    pub invalid_count: u32,
    pub validation_results: Vec<ValidationResult>,
    pub can_bundle: bool,
    pub total_volume: i128,
    pub created_at: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct RatingInput {
    pub tx_id: u64,
    pub score: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub enum RatingStatus {
    Success,
    InvalidScore,
    UnknownTransaction,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct RatingResult {
    pub tx_id: u64,
    pub score: u32,
    pub status: RatingStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct TransactionStatusUpdate {
    pub tx_id: u64,
    pub status: TransactionStatus,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct StatusUpdateResult {
    pub tx_id: u64,
    pub is_valid: bool,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchStatusUpdateResult {
    pub total_requests: u32,
    pub successful: u32,
    pub failed: u32,
    pub results: Vec<StatusUpdateResult>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct PaginatedBatchMetrics {
    pub metrics: Vec<BatchMetrics>,
    pub total_count: u32,
    pub page_number: u32,
    pub page_size: u32,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    LastBatchId,
    BatchMetrics(u64),
    TotalTxProcessed,
    AuditLog(u64),
    TotalAuditLogs,
    LastBundleId,
    BundleResult(u64),
    LastRefundBatchId,
    RefundBatchMetrics(u64),
    TotalRefundAmount,
    RefundedTransactions,
    KnownTransaction(u64),
    Rating(u64, Address),
    TransactionStatus(u64),
    MonthlyAnalytics(u32, u32, Address),
    UserSpendingSummary(Address),
    TotalTrackedUsers,
    LastAnalyticsUpdate,
    CurrentFeeConfig,
    OperationFeeConfig(Symbol),
    FeePaused,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum RefundStatus {
    Eligible,
    AlreadyRefunded,
    Pending,
    NotEligible,
    NotFound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ValidationError {
    InvalidAddress,
    InvalidAmount,
    InvalidTimestamp,
    InvalidCategory,
    InvalidTransactionId,
    InvalidReason,
    InvalidRating,
    InvalidMemo,
    InvalidYear,
    InvalidMonth,
    InvalidPercentage,
    SameAddress,
    EmptyBatch,
    BatchTooLarge,
    DuplicateTransactionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum FeeModel {
    Flat(i128),
    Percentage(u32),
    Tiered(Vec<FeeTier>),
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct FeeTier {
    pub threshold: i128,
    pub fee_model: FeeModel,
    pub default_percentage_bps: u32,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct FeeConfig {
    pub fee_model: FeeModel,
    pub min_fee: Option<u64>,
    pub max_fee: Option<u64>,
    pub enabled: bool,
    pub description: Option<Symbol>,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct FeeCalculationResult {
    pub gross_amount: i128,
    pub fee_amount: i128,
    pub net_amount: i128,
    pub fee_percentage_bps: u32,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct FeeDeductionEvent {
    pub timestamp: u64,
    pub gross_amount: i128,
    pub fee_amount: i128,
    pub net_amount: i128,
    pub fee_percentage_bps: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct RefundRequest {
    pub tx_id: u64,
    pub reason: Option<Symbol>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct RefundResult {
    pub tx_id: u64,
    pub success: bool,
    pub status: RefundStatus,
    pub amount_refunded: i128,
    pub error_message: Option<Symbol>,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct RefundBatchMetrics {
    pub request_count: u32,
    pub successful_refunds: u32,
    pub failed_refunds: u32,
    pub total_refunded_amount: i128,
    pub avg_refund_amount: i128,
    pub processed_at: u64,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct MonthlySpendingAnalytics {
    pub year: u32,
    pub month: u32,
    pub user: Address,
    pub total_spending: i128,
    pub category_spending: Vec<(Symbol, i128)>,
    pub transaction_count: u32,
}

#[derive(Clone, Debug, Default)]
#[contracttype]
pub struct UserSpendingSummary {
    pub user: Address,
    pub total_spending: i128,
    pub total_transactions: u32,
    pub primary_category: Symbol,
    pub avg_monthly_spending: i128,
}

pub struct AnalyticsEvents;

impl AnalyticsEvents {
    pub fn batch_processed(env: &Env, batch_id: u64, metrics: &BatchMetrics) {
        let topics = (symbol_short!("batch"), symbol_short!("processed"), batch_id);
        env.events().publish(topics, metrics.clone());
    }

    pub fn category_analytics(env: &Env, batch_id: u64, category_metrics: &CategoryMetrics) {
        let topics = (symbol_short!("category"), batch_id);
        env.events().publish(
            topics,
            (category_metrics.category.clone(), category_metrics.clone()),
        );
    }

    pub fn analytics_started(env: &Env, batch_id: u64, tx_count: u32) {
        let topics = (symbol_short!("analytics"), symbol_short!("started"));
        env.events().publish(topics, (batch_id, tx_count));
    }

    pub fn analytics_completed(env: &Env, batch_id: u64, processing_cost: u64) {
        let topics = (symbol_short!("analytics"), symbol_short!("complete"));
        env.events().publish(topics, (batch_id, processing_cost));
    }

    pub fn high_value_alert(env: &Env, batch_id: u64, tx_id: u64, amount: i128) {
        let topics = (symbol_short!("alert"), symbol_short!("highval"));
        env.events().publish(topics, (batch_id, tx_id, amount));
    }

    pub fn audit_logged(env: &Env, actor: &Address, operation: &Symbol, status: &Symbol) {
        let topics = (symbol_short!("audit"), symbol_short!("log"));
        env.events()
            .publish(topics, (actor.clone(), operation.clone(), status.clone()));
    }

    pub fn rating_submitted(
        env: &Env,
        user: &Address,
        tx_id: u64,
        score: u32,
        status: RatingStatus,
    ) {
        let topics = (symbol_short!("rating"), symbol_short!("submit"), user);
        env.events().publish(topics, (tx_id, score, status));
    }

    pub fn transaction_status_updated(
        env: &Env,
        tx_id: u64,
        previous_status: Option<TransactionStatus>,
        new_status: TransactionStatus,
    ) {
        let topics = (symbol_short!("status"), symbol_short!("updated"));
        env.events()
            .publish(topics, (tx_id, previous_status, new_status));
    }

    pub fn transaction_status_update_failed(env: &Env, tx_id: u64) {
        let topics = (symbol_short!("status"), symbol_short!("failed"));
        env.events().publish(topics, tx_id);
    }

    pub fn operation_fee_updated(
        env: &Env,
        admin: &Address,
        operation: &Symbol,
        previous: Option<FeeConfig>,
        new: FeeConfig,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("operation_updated"));
        env.events()
            .publish(topics, (admin.clone(), operation.clone(), previous, new));
    }

    pub fn fee_cap_changed(env: &Env, admin: &Address, previous: Option<u64>, new: Option<u64>) {
        let topics = (symbol_short!("fee"), symbol_short!("cap_changed"));
        env.events().publish(topics, (admin.clone(), previous, new));
    }

    pub fn bundle_created(env: &Env, bundle_id: u64, result: &BundleResult) {
        let topics = (symbol_short!("bundle"), symbol_short!("created"), bundle_id);
        env.events().publish(topics, result.clone());
    }

    pub fn transaction_validated(env: &Env, bundle_id: u64, validation_result: &ValidationResult) {
        let topics = (
            symbol_short!("bundle"),
            symbol_short!("validated"),
            bundle_id,
        );
        env.events().publish(topics, validation_result.clone());
    }

    pub fn bundling_started(env: &Env, bundle_id: u64, tx_count: u32) {
        let topics = (symbol_short!("bundle"), symbol_short!("started"));
        env.events().publish(topics, (bundle_id, tx_count));
    }

    pub fn bundling_completed(env: &Env, bundle_id: u64, can_bundle: bool) {
        let topics = (symbol_short!("bundle"), symbol_short!("completed"));
        env.events().publish(topics, (bundle_id, can_bundle));
    }

    pub fn transaction_validation_failed(env: &Env, bundle_id: u64, tx_id: u64, error: &Symbol) {
        let topics = (symbol_short!("bundle"), symbol_short!("failed"));
        env.events()
            .publish(topics, (bundle_id, tx_id, error.clone()));
    }

    pub fn refund_batch_started(env: &Env, batch_id: u64, request_count: u32) {
        let topics = (symbol_short!("refund"), symbol_short!("started"));
        env.events().publish(topics, (batch_id, request_count));
    }

    pub fn refund_processed(env: &Env, batch_id: u64, refund_result: &RefundResult) {
        let topics = (
            symbol_short!("refund"),
            symbol_short!("processed"),
            batch_id,
        );
        env.events().publish(topics, refund_result.clone());
    }

    pub fn refund_batch_completed(env: &Env, batch_id: u64, metrics: &RefundBatchMetrics) {
        let topics = (
            symbol_short!("refund"),
            symbol_short!("completed"),
            batch_id,
        );
        env.events().publish(topics, metrics.clone());
    }

    pub fn refund_error(env: &Env, batch_id: u64, tx_id: u64, error_msg: Symbol) {
        let topics = (symbol_short!("refund"), symbol_short!("error"));
        env.events().publish(topics, (batch_id, tx_id, error_msg));
    }

    pub fn analytics_updated(
        env: &Env,
        user: &Address,
        year: u32,
        month: u32,
        analytics: &MonthlySpendingAnalytics,
    ) {
        let topics = (symbol_short!("analytics"), symbol_short!("updated"), user);
        env.events().publish(
            topics,
            (
                year,
                month,
                analytics.total_spending,
                analytics.transaction_count,
            ),
        );
    }

    pub fn fee_deducted(
        env: &Env,
        gross_amount: i128,
        fee_amount: i128,
        net_amount: i128,
        fee_percentage_bps: u32,
    ) {
        let topics = (symbol_short!("fee"), symbol_short!("deducted"));
        env.events().publish(
            topics,
            (gross_amount, fee_amount, net_amount, fee_percentage_bps),
        );
    }

    pub fn fee_distributed(env: &Env, recipient: &Address, amount: i128, share_bps: u32) {
        let topics = (
            symbol_short!("fee"),
            symbol_short!("distributed"),
            recipient,
        );
        env.events().publish(topics, (amount, share_bps));
    }

    pub fn fee_pause_toggled(env: &Env, admin: &Address, paused: bool) {
        let topics = (
            symbol_short!("fee"),
            if paused {
                symbol_short!("paused")
            } else {
                symbol_short!("resumed")
            },
        );
        env.events().publish(topics, admin.clone());
    }
}
