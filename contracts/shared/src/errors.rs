use soroban_sdk::contracterror;

/// Common error definitions shared across contracts.
///
/// Error codes are intentionally stable and should never be reused
/// once deployed to production, as external clients may depend on them.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SharedError {
    // ---------------------------------------------------------------------
    // Authorization & Access Control
    // ---------------------------------------------------------------------

    /// Caller lacks permission to perform the requested action.
    Unauthorized = 1,

    /// Contract has not been initialized.
    NotInitialized = 2,

    /// Contract has already been initialized.
    AlreadyInitialized = 3,

    // ---------------------------------------------------------------------
    // Validation Errors
    // ---------------------------------------------------------------------

    /// Input validation failed.
    InvalidInput = 10,

    /// Provided string, vector, or collection length is invalid.
    InvalidLength = 11,

    /// Required field was not provided.
    MissingRequiredField = 12,

    /// Value is outside the permitted range.
    ValueOutOfRange = 13,

    /// Invalid state transition requested.
    InvalidStateTransition = 14,

    // ---------------------------------------------------------------------
    // Resource Errors
    // ---------------------------------------------------------------------

    /// Requested resource could not be found.
    ResourceNotFound = 20,

    /// Resource already exists.
    ResourceAlreadyExists = 21,

    /// Resource is inactive, expired, or unavailable.
    ResourceUnavailable = 22,

    // ---------------------------------------------------------------------
    // Financial & Balance Errors
    // ---------------------------------------------------------------------

    /// Insufficient balance for operation.
    InsufficientBalance = 30,

    /// Amount must be greater than zero.
    InvalidAmount = 31,

    /// Arithmetic overflow occurred.
    ArithmeticOverflow = 32,

    /// Arithmetic underflow occurred.
    ArithmeticUnderflow = 33,

    // ---------------------------------------------------------------------
    // Time & Expiration Errors
    // ---------------------------------------------------------------------

    /// Operation attempted before allowed time.
    TooEarly = 40,

    /// Operation attempted after expiration.
    Expired = 41,

    /// Deadline has already passed.
    DeadlineExceeded = 42,

    // ---------------------------------------------------------------------
    // Contract State Errors
    // ---------------------------------------------------------------------

    /// Operation is not allowed in current contract state.
    InvalidState = 50,

    /// Resource is locked and cannot be modified.
    ResourceLocked = 51,

    /// Operation has already been completed.
    AlreadyCompleted = 52,

    // ---------------------------------------------------------------------
    // Upgrade & Migration Errors
    // ---------------------------------------------------------------------

    /// Contract version mismatch detected.
    VersionMismatch = 60,

    /// Storage migration failed.
    MigrationFailed = 61,

    // ---------------------------------------------------------------------
    // Generic Fallback
    // ---------------------------------------------------------------------

    /// Unexpected internal contract error.
    InternalError = 99,
}