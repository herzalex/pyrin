//! Security primitives for smart contract execution
//!
//! This module provides security features to protect against common
//! smart contract vulnerabilities.

use std::collections::HashSet;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::{ContractAddress, ContractError, ContractResult};

/// Reentrancy guard to prevent reentrancy attacks
/// 
/// Reentrancy attacks occur when a contract calls an external contract
/// before finishing its own execution, allowing the external contract
/// to call back and exploit the incomplete state.
#[derive(Debug)]
pub struct ReentrancyGuard {
    /// Contracts currently being executed
    locked: Arc<RwLock<HashSet<ContractAddress>>>,
}

impl ReentrancyGuard {
    /// Creates a new reentrancy guard
    pub fn new() -> Self {
        Self {
            locked: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Attempts to acquire a lock for the given contract
    /// Returns an error if the contract is already locked
    pub fn acquire(&self, address: &ContractAddress) -> ContractResult<ReentrancyLock> {
        let mut locked = self.locked.write();
        if locked.contains(address) {
            return Err(ContractError::Reentrancy(address.to_string()));
        }
        locked.insert(*address);
        Ok(ReentrancyLock {
            address: *address,
            guard: Arc::clone(&self.locked),
        })
    }

    /// Checks if a contract is currently locked
    pub fn is_locked(&self, address: &ContractAddress) -> bool {
        self.locked.read().contains(address)
    }

    /// Returns the number of currently locked contracts
    pub fn locked_count(&self) -> usize {
        self.locked.read().len()
    }
}

impl Default for ReentrancyGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ReentrancyGuard {
    fn clone(&self) -> Self {
        Self {
            locked: Arc::clone(&self.locked),
        }
    }
}

/// RAII guard that releases the lock when dropped
#[derive(Debug)]
pub struct ReentrancyLock {
    address: ContractAddress,
    guard: Arc<RwLock<HashSet<ContractAddress>>>,
}

impl Drop for ReentrancyLock {
    fn drop(&mut self) {
        self.guard.write().remove(&self.address);
    }
}

/// Input validation utilities
pub struct InputValidator;

impl InputValidator {
    /// Maximum allowed input data size (64KB)
    pub const MAX_INPUT_SIZE: usize = 64 * 1024;
    
    /// Maximum allowed return data size (64KB)
    pub const MAX_RETURN_SIZE: usize = 64 * 1024;
    
    /// Maximum number of log topics
    pub const MAX_LOG_TOPICS: usize = 4;
    
    /// Maximum log data size (16KB)
    pub const MAX_LOG_DATA_SIZE: usize = 16 * 1024;

    /// Validates input data size
    pub fn validate_input_size(data: &[u8]) -> ContractResult<()> {
        if data.len() > Self::MAX_INPUT_SIZE {
            return Err(ContractError::InvalidArguments(format!(
                "Input size {} exceeds maximum {}",
                data.len(),
                Self::MAX_INPUT_SIZE
            )));
        }
        Ok(())
    }

    /// Validates return data size
    pub fn validate_return_size(data: &[u8]) -> ContractResult<()> {
        if data.len() > Self::MAX_RETURN_SIZE {
            return Err(ContractError::InvalidArguments(format!(
                "Return size {} exceeds maximum {}",
                data.len(),
                Self::MAX_RETURN_SIZE
            )));
        }
        Ok(())
    }

    /// Validates log parameters
    pub fn validate_log(topics: usize, data_size: usize) -> ContractResult<()> {
        if topics > Self::MAX_LOG_TOPICS {
            return Err(ContractError::InvalidArguments(format!(
                "Too many log topics: {} (max {})",
                topics,
                Self::MAX_LOG_TOPICS
            )));
        }
        if data_size > Self::MAX_LOG_DATA_SIZE {
            return Err(ContractError::InvalidArguments(format!(
                "Log data size {} exceeds maximum {}",
                data_size,
                Self::MAX_LOG_DATA_SIZE
            )));
        }
        Ok(())
    }

    /// Validates a contract address
    pub fn validate_address(address: &ContractAddress) -> ContractResult<()> {
        // Zero address is not allowed as a target
        if address.is_zero() {
            return Err(ContractError::InvalidAddress("zero address".to_string()));
        }
        Ok(())
    }

    /// Validates a value transfer amount
    pub fn validate_value(value: u64, balance: u64) -> ContractResult<()> {
        if value > balance {
            return Err(ContractError::insufficient_balance(value, balance));
        }
        Ok(())
    }
}

/// Rate limiter for contract operations
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum operations per block
    max_ops_per_block: u32,
    /// Current operation count
    current_ops: RwLock<u32>,
}

impl RateLimiter {
    /// Creates a new rate limiter
    pub fn new(max_ops_per_block: u32) -> Self {
        Self {
            max_ops_per_block,
            current_ops: RwLock::new(0),
        }
    }

    /// Attempts to consume an operation slot
    pub fn try_consume(&self) -> ContractResult<()> {
        let mut ops = self.current_ops.write();
        if *ops >= self.max_ops_per_block {
            return Err(ContractError::Internal(
                "Rate limit exceeded for block".to_string()
            ));
        }
        *ops += 1;
        Ok(())
    }

    /// Resets the operation count for a new block
    pub fn reset(&self) {
        *self.current_ops.write() = 0;
    }

    /// Returns the remaining operations allowed
    pub fn remaining(&self) -> u32 {
        self.max_ops_per_block.saturating_sub(*self.current_ops.read())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(10_000) // Default 10K ops per block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reentrancy_guard() {
        let guard = ReentrancyGuard::new();
        let addr1 = ContractAddress::new([1u8; 20]);
        let addr2 = ContractAddress::new([2u8; 20]);

        // First lock should succeed
        let lock1 = guard.acquire(&addr1).unwrap();
        assert!(guard.is_locked(&addr1));

        // Second lock on same address should fail
        assert!(guard.acquire(&addr1).is_err());

        // Lock on different address should succeed
        let _lock2 = guard.acquire(&addr2).unwrap();
        assert!(guard.is_locked(&addr2));

        // After dropping lock1, addr1 should be unlocked
        drop(lock1);
        assert!(!guard.is_locked(&addr1));

        // Now we can acquire addr1 again
        let _lock3 = guard.acquire(&addr1).unwrap();
        assert!(guard.is_locked(&addr1));
    }

    #[test]
    fn test_input_validator() {
        // Valid input
        let data = vec![0u8; 1024];
        assert!(InputValidator::validate_input_size(&data).is_ok());

        // Too large input
        let large_data = vec![0u8; InputValidator::MAX_INPUT_SIZE + 1];
        assert!(InputValidator::validate_input_size(&large_data).is_err());
    }

    #[test]
    fn test_log_validation() {
        assert!(InputValidator::validate_log(4, 1024).is_ok());
        assert!(InputValidator::validate_log(5, 1024).is_err()); // Too many topics
        assert!(InputValidator::validate_log(2, 20_000).is_err()); // Data too large
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(3);
        
        assert!(limiter.try_consume().is_ok());
        assert!(limiter.try_consume().is_ok());
        assert!(limiter.try_consume().is_ok());
        assert!(limiter.try_consume().is_err()); // Limit reached

        limiter.reset();
        assert!(limiter.try_consume().is_ok()); // Works again after reset
    }
}
