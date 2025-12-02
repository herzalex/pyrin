//! Smart Contract RPC model types

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// RPC representation of a contract address (20 bytes)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcContractAddress {
    /// The address bytes as hex string
    pub address: String,
}

impl RpcContractAddress {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}

/// Request to deploy a smart contract
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployContractRequest {
    /// The WASM bytecode as hex string
    pub code: String,
    /// Constructor arguments as hex string
    pub init_data: String,
    /// Gas limit for deployment
    pub gas_limit: u64,
    /// Value to send with deployment (in sompis)
    pub value: u64,
}

/// Response from contract deployment
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployContractResponse {
    /// The deployed contract address
    pub contract_address: RpcContractAddress,
    /// Transaction hash
    pub tx_hash: String,
    /// Gas used for deployment
    pub gas_used: u64,
}

/// Request to call a smart contract
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallContractRequest {
    /// Target contract address
    pub contract_address: RpcContractAddress,
    /// Call data (function selector + arguments) as hex string
    pub data: String,
    /// Gas limit for the call
    pub gas_limit: u64,
    /// Value to send with the call (in sompis)
    pub value: u64,
}

/// Response from contract call
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallContractResponse {
    /// Whether the call was successful
    pub success: bool,
    /// Return data as hex string
    pub return_data: String,
    /// Gas used
    pub gas_used: u64,
    /// Error message (if any)
    pub error: Option<String>,
    /// Logs emitted during execution
    pub logs: Vec<RpcContractLog>,
}

/// Request to estimate gas for a contract call
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateContractGasRequest {
    /// Target contract address (None for deployment)
    pub contract_address: Option<RpcContractAddress>,
    /// Call data as hex string
    pub data: String,
    /// Value to send (in sompis)
    pub value: u64,
}

/// Response from gas estimation
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateContractGasResponse {
    /// Estimated gas required
    pub gas_estimate: u64,
}

/// Request to get contract code
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractCodeRequest {
    /// Contract address
    pub contract_address: RpcContractAddress,
}

/// Response containing contract code
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractCodeResponse {
    /// Contract code as hex string (empty if no contract)
    pub code: String,
    /// Code hash
    pub code_hash: String,
}

/// Request to get contract storage
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractStorageRequest {
    /// Contract address
    pub contract_address: RpcContractAddress,
    /// Storage key as hex string
    pub key: String,
}

/// Response containing storage value
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractStorageResponse {
    /// Storage value as hex string
    pub value: String,
}

/// A log entry emitted by a contract
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcContractLog {
    /// Contract address that emitted the log
    pub address: RpcContractAddress,
    /// Log topics (up to 4)
    pub topics: Vec<String>,
    /// Log data as hex string
    pub data: String,
}

/// Request to get contract logs
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractLogsRequest {
    /// Filter by contract address (optional)
    pub contract_address: Option<RpcContractAddress>,
    /// Start block (optional)
    pub from_block: Option<u64>,
    /// End block (optional)
    pub to_block: Option<u64>,
    /// Filter by topics (optional)
    pub topics: Option<Vec<Option<String>>>,
}

/// Response containing contract logs
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractLogsResponse {
    /// Matching logs
    pub logs: Vec<RpcContractLogEntry>,
}

/// A log entry with block context
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcContractLogEntry {
    /// Block number
    pub block_number: u64,
    /// Transaction hash
    pub tx_hash: String,
    /// Transaction index in block
    pub tx_index: u32,
    /// Log index in transaction
    pub log_index: u32,
    /// The log itself
    pub log: RpcContractLog,
}
