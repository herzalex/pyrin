//! Pyrin Smart Contracts - WASM Virtual Machine
//!
//! This crate provides the execution engine for smart contracts:
//! - WASM runtime (using wasmi for deterministic execution)
//! - Host functions for blockchain interaction
//! - Memory management
//! - Gas metering during execution

mod executor;
mod host;
mod memory;

pub use executor::{ContractExecutor, ExecutionContext, ExecutionResult};
pub use host::HostFunctions;
pub use memory::ContractMemory;
