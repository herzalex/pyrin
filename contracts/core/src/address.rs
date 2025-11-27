//! Contract Address implementation

use borsh::{BorshDeserialize, BorshSerialize};
use pyrin_hashes::Hash;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::fmt;

/// Size of a contract address in bytes
pub const CONTRACT_ADDRESS_SIZE: usize = 20;

/// Represents a smart contract address on the Pyrin network.
/// 
/// Contract addresses are derived from the deployer's address and a nonce,
/// similar to Ethereum's CREATE address derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ContractAddress([u8; CONTRACT_ADDRESS_SIZE]);

impl ContractAddress {
    /// Creates a new contract address from raw bytes
    pub fn new(bytes: [u8; CONTRACT_ADDRESS_SIZE]) -> Self {
        Self(bytes)
    }

    /// Creates a contract address from a slice
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != CONTRACT_ADDRESS_SIZE {
            return None;
        }
        let mut bytes = [0u8; CONTRACT_ADDRESS_SIZE];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Creates a zero address (used for contract creation transactions)
    pub fn zero() -> Self {
        Self([0u8; CONTRACT_ADDRESS_SIZE])
    }

    /// Returns true if this is the zero address
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; CONTRACT_ADDRESS_SIZE]
    }

    /// Returns the raw bytes of the address
    pub fn as_bytes(&self) -> &[u8; CONTRACT_ADDRESS_SIZE] {
        &self.0
    }

    /// Computes a contract address from deployer address and nonce
    /// 
    /// This uses RLP encoding similar to Ethereum:
    /// address = keccak256(rlp([deployer, nonce]))[12:]
    pub fn create(deployer: &[u8], nonce: u64) -> Self {
        let mut hasher = Keccak256::new();
        
        // Simple RLP encoding for [deployer, nonce]
        // For simplicity, we use a custom encoding: deployer || nonce (big endian)
        hasher.update(deployer);
        hasher.update(&nonce.to_be_bytes());
        
        let result = hasher.finalize();
        let mut address = [0u8; CONTRACT_ADDRESS_SIZE];
        address.copy_from_slice(&result[12..32]);
        
        Self(address)
    }

    /// Computes a contract address using CREATE2 semantics
    /// 
    /// address = keccak256(0xff || deployer || salt || code_hash)[12:]
    pub fn create2(deployer: &ContractAddress, salt: &[u8; 32], code_hash: &Hash) -> Self {
        let mut hasher = Keccak256::new();
        hasher.update([0xff]);
        hasher.update(deployer.as_bytes());
        hasher.update(salt);
        hasher.update(code_hash.as_bytes());
        
        let result = hasher.finalize();
        let mut address = [0u8; CONTRACT_ADDRESS_SIZE];
        address.copy_from_slice(&result[12..32]);
        
        Self(address)
    }

    /// Converts to a hex string (without 0x prefix)
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parses from a hex string (with or without 0x prefix)
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)?;
        Self::from_slice(&bytes).ok_or(hex::FromHexError::InvalidStringLength)
    }
}

impl Default for ContractAddress {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for ContractAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", self.to_hex())
    }
}

impl AsRef<[u8]> for ContractAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; CONTRACT_ADDRESS_SIZE]> for ContractAddress {
    fn from(bytes: [u8; CONTRACT_ADDRESS_SIZE]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_address() {
        let zero = ContractAddress::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.to_hex(), "0000000000000000000000000000000000000000");
    }

    #[test]
    fn test_create_address() {
        let deployer = [1u8; 32];
        let addr1 = ContractAddress::create(&deployer, 0);
        let addr2 = ContractAddress::create(&deployer, 1);
        
        assert!(!addr1.is_zero());
        assert!(!addr2.is_zero());
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_hex_roundtrip() {
        let addr = ContractAddress::new([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                         0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                         0x12, 0x34, 0x56, 0x78]);
        let hex = addr.to_hex();
        let parsed = ContractAddress::from_hex(&hex).unwrap();
        assert_eq!(addr, parsed);
    }
}
