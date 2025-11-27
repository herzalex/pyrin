//! Contract types for transactions, receipts, and logs

use crate::{ContractAddress, Gas};
use borsh::{BorshDeserialize, BorshSerialize};
use pyrin_hashes::Hash;
use serde::{Deserialize, Serialize};

/// Payload for contract transactions
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ContractPayload {
    /// Deploy a new contract
    Deploy(ContractDeploy),
    /// Call an existing contract
    Call(ContractCall),
}

impl ContractPayload {
    /// Returns true if this is a deployment payload
    pub fn is_deploy(&self) -> bool {
        matches!(self, Self::Deploy(_))
    }

    /// Returns true if this is a call payload
    pub fn is_call(&self) -> bool {
        matches!(self, Self::Call(_))
    }

    /// Returns the gas limit from the payload
    pub fn gas_limit(&self) -> Gas {
        match self {
            Self::Deploy(d) => d.gas_limit,
            Self::Call(c) => c.gas_limit,
        }
    }

    /// Serializes the payload to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("serialization should not fail")
    }

    /// Deserializes a payload from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        borsh::BorshDeserialize::try_from_slice(bytes)
    }
}

/// Contract deployment data
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ContractDeploy {
    /// The WASM bytecode of the contract
    pub code: Vec<u8>,
    /// Constructor arguments (ABI encoded)
    pub init_data: Vec<u8>,
    /// Gas limit for deployment
    pub gas_limit: Gas,
    /// Value to send with deployment (in sompis)
    pub value: u64,
    /// Salt for CREATE2 deployment (optional)
    pub salt: Option<[u8; 32]>,
}

impl ContractDeploy {
    /// Creates a new deployment payload
    pub fn new(code: Vec<u8>, init_data: Vec<u8>, gas_limit: Gas, value: u64) -> Self {
        Self {
            code,
            init_data,
            gas_limit,
            value,
            salt: None,
        }
    }

    /// Creates a CREATE2 deployment payload
    pub fn new_create2(code: Vec<u8>, init_data: Vec<u8>, gas_limit: Gas, value: u64, salt: [u8; 32]) -> Self {
        Self {
            code,
            init_data,
            gas_limit,
            value,
            salt: Some(salt),
        }
    }

    /// Returns the hash of the code
    pub fn code_hash(&self) -> Hash {
        pyrin_hashes::Hash::from_slice(&sha3::Keccak256::digest(&self.code))
    }
}

/// Contract call data
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ContractCall {
    /// Target contract address
    pub to: ContractAddress,
    /// Function selector and arguments (ABI encoded)
    pub data: Vec<u8>,
    /// Gas limit for the call
    pub gas_limit: Gas,
    /// Value to send with the call (in sompis)
    pub value: u64,
}

impl ContractCall {
    /// Creates a new call payload
    pub fn new(to: ContractAddress, data: Vec<u8>, gas_limit: Gas, value: u64) -> Self {
        Self {
            to,
            data,
            gas_limit,
            value,
        }
    }

    /// Returns the function selector (first 4 bytes of data)
    pub fn selector(&self) -> Option<[u8; 4]> {
        if self.data.len() >= 4 {
            let mut selector = [0u8; 4];
            selector.copy_from_slice(&self.data[..4]);
            Some(selector)
        } else {
            None
        }
    }

    /// Returns the call arguments (data after selector)
    pub fn arguments(&self) -> &[u8] {
        if self.data.len() >= 4 {
            &self.data[4..]
        } else {
            &[]
        }
    }
}

use sha3::Digest;

/// Log topic (32 bytes)
pub type LogTopic = [u8; 32];

/// Event log emitted by a contract
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Log {
    /// Address of the contract that emitted this log
    pub address: ContractAddress,
    /// Indexed topics (up to 4)
    pub topics: Vec<LogTopic>,
    /// Non-indexed data
    pub data: Vec<u8>,
}

impl Log {
    /// Creates a new log
    pub fn new(address: ContractAddress, topics: Vec<LogTopic>, data: Vec<u8>) -> Self {
        Self {
            address,
            topics,
            data,
        }
    }

    /// Returns the first topic (usually the event signature)
    pub fn event_signature(&self) -> Option<&LogTopic> {
        self.topics.first()
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution succeeded
    Success,
    /// Execution reverted
    Reverted,
    /// Execution failed (out of gas, etc.)
    Failed,
}

/// Receipt for a contract transaction
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ContractReceipt {
    /// Transaction hash
    pub tx_hash: Hash,
    /// Execution status
    pub status: ExecutionStatus,
    /// Gas used by this transaction
    pub gas_used: Gas,
    /// Contract address (for deployments)
    pub contract_address: Option<ContractAddress>,
    /// Return data from execution
    pub return_data: Vec<u8>,
    /// Logs emitted during execution
    pub logs: Vec<Log>,
    /// Error message (if failed/reverted)
    pub error: Option<String>,
}

impl ContractReceipt {
    /// Creates a successful receipt
    pub fn success(
        tx_hash: Hash,
        gas_used: Gas,
        contract_address: Option<ContractAddress>,
        return_data: Vec<u8>,
        logs: Vec<Log>,
    ) -> Self {
        Self {
            tx_hash,
            status: ExecutionStatus::Success,
            gas_used,
            contract_address,
            return_data,
            logs,
            error: None,
        }
    }

    /// Creates a reverted receipt
    pub fn reverted(tx_hash: Hash, gas_used: Gas, error: String, logs: Vec<Log>) -> Self {
        Self {
            tx_hash,
            status: ExecutionStatus::Reverted,
            gas_used,
            contract_address: None,
            return_data: Vec::new(),
            logs,
            error: Some(error),
        }
    }

    /// Creates a failed receipt
    pub fn failed(tx_hash: Hash, gas_used: Gas, error: String) -> Self {
        Self {
            tx_hash,
            status: ExecutionStatus::Failed,
            gas_used,
            contract_address: None,
            return_data: Vec::new(),
            logs: Vec::new(),
            error: Some(error),
        }
    }

    /// Returns true if execution was successful
    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Success
    }

    /// Returns true if execution was reverted
    pub fn is_reverted(&self) -> bool {
        self.status == ExecutionStatus::Reverted
    }

    /// Returns true if execution failed
    pub fn is_failed(&self) -> bool {
        self.status == ExecutionStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_deploy() {
        let deploy = ContractDeploy::new(
            vec![0x00, 0x61, 0x73, 0x6d], // WASM magic bytes
            vec![],
            100_000,
            0,
        );
        
        let payload = ContractPayload::Deploy(deploy);
        assert!(payload.is_deploy());
        assert_eq!(payload.gas_limit(), 100_000);
        
        // Test serialization roundtrip
        let bytes = payload.to_bytes();
        let decoded = ContractPayload::from_bytes(&bytes).unwrap();
        assert_eq!(payload, decoded);
    }

    #[test]
    fn test_contract_call() {
        let call = ContractCall::new(
            ContractAddress::zero(),
            vec![0xa9, 0x05, 0x9c, 0xbb, 0x01, 0x02, 0x03], // transfer(...)
            50_000,
            100,
        );
        
        assert_eq!(call.selector(), Some([0xa9, 0x05, 0x9c, 0xbb]));
        assert_eq!(call.arguments(), &[0x01, 0x02, 0x03]);
        
        let payload = ContractPayload::Call(call);
        assert!(payload.is_call());
    }

    #[test]
    fn test_log() {
        let log = Log::new(
            ContractAddress::zero(),
            vec![[1u8; 32], [2u8; 32]],
            vec![0x01, 0x02, 0x03],
        );
        
        assert_eq!(log.topics.len(), 2);
        assert_eq!(log.event_signature(), Some(&[1u8; 32]));
    }

    #[test]
    fn test_receipt() {
        let receipt = ContractReceipt::success(
            Hash::from_bytes([0u8; 32]),
            50_000,
            Some(ContractAddress::new([1u8; 20])),
            vec![0x01],
            vec![],
        );
        
        assert!(receipt.is_success());
        assert!(receipt.contract_address.is_some());
    }
}
