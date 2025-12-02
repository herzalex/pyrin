//! Contract code storage

use pyrin_contracts_core::ContractAddress;
use pyrin_hashes::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum contract code size (24KB, similar to Ethereum's limit)
pub const MAX_CODE_SIZE: usize = 24 * 1024;

/// Contract bytecode with metadata
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ContractCode {
    /// The WASM bytecode
    pub code: Vec<u8>,
    /// Hash of the code
    pub code_hash: Hash,
    /// Deployer address
    pub deployer: ContractAddress,
    /// Block height when deployed
    pub deployed_at: u64,
}

impl ContractCode {
    /// Creates a new contract code entry
    pub fn new(code: Vec<u8>, deployer: ContractAddress, deployed_at: u64) -> Self {
        let code_hash = Self::compute_hash(&code);
        Self {
            code,
            code_hash,
            deployer,
            deployed_at,
        }
    }

    /// Computes the hash of the bytecode
    pub fn compute_hash(code: &[u8]) -> Hash {
        use sha3::Digest;
        let hash = sha3::Keccak256::digest(code);
        Hash::from_slice(&hash)
    }

    /// Returns the size of the code
    pub fn size(&self) -> usize {
        self.code.len()
    }

    /// Returns true if the code is empty
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Validates the WASM bytecode
    pub fn validate(&self) -> Result<(), String> {
        // Check size
        if self.code.len() > MAX_CODE_SIZE {
            return Err(format!(
                "Code size {} exceeds maximum {}",
                self.code.len(),
                MAX_CODE_SIZE
            ));
        }

        // Check WASM magic bytes
        if self.code.len() < 8 {
            return Err("Code too short".to_string());
        }

        let wasm_magic = &[0x00, 0x61, 0x73, 0x6d]; // \0asm
        if &self.code[0..4] != wasm_magic {
            return Err("Invalid WASM magic bytes".to_string());
        }

        // Check WASM version (1)
        let wasm_version = &[0x01, 0x00, 0x00, 0x00];
        if &self.code[4..8] != wasm_version {
            return Err("Unsupported WASM version".to_string());
        }

        Ok(())
    }
}

/// Code storage for contracts
pub struct CodeStorage {
    /// Code by contract address
    by_address: RwLock<HashMap<ContractAddress, Arc<ContractCode>>>,
    /// Code by hash (for deduplication)
    by_hash: RwLock<HashMap<Hash, Arc<ContractCode>>>,
}

impl CodeStorage {
    /// Creates a new code storage
    pub fn new() -> Self {
        Self {
            by_address: RwLock::new(HashMap::new()),
            by_hash: RwLock::new(HashMap::new()),
        }
    }

    /// Gets the code for a contract address
    pub fn get(&self, address: &ContractAddress) -> Option<Arc<ContractCode>> {
        let by_address = self.by_address.read();
        by_address.get(address).cloned()
    }

    /// Gets the code by its hash
    pub fn get_by_hash(&self, hash: &Hash) -> Option<Arc<ContractCode>> {
        let by_hash = self.by_hash.read();
        by_hash.get(hash).cloned()
    }

    /// Stores code for a contract
    pub fn store(&self, address: ContractAddress, code: ContractCode) -> Arc<ContractCode> {
        let code_hash = code.code_hash;
        let code = Arc::new(code);

        // Store by address
        {
            let mut by_address = self.by_address.write();
            by_address.insert(address, Arc::clone(&code));
        }

        // Store by hash (for deduplication)
        {
            let mut by_hash = self.by_hash.write();
            by_hash.entry(code_hash).or_insert_with(|| Arc::clone(&code));
        }

        code
    }

    /// Checks if a contract exists at the given address
    pub fn exists(&self, address: &ContractAddress) -> bool {
        let by_address = self.by_address.read();
        by_address.contains_key(address)
    }

    /// Removes a contract (for self-destruct)
    pub fn remove(&self, address: &ContractAddress) -> Option<Arc<ContractCode>> {
        let mut by_address = self.by_address.write();
        by_address.remove(address)
    }

    /// Returns the number of deployed contracts
    pub fn count(&self) -> usize {
        let by_address = self.by_address.read();
        by_address.len()
    }

    /// Returns all contract addresses
    pub fn addresses(&self) -> Vec<ContractAddress> {
        let by_address = self.by_address.read();
        by_address.keys().cloned().collect()
    }
}

impl Default for CodeStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_wasm_header() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6d, // \0asm
            0x01, 0x00, 0x00, 0x00, // version 1
        ]
    }

    #[test]
    fn test_contract_code() {
        let code = ContractCode::new(
            valid_wasm_header(),
            ContractAddress::zero(),
            100,
        );

        assert_eq!(code.size(), 8);
        assert!(!code.is_empty());
        assert!(code.validate().is_ok());
    }

    #[test]
    fn test_invalid_wasm() {
        let code = ContractCode::new(
            vec![0x01, 0x02, 0x03, 0x04],
            ContractAddress::zero(),
            100,
        );

        assert!(code.validate().is_err());
    }

    #[test]
    fn test_code_storage() {
        let storage = CodeStorage::new();
        let address = ContractAddress::new([1u8; 20]);
        
        assert!(!storage.exists(&address));
        
        let code = ContractCode::new(
            valid_wasm_header(),
            ContractAddress::zero(),
            100,
        );
        
        storage.store(address, code);
        
        assert!(storage.exists(&address));
        assert_eq!(storage.count(), 1);
        
        let retrieved = storage.get(&address).unwrap();
        assert_eq!(retrieved.size(), 8);
    }

    #[test]
    fn test_code_deduplication() {
        let storage = CodeStorage::new();
        let addr1 = ContractAddress::new([1u8; 20]);
        let addr2 = ContractAddress::new([2u8; 20]);
        
        let wasm = valid_wasm_header();
        
        let code1 = ContractCode::new(wasm.clone(), ContractAddress::zero(), 100);
        let code2 = ContractCode::new(wasm, ContractAddress::zero(), 101);
        
        storage.store(addr1, code1);
        storage.store(addr2, code2);
        
        // Both addresses point to code with the same hash
        let retrieved1 = storage.get(&addr1).unwrap();
        let retrieved2 = storage.get(&addr2).unwrap();
        
        assert_eq!(retrieved1.code_hash, retrieved2.code_hash);
    }
}
