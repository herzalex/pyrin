//! Host functions exposed to WASM contracts

use pyrin_contracts_core::{ContractAddress, ContractError, ContractResult, Gas, Log, LogTopic};
use pyrin_contracts_storage::{ContractStorage, StorageKey, StorageValue};
use std::sync::Arc;

/// Maximum log data size
pub const MAX_LOG_DATA_SIZE: usize = 8 * 1024; // 8KB

/// Maximum number of topics per log
pub const MAX_LOG_TOPICS: usize = 4;

/// Host functions available to contracts
pub struct HostFunctions {
    /// Contract storage
    storage: Arc<ContractStorage>,
    /// Current contract address
    address: ContractAddress,
    /// Caller address
    caller: ContractAddress,
    /// Transaction origin
    origin: ContractAddress,
    /// Value sent with call
    value: u64,
    /// Block number
    block_number: u64,
    /// Block timestamp
    timestamp: u64,
    /// DAA score
    daa_score: u64,
    /// Gas meter
    gas: Arc<pyrin_contracts_core::GasMeter>,
}

impl HostFunctions {
    /// Creates a new host functions instance
    pub fn new(
        storage: Arc<ContractStorage>,
        address: ContractAddress,
        caller: ContractAddress,
        origin: ContractAddress,
        value: u64,
        block_number: u64,
        timestamp: u64,
        daa_score: u64,
        gas: Arc<pyrin_contracts_core::GasMeter>,
    ) -> Self {
        Self {
            storage,
            address,
            caller,
            origin,
            value,
            block_number,
            timestamp,
            daa_score,
            gas,
        }
    }

    // ========== Storage Functions ==========

    /// Reads from storage
    pub fn storage_read(&self, key: &StorageKey) -> ContractResult<StorageValue> {
        // Charge gas
        let cost = if self.storage.is_warm(&self.address, key) {
            self.gas.costs().sload_warm
        } else {
            self.storage.mark_warm(&self.address, *key);
            self.gas.costs().sload_cold
        };
        self.gas.consume(cost)?;

        Ok(self.storage.read(&self.address, key))
    }

    /// Writes to storage
    pub fn storage_write(&self, key: StorageKey, value: StorageValue) -> ContractResult<()> {
        // Get current value
        let current = self.storage.read(&self.address, &key);

        // Calculate gas cost
        let is_warm = self.storage.is_warm(&self.address, &key);
        if !is_warm {
            self.storage.mark_warm(&self.address, key);
        }

        let cost = if current == [0u8; 32] && value != [0u8; 32] {
            // New storage slot
            self.gas.costs().storage_write_new
        } else if current != [0u8; 32] && value == [0u8; 32] {
            // Clearing storage slot - add refund
            self.gas.add_refund(self.gas.costs().storage_clear_refund);
            self.gas.costs().storage_write_existing
        } else {
            self.gas.costs().storage_write_existing
        };

        self.gas.consume(cost)?;
        self.storage.write(&self.address, key, value);
        Ok(())
    }

    // ========== Context Functions ==========

    /// Returns the caller address
    pub fn get_caller(&self) -> ContractAddress {
        self.caller
    }

    /// Returns the origin address (tx sender)
    pub fn get_origin(&self) -> ContractAddress {
        self.origin
    }

    /// Returns the current contract address
    pub fn get_address(&self) -> ContractAddress {
        self.address
    }

    /// Returns the value sent with the call
    pub fn get_value(&self) -> u64 {
        self.value
    }

    // ========== Block Functions ==========

    /// Returns the current block number
    pub fn get_block_number(&self) -> u64 {
        self.block_number
    }

    /// Returns the current block timestamp
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the current DAA score
    pub fn get_daa_score(&self) -> u64 {
        self.daa_score
    }

    // ========== Crypto Functions ==========

    /// Computes SHA3-256 (Keccak256) hash
    pub fn sha3(&self, data: &[u8]) -> ContractResult<[u8; 32]> {
        use sha3::Digest;

        // Charge gas
        let words = (data.len() + 31) / 32;
        let cost = self.gas.costs().sha3_base + 
                   (words as Gas) * self.gas.costs().sha3_per_word;
        self.gas.consume(cost)?;

        let result = sha3::Keccak256::digest(data);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(hash)
    }

    // ========== Logging Functions ==========

    /// Emits a log event
    pub fn emit_log(&self, topics: Vec<LogTopic>, data: Vec<u8>) -> ContractResult<Log> {
        // Validate
        if topics.len() > MAX_LOG_TOPICS {
            return Err(ContractError::InvalidArguments(format!(
                "Too many topics: {} > {}",
                topics.len(),
                MAX_LOG_TOPICS
            )));
        }

        if data.len() > MAX_LOG_DATA_SIZE {
            return Err(ContractError::InvalidArguments(format!(
                "Log data too large: {} > {}",
                data.len(),
                MAX_LOG_DATA_SIZE
            )));
        }

        // Charge gas
        let cost = self.gas.costs().log_cost(topics.len(), data.len());
        self.gas.consume(cost)?;

        Ok(Log::new(self.address, topics, data))
    }

    // ========== Balance Functions ==========

    /// Returns the balance of an address
    pub fn get_balance(&self, address: &ContractAddress) -> ContractResult<u64> {
        self.gas.consume(self.gas.costs().balance)?;
        
        // Would need to access account storage
        // For now, return 0
        Ok(0)
    }

    /// Returns the balance of this contract
    pub fn get_self_balance(&self) -> ContractResult<u64> {
        self.gas.consume(5)?; // Cheap for self
        
        // Would need to access account storage
        Ok(0)
    }

    // ========== Gas Functions ==========

    /// Returns the remaining gas
    pub fn gas_remaining(&self) -> Gas {
        self.gas.remaining()
    }

    /// Returns the gas used so far
    pub fn gas_used(&self) -> Gas {
        self.gas.used()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyrin_contracts_core::GasMeter;

    fn create_host_functions() -> HostFunctions {
        HostFunctions::new(
            Arc::new(ContractStorage::new()),
            ContractAddress::new([1u8; 20]),
            ContractAddress::new([2u8; 20]),
            ContractAddress::new([2u8; 20]),
            0,
            100,
            1234567890,
            50,
            Arc::new(GasMeter::new(1_000_000)),
        )
    }

    #[test]
    fn test_context_functions() {
        let host = create_host_functions();
        
        assert_eq!(host.get_address(), ContractAddress::new([1u8; 20]));
        assert_eq!(host.get_caller(), ContractAddress::new([2u8; 20]));
        assert_eq!(host.get_block_number(), 100);
        assert_eq!(host.get_timestamp(), 1234567890);
    }

    #[test]
    fn test_storage() {
        let host = create_host_functions();
        let key = [1u8; 32];
        let value = [2u8; 32];

        // Read empty
        let read = host.storage_read(&key).unwrap();
        assert_eq!(read, [0u8; 32]);

        // Write
        host.storage_write(key, value).unwrap();

        // Read back
        let read = host.storage_read(&key).unwrap();
        assert_eq!(read, value);
    }

    #[test]
    fn test_sha3() {
        let host = create_host_functions();
        
        let hash = host.sha3(b"hello").unwrap();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_emit_log() {
        let host = create_host_functions();
        
        let topic = [1u8; 32];
        let log = host.emit_log(vec![topic], vec![0x01, 0x02]).unwrap();
        
        assert_eq!(log.address, ContractAddress::new([1u8; 20]));
        assert_eq!(log.topics.len(), 1);
        assert_eq!(log.data, vec![0x01, 0x02]);
    }

    #[test]
    fn test_too_many_topics() {
        let host = create_host_functions();
        
        let topics = vec![[1u8; 32]; 5]; // 5 topics > MAX_LOG_TOPICS
        let result = host.emit_log(topics, vec![]);
        
        assert!(result.is_err());
    }
}
