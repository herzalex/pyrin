//! Gas metering for contract execution

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Represents gas units
pub type Gas = u64;

/// Default gas costs for various operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct GasCosts {
    /// Base cost for any transaction
    pub tx_base: Gas,
    /// Cost per byte of transaction data
    pub tx_data_zero: Gas,
    /// Cost per non-zero byte of transaction data
    pub tx_data_non_zero: Gas,
    /// Cost for contract creation
    pub tx_create: Gas,
    
    /// Base cost for storage read
    pub storage_read: Gas,
    /// Cost for storage write (new value)
    pub storage_write_new: Gas,
    /// Cost for storage write (existing value)
    pub storage_write_existing: Gas,
    /// Refund for clearing storage
    pub storage_clear_refund: Gas,
    
    /// Cost per word of memory expansion
    pub memory_per_word: Gas,
    /// Cost for copying per word
    pub copy_per_word: Gas,
    
    /// Base cost for a CALL instruction
    pub call_base: Gas,
    /// Additional cost for calls with value transfer
    pub call_value_transfer: Gas,
    /// Cost for creating a new account
    pub call_new_account: Gas,
    
    /// Cost for LOG0 (no topics)
    pub log_base: Gas,
    /// Cost per topic in LOG
    pub log_per_topic: Gas,
    /// Cost per byte of log data
    pub log_per_byte: Gas,
    
    /// Cost for SHA3/Keccak256
    pub sha3_base: Gas,
    /// Cost per word for SHA3
    pub sha3_per_word: Gas,
    
    /// Cost for BALANCE opcode
    pub balance: Gas,
    /// Cost for EXTCODESIZE
    pub ext_code_size: Gas,
    /// Cost for EXTCODECOPY base
    pub ext_code_copy_base: Gas,
    /// Cost for EXTCODEHASH
    pub ext_code_hash: Gas,
    
    /// Cost for SLOAD (cold)
    pub sload_cold: Gas,
    /// Cost for SLOAD (warm)
    pub sload_warm: Gas,
    /// Cost for SSTORE (cold, set)
    pub sstore_cold_set: Gas,
    /// Cost for SSTORE (warm)
    pub sstore_warm: Gas,
}

impl Default for GasCosts {
    fn default() -> Self {
        Self {
            tx_base: 21_000,
            tx_data_zero: 4,
            tx_data_non_zero: 16,
            tx_create: 32_000,
            
            storage_read: 200,
            storage_write_new: 20_000,
            storage_write_existing: 5_000,
            storage_clear_refund: 15_000,
            
            memory_per_word: 3,
            copy_per_word: 3,
            
            call_base: 700,
            call_value_transfer: 9_000,
            call_new_account: 25_000,
            
            log_base: 375,
            log_per_topic: 375,
            log_per_byte: 8,
            
            sha3_base: 30,
            sha3_per_word: 6,
            
            balance: 700,
            ext_code_size: 700,
            ext_code_copy_base: 700,
            ext_code_hash: 700,
            
            sload_cold: 2_100,
            sload_warm: 100,
            sstore_cold_set: 22_100,
            sstore_warm: 100,
        }
    }
}

impl GasCosts {
    /// Creates a new GasCosts with custom values
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculates the gas cost for transaction data
    pub fn tx_data_cost(&self, data: &[u8]) -> Gas {
        let mut cost = 0u64;
        for byte in data {
            if *byte == 0 {
                cost = cost.saturating_add(self.tx_data_zero);
            } else {
                cost = cost.saturating_add(self.tx_data_non_zero);
            }
        }
        cost
    }

    /// Calculates the gas cost for memory expansion
    pub fn memory_expansion_cost(&self, current_size: u64, new_size: u64) -> Gas {
        if new_size <= current_size {
            return 0;
        }
        
        let new_words = (new_size + 31) / 32;
        let current_words = (current_size + 31) / 32;
        
        let new_cost = new_words * self.memory_per_word + (new_words * new_words) / 512;
        let current_cost = current_words * self.memory_per_word + (current_words * current_words) / 512;
        
        new_cost.saturating_sub(current_cost)
    }

    /// Calculates the gas cost for LOG operation
    pub fn log_cost(&self, topic_count: usize, data_len: usize) -> Gas {
        self.log_base
            .saturating_add((topic_count as Gas).saturating_mul(self.log_per_topic))
            .saturating_add((data_len as Gas).saturating_mul(self.log_per_byte))
    }
}

/// Gas meter for tracking gas consumption during execution
#[derive(Debug)]
pub struct GasMeter {
    /// Gas limit for this execution
    limit: Gas,
    /// Gas used so far
    used: AtomicU64,
    /// Gas refund accumulated
    refund: AtomicU64,
    /// Gas costs configuration
    costs: GasCosts,
}

impl GasMeter {
    /// Creates a new gas meter with the given limit
    pub fn new(limit: Gas) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
            refund: AtomicU64::new(0),
            costs: GasCosts::default(),
        }
    }

    /// Creates a new gas meter with custom costs
    pub fn with_costs(limit: Gas, costs: GasCosts) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
            refund: AtomicU64::new(0),
            costs,
        }
    }

    /// Returns the gas limit
    pub fn limit(&self) -> Gas {
        self.limit
    }

    /// Returns the gas used so far
    pub fn used(&self) -> Gas {
        self.used.load(Ordering::Relaxed)
    }

    /// Returns the remaining gas
    pub fn remaining(&self) -> Gas {
        self.limit.saturating_sub(self.used())
    }

    /// Returns the accumulated refund
    pub fn refund(&self) -> Gas {
        self.refund.load(Ordering::Relaxed)
    }

    /// Returns the gas costs configuration
    pub fn costs(&self) -> &GasCosts {
        &self.costs
    }

    /// Consumes the specified amount of gas
    /// Returns an error if there isn't enough gas
    pub fn consume(&self, amount: Gas) -> Result<(), super::ContractError> {
        let current = self.used.load(Ordering::Relaxed);
        let new_used = current.saturating_add(amount);
        
        if new_used > self.limit {
            return Err(super::ContractError::out_of_gas(new_used, self.limit));
        }
        
        self.used.store(new_used, Ordering::Relaxed);
        Ok(())
    }

    /// Adds a refund (capped at used/5 per EIP-3529)
    pub fn add_refund(&self, amount: Gas) {
        self.refund.fetch_add(amount, Ordering::Relaxed);
    }

    /// Calculates the effective gas used (after refunds)
    pub fn effective_used(&self) -> Gas {
        let used = self.used();
        let max_refund = used / 5; // Max 20% refund per EIP-3529
        let refund = std::cmp::min(self.refund(), max_refund);
        used.saturating_sub(refund)
    }

    /// Resets the gas meter for a new execution
    /// Note: This only resets used/refund counters. For a new limit, create a new GasMeter.
    pub fn reset(&self) {
        self.used.store(0, Ordering::Relaxed);
        self.refund.store(0, Ordering::Relaxed);
    }

    /// Creates a child gas meter for subcalls
    pub fn child(&self, limit: Gas) -> Self {
        let available = self.remaining();
        let child_limit = std::cmp::min(limit, available - available / 64); // Reserve 1/64 for parent
        
        Self {
            limit: child_limit,
            used: AtomicU64::new(0),
            refund: AtomicU64::new(0),
            costs: self.costs.clone(),
        }
    }

    /// Merges a child gas meter back into this one
    pub fn merge_child(&self, child: &GasMeter) {
        self.used.fetch_add(child.used(), Ordering::Relaxed);
        self.refund.fetch_add(child.refund(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_meter_basic() {
        let meter = GasMeter::new(1000);
        
        assert_eq!(meter.limit(), 1000);
        assert_eq!(meter.used(), 0);
        assert_eq!(meter.remaining(), 1000);
        
        meter.consume(100).unwrap();
        assert_eq!(meter.used(), 100);
        assert_eq!(meter.remaining(), 900);
    }

    #[test]
    fn test_gas_meter_out_of_gas() {
        let meter = GasMeter::new(100);
        
        meter.consume(50).unwrap();
        let result = meter.consume(100);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().is_out_of_gas());
    }

    #[test]
    fn test_gas_refund() {
        let meter = GasMeter::new(1000);
        
        meter.consume(500).unwrap();
        meter.add_refund(200);
        
        // Max refund is 20% of used = 100
        assert_eq!(meter.effective_used(), 400);
    }

    #[test]
    fn test_tx_data_cost() {
        let costs = GasCosts::default();
        
        // Empty data
        assert_eq!(costs.tx_data_cost(&[]), 0);
        
        // All zeros
        assert_eq!(costs.tx_data_cost(&[0, 0, 0, 0]), 16);
        
        // All non-zero
        assert_eq!(costs.tx_data_cost(&[1, 2, 3, 4]), 64);
        
        // Mixed
        assert_eq!(costs.tx_data_cost(&[0, 1, 0, 2]), 40);
    }

    #[test]
    fn test_child_gas_meter() {
        let parent = GasMeter::new(1000);
        parent.consume(100).unwrap();
        
        let child = parent.child(500);
        assert!(child.limit() <= 500);
        
        child.consume(200).unwrap();
        parent.merge_child(&child);
        
        assert_eq!(parent.used(), 300);
    }
}
