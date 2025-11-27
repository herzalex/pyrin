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
- **ReentrancyGuard** - Protection against reentrancy attacks
- **InputValidator** - Input validation utilities
- **RateLimiter** - Block-level rate limiting

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

## Security Features

### Reentrancy Protection

The runtime implements a reentrancy guard that prevents contracts from being re-entered while still executing. This protects against the famous DAO-style attack:

```rust
use pyrin_contracts_core::ReentrancyGuard;

let guard = ReentrancyGuard::new();
let _lock = guard.acquire(&contract_address)?; // Returns error if already locked
// Contract execution...
// Lock is automatically released when dropped
```

### Input Validation

All inputs are validated for size and format:

- Maximum input data size: 64KB
- Maximum return data size: 64KB  
- Maximum log topics: 4
- Maximum log data size: 16KB

### Rate Limiting

Block-level rate limiting prevents DoS attacks:

```rust
use pyrin_contracts_core::RateLimiter;

let limiter = RateLimiter::new(10_000); // Max 10K ops per block
limiter.try_consume()?; // Returns error when limit reached
limiter.reset(); // Reset at block boundary
```

### Resource Limits

| Resource | Limit |
|----------|-------|
| Maximum call depth | 1024 |
| Maximum memory | 16MB (256 pages × 64KB) |
| Maximum code size | 24KB |
| Block gas limit | 30,000,000 |
| Transaction gas limit | 10,000,000 |

## RPC API

New RPC operations for smart contracts:

| Operation | Description |
|-----------|-------------|
| `DeployContract` | Deploy new contract from WASM bytecode |
| `CallContract` | Execute contract function (state-changing) |
| `EstimateContractGas` | Estimate gas for a call |
| `GetContractCode` | Retrieve contract bytecode |
| `GetContractStorage` | Read storage slot value |
| `GetContractLogs` | Query contract event logs |

## CLI Commands

The `contract` command provides CLI access:

```bash
# Deploy a contract
pyrin-cli contract deploy mycontract.wasm --init-data 0x1234 --gas 1000000

# Call a contract (state-changing)
pyrin-cli contract call 0x... 0x1234 --gas 100000 --value 1000

# View (read-only call)
pyrin-cli contract view 0x... 0x1234

# Get contract bytecode
pyrin-cli contract code 0x...

# Read storage slot
pyrin-cli contract storage 0x... 0x0000...0001

# Query event logs
pyrin-cli contract logs --address 0x... --from 100 --to 200

# Estimate gas
pyrin-cli contract estimate 0x... 0x1234
```

## License

Same as the main Pyrin project.
