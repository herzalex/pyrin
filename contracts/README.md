# Pyrin Smart Contracts

This directory contains the Smart Contract infrastructure for the Pyrin blockchain.

## Crate Structure

```
contracts/
├── core/       # Core types and definitions
├── storage/    # Persistent storage layer  
├── vm/         # WASM virtual machine
└── runtime/    # Execution runtime
```

## Overview

### pyrin-contracts-core

Core types and definitions for smart contracts:

- **ContractAddress** - 20-byte contract addresses with CREATE/CREATE2 derivation
- **AbiType** - Complete ABI type definitions for function encoding
- **FunctionSelector** - Keccak256-based function selectors
- **GasMeter** - Gas tracking with EIP-3529 refund support
- **ContractError** - Comprehensive error types
- **ContractPayload** - Deploy/Call transaction payloads

### pyrin-contracts-storage

Persistent storage layer for smart contracts:

- **ContractStorage** - State management with warm/cold access tracking
- **CodeStorage** - WASM bytecode storage with validation
- **AccountStorage** - Balance and nonce management

### pyrin-contracts-vm

WASM virtual machine for contract execution:

- **ContractExecutor** - WASM execution using wasmi runtime
- **HostFunctions** - Blockchain interaction (storage, events, crypto)
- **ContractMemory** - Memory management with gas accounting

### pyrin-contracts-runtime

High-level runtime for block processing:

- **ContractRuntime** - Main runtime with block gas limits
- **TransactionProcessor** - Contract transaction processing
- **EventEmitter** - Event system with topic-based filtering

## Usage

```rust
use pyrin_contracts_core::{ContractAddress, GasMeter, ContractPayload};
use pyrin_contracts_storage::{ContractStorage, CodeStorage, AccountStorage};
use pyrin_contracts_vm::ContractExecutor;
use pyrin_contracts_runtime::ContractRuntime;

// Create runtime
let runtime = ContractRuntime::default();

// Deploy a contract
let deploy_tx = ContractTransaction::deploy(
    deployer_address,
    wasm_bytecode,
    init_data,
    gas_limit,
);
let receipt = processor.process(&deploy_tx, &block_info)?;

// Call a contract
let call_tx = ContractTransaction::call(
    caller_address,
    contract_address,
    call_data,
    value,
    gas_limit,
);
let result = processor.process(&call_tx, &block_info)?;
```

## Gas Costs

The gas metering system is EIP-compatible:

| Operation | Base Cost | Per-Unit Cost |
|-----------|-----------|---------------|
| SLOAD (warm) | 100 | - |
| SLOAD (cold) | 2100 | - |
| SSTORE (new) | 20000 | - |
| SSTORE (modify) | 5000 | - |
| SHA3 | 30 | 6 per word |
| LOG | 375 | 8 per byte |
| CALL | 100 | + 2600 if cold |
| CREATE | 32000 | + 200 per byte |

## Testing

Run the test suite:

```bash
cargo test -p pyrin-contracts-core \
           -p pyrin-contracts-storage \
           -p pyrin-contracts-vm \
           -p pyrin-contracts-runtime
```

## Integration

Smart contracts use `SUBNETWORK_ID_CONTRACT` (byte 3) for transaction identification.
Contract transactions include:
- `gas` field for gas limit
- `payload` field for contract data

## Security Considerations

- Maximum call depth: 1024
- Maximum memory: 16MB (256 pages × 64KB)
- Maximum code size: 24KB
- Reentrancy protection via static call tracking
- Gas metering prevents infinite loops

## License

Same as the main Pyrin project.
