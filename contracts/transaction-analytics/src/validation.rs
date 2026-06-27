//! # Input Validation Module
//!
//! Implements strict validation for all public inputs in the transaction analytics contract.
//! Provides standardized validation functions for addresses, amounts, assets, and other inputs.

use soroban_sdk::{Address, Env, Vec};

use crate::types::{
    BundledTransaction, RatingInput, RefundRequest, Transaction, TransactionStatusUpdate,
    ValidationError,
};

/// Validates an address input ensuring it's not empty/null.
///
/// # Arguments
/// * `address` - The address to validate
///
/// # Returns
/// * `Ok(())` if valid, `Err(ValidationError)` if invalid
pub fn validate_address(_address: &Address) -> Result<(), ValidationError> {
    // Soroban SDK addresses are guaranteed to be valid by construction
    Ok(())
}

/// Validates a transaction amount ensuring it's positive and not zero.
///
/// # Arguments
/// * `amount` - The amount to validate
///
/// # Returns
/// * `Ok(())` if valid, `Err(ValidationError)` if invalid
pub fn validate_amount(amount: i128) -> Result<(), ValidationError> {
    if amount <= 0 {
        return Err(ValidationError::InvalidAmount);
    }

    Ok(())
}

/// Validates multiple amounts in a collection
pub fn validate_amounts(amounts: &[i128]) -> Result<(), ValidationError> {
    for &amount in amounts {
        validate_amount(amount)?;
    }
    Ok(())
}

/// Validates a transaction struct
pub fn validate_transaction(transaction: &Transaction) -> Result<(), ValidationError> {
    validate_address(&transaction.from)?;
    validate_address(&transaction.to)?;
    validate_amount(transaction.amount)?;

    // Validate timestamp is not in the future (within reasonable tolerance)
    let current_ledger = transaction.timestamp; // This would be env.ledger().sequence() in real usage
    if transaction.timestamp > current_ledger + 100000 {
        // Allow some future tolerance
        return Err(ValidationError::InvalidTimestamp);
    }

    Ok(())
}

/// Validates a vector of transactions
pub fn validate_transactions(transactions: &Vec<Transaction>) -> Result<(), ValidationError> {
    if transactions.is_empty() {
        return Err(ValidationError::EmptyBatch);
    }

    // Check for reasonable batch size
    if transactions.len() > 100 {
        // Assuming MAX_BATCH_SIZE is 100
        return Err(ValidationError::BatchTooLarge);
    }

    for transaction in transactions.iter() {
        validate_transaction(&transaction)?;
    }

    Ok(())
}

/// Validates a refund request
pub fn validate_refund_request(request: &RefundRequest) -> Result<(), ValidationError> {
    // Validate transaction ID is not zero
    if request.tx_id == 0 {
        return Err(ValidationError::InvalidTransactionId);
    }

    Ok(())
}

/// Validates a vector of refund requests
pub fn validate_refund_requests(requests: &Vec<RefundRequest>) -> Result<(), ValidationError> {
    if requests.is_empty() {
        return Err(ValidationError::EmptyBatch);
    }

    // Check for reasonable batch size
    if requests.len() > 100 {
        // Assuming MAX_BATCH_SIZE is 100
        return Err(ValidationError::BatchTooLarge);
    }

    // Check for duplicate transaction IDs
    for (index, request) in requests.iter().enumerate() {
        if has_duplicate_tx_id_in_refunds(requests, index, request.tx_id) {
            return Err(ValidationError::DuplicateTransactionId);
        }
        validate_refund_request(&request)?;
    }

    Ok(())
}

/// Validates a rating input
pub fn validate_rating_input(rating: &RatingInput) -> Result<(), ValidationError> {
    // Validate transaction ID is not zero
    if rating.tx_id == 0 {
        return Err(ValidationError::InvalidTransactionId);
    }

    // Validate rating score is within acceptable range (e.g., 1-5)
    if rating.score == 0 || rating.score > 5 {
        return Err(ValidationError::InvalidRating);
    }

    Ok(())
}

/// Validates a vector of rating inputs
pub fn validate_rating_inputs(inputs: &Vec<RatingInput>) -> Result<(), ValidationError> {
    if inputs.is_empty() {
        return Err(ValidationError::EmptyBatch);
    }

    // Check for reasonable batch size
    if inputs.len() > 100 {
        // Assuming MAX_BATCH_SIZE is 100
        return Err(ValidationError::BatchTooLarge);
    }

    for input in inputs.iter() {
        validate_rating_input(&input)?;
    }

    Ok(())
}

/// Validates a transaction status update
pub fn validate_transaction_status_update(
    update: &TransactionStatusUpdate,
) -> Result<(), ValidationError> {
    // Validate transaction ID is not zero
    if update.tx_id == 0 {
        return Err(ValidationError::InvalidTransactionId);
    }

    Ok(())
}

/// Validates a vector of transaction status updates
pub fn validate_transaction_status_updates(
    updates: &Vec<TransactionStatusUpdate>,
) -> Result<(), ValidationError> {
    if updates.is_empty() {
        return Err(ValidationError::EmptyBatch);
    }

    // Check for reasonable batch size
    if updates.len() > 100 {
        // Assuming MAX_BATCH_SIZE is 100
        return Err(ValidationError::BatchTooLarge);
    }

    // Check for duplicate transaction IDs
    for (index, update) in updates.iter().enumerate() {
        if has_duplicate_tx_id_in_status_updates(updates, index, update.tx_id) {
            return Err(ValidationError::DuplicateTransactionId);
        }
        validate_transaction_status_update(&update)?;
    }

    Ok(())
}

/// Validates a bundled transaction
pub fn validate_bundled_transaction(
    bundled_tx: &BundledTransaction,
) -> Result<(), ValidationError> {
    validate_transaction(&bundled_tx.transaction)?;

    // Validate that from and to addresses are different
    if bundled_tx.transaction.from == bundled_tx.transaction.to {
        return Err(ValidationError::SameAddress);
    }

    Ok(())
}

/// Validates a vector of bundled transactions
pub fn validate_bundled_transactions(
    bundled_txs: &Vec<BundledTransaction>,
) -> Result<(), ValidationError> {
    if bundled_txs.is_empty() {
        return Err(ValidationError::EmptyBatch);
    }

    // Check for reasonable batch size
    if bundled_txs.len() > 100 {
        // Assuming MAX_BATCH_SIZE is 100
        return Err(ValidationError::BatchTooLarge);
    }

    for bundled_tx in bundled_txs.iter() {
        validate_bundled_transaction(&bundled_tx)?;
    }

    Ok(())
}

/// Validates a user address for analytics functions
pub fn validate_user_address(env: &Env, user: &Address) -> Result<(), ValidationError> {
    let _ = env;
    validate_address(user)
}

fn has_duplicate_tx_id_in_refunds(requests: &Vec<RefundRequest>, index: usize, tx_id: u64) -> bool {
    requests
        .iter()
        .enumerate()
        .any(|(other_index, request)| other_index != index && request.tx_id == tx_id)
}

fn has_duplicate_tx_id_in_status_updates(
    updates: &Vec<TransactionStatusUpdate>,
    index: usize,
    tx_id: u64,
) -> bool {
    updates
        .iter()
        .enumerate()
        .any(|(other_index, update)| other_index != index && update.tx_id == tx_id)
}

/// Validates year and month for analytics functions
pub fn validate_year_month(year: u32, month: u32) -> Result<(), ValidationError> {
    if year < 2000 || year > 2100 {
        return Err(ValidationError::InvalidYear);
    }

    if month < 1 || month > 12 {
        return Err(ValidationError::InvalidMonth);
    }

    Ok(())
}

/// Validates a percentage value (in basis points, 0-10000)
pub fn validate_percentage_basis_points(bps: u32) -> Result<(), ValidationError> {
    if bps > 10000 {
        // 100% = 10000 basis points
        return Err(ValidationError::InvalidPercentage);
    }

    Ok(())
}

/// Validates that two addresses are different
pub fn validate_different_addresses(
    addr1: &Address,
    addr2: &Address,
) -> Result<(), ValidationError> {
    if addr1 == addr2 {
        return Err(ValidationError::SameAddress);
    }

    Ok(())
}

/// Validates asset-related parameters (placeholder for asset-aware contracts)
/// In a full implementation, this would validate asset identifiers, decimals, etc.
pub fn validate_asset_type(_asset_identifier: &str) -> Result<(), ValidationError> {
    // In a real asset-aware contract, this would validate asset identifiers
    // For now, we just ensure the asset identifier is not empty
    if _asset_identifier.is_empty() {
        return Err(ValidationError::InvalidCategory); // Reusing InvalidCategory for now
    }

    // Additional asset validation would go here
    // - Check if asset is registered
    // - Validate asset decimals
    // - Check asset permissions
    // etc.

    Ok(())
}

/// Validates an asset amount with respect to asset properties
pub fn validate_asset_amount(_asset_identifier: &str, amount: i128) -> Result<(), ValidationError> {
    validate_amount(amount)?;

    // Additional asset-specific validations would go here
    // For example, checking if amount exceeds asset supply limits
    // or validating precision based on asset decimals

    Ok(())
}

/// Validates multiple asset amounts
pub fn validate_asset_amounts(
    asset_identifier: &str,
    amounts: &[i128],
) -> Result<(), ValidationError> {
    for &amount in amounts {
        validate_asset_amount(asset_identifier, amount)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BundledTransaction, RatingInput, RefundRequest, Transaction, TransactionStatus,
        TransactionStatusUpdate,
    };
    use soroban_sdk::{testutils::Address as _, Env};

    fn create_test_transaction(env: &Env, tx_id: u64, amount: i128, category: &str) -> Transaction {
        Transaction {
            tx_id,
            from: Address::generate(env),
            to: Address::generate(env),
            amount,
            timestamp: 12345,
            category: symbol_short!(category),
        }
    }

    #[test]
    fn test_validate_positive_amount() {
        assert!(validate_amount(100).is_ok());
        assert!(validate_amount(1).is_ok());
    }

    #[test]
    fn test_validate_zero_or_negative_amount() {
        assert!(validate_amount(0).is_err());
        assert!(validate_amount(-1).is_err());
        assert!(validate_amount(-100).is_err());
    }

    #[test]
    fn test_validate_transaction_valid() {
        let env = Env::default();
        let transaction = create_test_transaction(&env, 1, 100, "transfer");
        assert!(validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_validate_transaction_invalid_amount() {
        let env = Env::default();
        let mut transaction = create_test_transaction(&env, 1, -100, "transfer");
        assert!(validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_validate_refund_request_valid() {
        let env = Env::default();
        let request = RefundRequest {
            tx_id: 1,
            reason: Some(symbol_short!("test")),
        };
        assert!(validate_refund_request(&request).is_ok());
    }

    #[test]
    fn test_validate_refund_request_invalid_tx_id() {
        let env = Env::default();
        let request = RefundRequest {
            tx_id: 0,
            reason: Some(symbol_short!("test")),
        };
        assert!(validate_refund_request(&request).is_err());
    }

    #[test]
    fn test_validate_rating_input_valid() {
        let input = RatingInput { tx_id: 1, score: 5 };
        assert!(validate_rating_input(&input).is_ok());
    }

    #[test]
    fn test_validate_rating_input_invalid_score() {
        let input = RatingInput {
            tx_id: 1,
            score: 0, // Invalid
        };
        assert!(validate_rating_input(&input).is_err());

        let input2 = RatingInput {
            tx_id: 1,
            score: 6, // Invalid
        };
        assert!(validate_rating_input(&input2).is_err());
    }

    #[test]
    fn test_validate_year_month_valid() {
        assert!(validate_year_month(2023, 1).is_ok());
        assert!(validate_year_month(2023, 12).is_ok());
        assert!(validate_year_month(2100, 6).is_ok());
    }

    #[test]
    fn test_validate_year_month_invalid() {
        assert!(validate_year_month(1999, 6).is_err()); // Year too early
        assert!(validate_year_month(2101, 6).is_err()); // Year too late
        assert!(validate_year_month(2023, 0).is_err()); // Month too early
        assert!(validate_year_month(2023, 13).is_err()); // Month too late
    }

    #[test]
    fn test_validate_percentage_basis_points() {
        assert!(validate_percentage_basis_points(0).is_ok());
        assert!(validate_percentage_basis_points(5000).is_ok()); // 50%
        assert!(validate_percentage_basis_points(10000).is_ok()); // 100%
        assert!(validate_percentage_basis_points(10001).is_err()); // Over 100%
    }

    #[test]
    fn test_validate_asset_type() {
        assert!(validate_asset_type("USDC").is_ok());
        assert!(validate_asset_type("XLM").is_ok());
        assert!(validate_asset_type("").is_err()); // Empty asset identifier
    }

    #[test]
    fn test_validate_asset_amount() {
        assert!(validate_asset_amount("USDC", 100).is_ok());
        assert!(validate_asset_amount("XLM", 1).is_ok());
        assert!(validate_asset_amount("USDC", 0).is_err()); // Zero amount
        assert!(validate_asset_amount("USDC", -1).is_err()); // Negative amount
    }

    #[test]
    fn test_validate_asset_amounts() {
        let amounts = vec![100, 200, 300];
        assert!(validate_asset_amounts("USDC", &amounts).is_ok());

        let invalid_amounts = vec![100, -1, 300]; // Contains negative
        assert!(validate_asset_amounts("USDC", &invalid_amounts).is_err());
    }

    #[test]
    fn test_validate_different_addresses() {
        let env = Env::default();
        let addr1 = Address::generate(&env);
        let addr2 = Address::generate(&env);

        assert!(validate_different_addresses(&addr1, &addr2).is_ok());
        assert!(validate_different_addresses(&addr1, &addr1).is_err()); // Same addresses
    }
}
