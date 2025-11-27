//! Pyrin Smart Contracts - Runtime Environment
//!
//! This crate provides the complete runtime for smart contract execution:
//! - Transaction processing
//! - State management
//! - Event handling
//! - Integration with consensus

mod runtime;
mod transaction;
mod events;

pub use runtime::{ContractRuntime, RuntimeConfig};
pub use transaction::{ContractTransaction, TransactionProcessor};
pub use events::{EventEmitter, ContractEvent};
