//! Pyrin Smart Contracts - Core Types and Interfaces
//!
//! This crate provides the fundamental types and interfaces for the Pyrin
//! Smart Contract system, including:
//! - Contract addresses
//! - ABI definitions
//! - Error types
//! - Gas metering primitives

mod address;
mod abi;
mod error;
mod gas;
mod types;

pub use address::ContractAddress;
pub use abi::{AbiType, FunctionSelector, ContractAbi};
pub use error::{ContractError, ContractResult};
pub use gas::{Gas, GasMeter, GasCosts};
pub use types::{ContractPayload, ContractCall, ContractDeploy, ContractReceipt, Log, LogTopic};
