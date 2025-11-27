//! Contract Error types

use thiserror::Error;

/// Result type for contract operations
pub type ContractResult<T> = Result<T, ContractError>;

/// Errors that can occur during contract execution
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ContractError {
    /// Ran out of gas during execution
    #[error("Out of gas: used {used}, limit {limit}")]
    OutOfGas { used: u64, limit: u64 },

    /// Invalid contract bytecode
    #[error("Invalid bytecode: {0}")]
    InvalidBytecode(String),

    /// Contract not found at the given address
    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    /// Function not found in contract
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Invalid function arguments
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Contract execution reverted
    #[error("Execution reverted: {0}")]
    Revert(String),

    /// Stack overflow during execution
    #[error("Stack overflow")]
    StackOverflow,

    /// Stack underflow during execution
    #[error("Stack underflow")]
    StackUnderflow,

    /// Invalid memory access
    #[error("Invalid memory access at offset {offset}, size {size}")]
    InvalidMemoryAccess { offset: u64, size: u64 },

    /// Invalid storage access
    #[error("Invalid storage access: {0}")]
    InvalidStorageAccess(String),

    /// Contract deployment failed
    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),

    /// Reentrancy detected
    #[error("Reentrancy detected for contract {0}")]
    Reentrancy(String),

    /// Insufficient balance for value transfer
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    /// Value transfer failed
    #[error("Transfer failed: {0}")]
    TransferFailed(String),

    /// Invalid call depth (too many nested calls)
    #[error("Call depth exceeded: {0}")]
    CallDepthExceeded(u32),

    /// WASM execution error
    #[error("WASM error: {0}")]
    WasmError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Contract already exists at address
    #[error("Contract already exists at address {0}")]
    ContractExists(String),

    /// Invalid contract address
    #[error("Invalid contract address: {0}")]
    InvalidAddress(String),

    /// Code size exceeds maximum
    #[error("Code size {size} exceeds maximum {max}")]
    CodeSizeExceeded { size: usize, max: usize },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ContractError {
    /// Creates an out of gas error
    pub fn out_of_gas(used: u64, limit: u64) -> Self {
        Self::OutOfGas { used, limit }
    }

    /// Creates an insufficient balance error
    pub fn insufficient_balance(required: u64, available: u64) -> Self {
        Self::InsufficientBalance { required, available }
    }

    /// Creates an invalid memory access error
    pub fn invalid_memory(offset: u64, size: u64) -> Self {
        Self::InvalidMemoryAccess { offset, size }
    }

    /// Creates a code size exceeded error
    pub fn code_size_exceeded(size: usize, max: usize) -> Self {
        Self::CodeSizeExceeded { size, max }
    }

    /// Returns true if this is a revert error
    pub fn is_revert(&self) -> bool {
        matches!(self, Self::Revert(_))
    }

    /// Returns true if this is an out of gas error
    pub fn is_out_of_gas(&self) -> bool {
        matches!(self, Self::OutOfGas { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ContractError::out_of_gas(1000, 500);
        assert_eq!(err.to_string(), "Out of gas: used 1000, limit 500");
        assert!(err.is_out_of_gas());
    }

    #[test]
    fn test_revert() {
        let err = ContractError::Revert("transfer failed".to_string());
        assert!(err.is_revert());
        assert!(!err.is_out_of_gas());
    }
}
