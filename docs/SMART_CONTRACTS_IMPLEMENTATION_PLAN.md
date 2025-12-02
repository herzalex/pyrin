# Pyrin Smart Contracts Implementation Plan

## Executive Summary

This document describes a detailed plan for implementing Smart Contracts in the Pyrin blockchain. The analysis of the existing codebase shows that Pyrin already has a solid foundation that can be extended to support Smart Contract functionality.

The transaction structure already includes a `gas` field and a `payload` field, indicating planned Smart Contract support. The subnetwork system provides a natural extension point for Smart Contracts.

## 1. Analysis of Existing Architecture

### 1.1 Core Components

The Pyrin blockchain is based on a Rust implementation with the following main components:

| Component | Path | Description |
|-----------|------|-------------|
| **Consensus** | `/consensus` | Consensus engine with GHOSTDAG protocol |
| **TxScript** | `/crypto/txscript` | Bitcoin-like script language |
| **Core** | `/core` | Basic data structures |
| **Wallet** | `/wallet` | Wallet functionality |
| **RPC** | `/rpc` | Remote Procedure Calls |
| **WASM** | `/wasm` | WebAssembly bindings |

### 1.2 Current Script Engine

The current `txscript` engine supports:
- **Stack-based operations**: OpDup, OpSwap, OpRot, etc.
- **Cryptographic operations**: OpSHA256, OpBlake3, OpCheckSig
- **Control flow**: OpIf, OpElse, OpEndIf, OpReturn
- **Arithmetic**: OpAdd, OpSub, Op1Add, etc.
- **Comparisons**: OpEqual, OpLessThan, OpGreaterThan
- **Time-based locks**: OpCheckLockTimeVerify, OpCheckSequenceVerify

### 1.3 Transaction Structure

```rust
pub struct Transaction {
    pub version: u16,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u64,
    pub subnetwork_id: SubnetworkId,
    pub gas: u64,                    // Already present!
    pub payload: Vec<u8>,            // Can be used for contract data
}
```

**Important observation**: The `gas` field and `payload` field are already present in the transaction structure, indicating planned Smart Contract support.

### 1.4 Subnetworks

The subnetwork system enables different transaction types:
- `SUBNETWORK_ID_NATIVE` (0): Standard transactions
- `SUBNETWORK_ID_COINBASE` (1): Coinbase transactions
- `SUBNETWORK_ID_REGISTRY` (2): Registry for new subnetworks

**This provides a natural extension point for Smart Contracts.**

---

## 2. Recommended Smart Contract Architecture

### 2.1 Option A: WASM-based Smart Contracts (Recommended)

```
┌──────────────────────────────────────────────────────────────┐
│                        Pyrin Node                            │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   RPC API   │  │   Wallet    │  │   Contract API      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Contract Execution Layer                   │ │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────────────┐   │ │
│  │  │ WASM VM   │  │  State    │  │  Contract Storage │   │ │
│  │  │ (wasmer)  │  │  Manager  │  │  (RocksDB)        │   │ │
│  │  └───────────┘  └───────────┘  └───────────────────┘   │ │
│  └─────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                 Consensus Layer                         │ │
│  │  ┌───────────────┐  ┌────────────┐  ┌───────────────┐  │ │
│  │  │   GHOSTDAG    │  │ Transaction│  │   Block       │  │ │
│  │  │   Protocol    │  │  Validator │  │   Processor   │  │ │
│  │  └───────────────┘  └────────────┘  └───────────────┘  │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**Advantages:**
- Deterministic execution
- Sandboxed and secure
- Supports multiple programming languages (Rust, AssemblyScript, C/C++)
- High performance
- Pyrin already has WASM experience (see `/wasm` directory)

### 2.2 Option B: Extended TxScript-based Contracts

Extension of the existing script engine with additional opcodes:

> **Note**: The opcodes 0xc0-0xc5 shown below currently map to OpUnknown192-OpUnknown197 in the existing codebase. These would need to be repurposed via a hard-fork. Alternative: use opcodes in the 0xf0-0xf9 range which are also currently undefined.

```rust
// New opcodes for Smart Contracts (using existing macro pattern)
// These would replace the current OpUnknown opcodes at these positions
opcode OpContractCreate<0xc0, u32>(self, vm) {
    // Deploy new contract code
    let code = vm.dstack.pop()?;
    let init_data = vm.dstack.pop()?;
    let address = deploy_contract(code, init_data)?;
    vm.dstack.push(address.to_vec());
    Ok(())
}

opcode OpContractCall<0xc1, u8>(self, vm) {
    // Call existing contract
    let address = vm.dstack.pop()?;
    let data = vm.dstack.pop()?;
    let gas = vm.dstack.pop_item::<u64>()?;
    let result = call_contract(address, data, gas)?;
    vm.dstack.push(result);
    Ok(())
}

opcode OpStateRead<0xc2, 1>(self, vm) {
    let key = vm.dstack.pop()?;
    let value = read_state(key)?;
    vm.dstack.push(value);
    Ok(())
}

opcode OpStateWrite<0xc3, 1>(self, vm) {
    let key = vm.dstack.pop()?;
    let value = vm.dstack.pop()?;
    write_state(key, value)?;
    Ok(())
}

opcode OpEmitEvent<0xc4, 1>(self, vm) {
    let topics_count = vm.dstack.pop_item::<i32>()?;
    let topics: Vec<_> = (0..topics_count).map(|_| vm.dstack.pop()).collect::<Result<_,_>>()?;
    let data = vm.dstack.pop()?;
    emit_event(topics, data)?;
    Ok(())
}

opcode OpGetBlockInfo<0xc5, 1>(self, vm) {
    let info_type = vm.dstack.pop_item::<u8>()?;
    let info = get_block_info(info_type)?;
    vm.dstack.push(info);
    Ok(())
}
```

**Advantages:**
- Simpler integration with existing code
- Lower implementation effort
- Bitcoin Script compatibility remains intact

---

## 3. Detailed Implementation Plan

### Phase 1: Foundations (2-3 Months)

#### 3.1.1 Contract Storage Layer

**New Crate: `/contracts/storage`**

```rust
// contracts/storage/src/lib.rs
pub struct ContractStorage {
    db: Arc<RocksDB>,
    cache: LruCache<ContractAddress, ContractState>,
}

pub struct ContractState {
    pub code_hash: Hash,
    pub storage: BTreeMap<[u8; 32], [u8; 32]>,
    pub balance: u64,
    pub nonce: u64,
}

impl ContractStorage {
    pub fn get_storage(&self, addr: &ContractAddress, key: &[u8; 32]) -> Option<[u8; 32]>;
    pub fn set_storage(&mut self, addr: &ContractAddress, key: [u8; 32], value: [u8; 32]);
    pub fn get_code(&self, addr: &ContractAddress) -> Option<Vec<u8>>;
    pub fn deploy_contract(&mut self, code: Vec<u8>) -> ContractAddress;
}
```

#### 3.1.2 Contract Subnetwork

**Extension: `/consensus/core/src/subnets.rs`**

> **Note**: The current SubnetworkId validation in `subnets.rs` only allows bytes 0 and 1. Adding SUBNETWORK_ID_CONTRACT (byte 3) requires updating the `TryFrom` and `FromStr` implementations to accept this new value. This change would be part of the hard-fork activation.

```rust
// New Subnetwork ID for Contracts
pub const SUBNETWORK_ID_CONTRACT: SubnetworkId = SubnetworkId::from_byte(3);

// Update validation to include the new subnetwork:
// In TryFrom<&[u8]> and FromStr implementations:
// if bytes != Self::from_byte(0).0 && bytes != Self::from_byte(1).0 && bytes != Self::from_byte(3).0 {
//     Err(Self::Error::InvalidBytes)
// }

// Contract Payload Structure
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractPayload {
    Deploy {
        code: Vec<u8>,
        init_data: Vec<u8>,
    },
    Call {
        contract_address: ContractAddress,
        function: String,
        args: Vec<u8>,
    },
}
```

#### 3.1.3 Gas Metering

**New Crate: `/contracts/gas`**

```rust
pub struct GasMeter {
    gas_limit: u64,
    gas_used: u64,
}

impl GasMeter {
    pub fn consume(&mut self, amount: u64) -> Result<(), OutOfGasError>;
    pub fn remaining(&self) -> u64;
}

// Gas cost table
pub const GAS_COSTS: GasCostTable = GasCostTable {
    storage_read: 200,
    storage_write: 5000,
    storage_create: 20000,
    memory_grow: 1,
    call_base: 700,
    call_data_byte: 16,
    log_base: 375,
    log_topic: 375,
    log_data_byte: 8,
};
```

### Phase 2: WASM VM Integration (2-3 Months)

#### 3.2.1 WASM Runtime

**New Crate: `/contracts/vm`**

```rust
use wasmer::{Store, Module, Instance, Memory};

pub struct ContractVM {
    store: Store,
    runtime_functions: HostFunctions,
}

impl ContractVM {
    pub fn new() -> Self;
    
    pub fn execute(
        &mut self,
        contract: &Contract,
        function: &str,
        args: &[u8],
        context: ExecutionContext,
    ) -> Result<Vec<u8>, ContractError>;
}

// Host functions for contracts
pub struct HostFunctions {
    pub fn storage_read(key: [u8; 32]) -> [u8; 32];
    pub fn storage_write(key: [u8; 32], value: [u8; 32]);
    pub fn emit_event(topics: Vec<[u8; 32]>, data: Vec<u8>);
    pub fn call_contract(addr: [u8; 32], data: Vec<u8>, gas: u64) -> Vec<u8>;
    pub fn get_caller() -> [u8; 32];
    pub fn get_value() -> u64;
    pub fn get_block_number() -> u64;
    pub fn get_timestamp() -> u64;
}
```

#### 3.2.2 Contract Interface Standard (PRC-20 / PRC-721)

```rust
// Token Interface (similar to ERC-20)
pub trait PRC20 {
    fn name(&self) -> String;
    fn symbol(&self) -> String;
    fn decimals(&self) -> u8;
    fn total_supply(&self) -> u128;
    fn balance_of(&self, owner: Address) -> u128;
    fn transfer(&mut self, to: Address, amount: u128) -> bool;
    fn approve(&mut self, spender: Address, amount: u128) -> bool;
    fn transfer_from(&mut self, from: Address, to: Address, amount: u128) -> bool;
}
```

### Phase 3: Consensus Integration (2-3 Months)

#### 3.3.1 Transaction Validator Extension

**Extension: `/consensus/src/processes/transaction_validator`**

```rust
// New file: contract_validator.rs
pub struct ContractValidator {
    vm: ContractVM,
    storage: ContractStorage,
}

impl ContractValidator {
    pub fn validate_contract_tx(
        &self,
        tx: &Transaction,
        utxos: &[UtxoEntry],
    ) -> Result<ContractResult, ContractError> {
        // 1. Parse payload
        let payload: ContractPayload = borsh::from_slice(&tx.payload)?;
        
        // 2. Check gas limit
        if tx.gas < MIN_GAS_LIMIT {
            return Err(ContractError::InsufficientGas);
        }
        
        // 3. Execute contract
        match payload {
            ContractPayload::Deploy { code, init_data } => {
                self.deploy_contract(tx, code, init_data)
            }
            ContractPayload::Call { contract_address, function, args } => {
                self.call_contract(tx, contract_address, function, args)
            }
        }
    }
}
```

#### 3.3.2 Block Processor Extension

```rust
// Extension of block processing
impl BlockProcessor {
    pub fn process_contract_transactions(
        &mut self,
        block: &Block,
    ) -> Result<Vec<ContractReceipt>, ProcessingError> {
        let mut receipts = Vec::new();
        
        for tx in &block.transactions {
            if tx.subnetwork_id == SUBNETWORK_ID_CONTRACT {
                let receipt = self.contract_validator.validate_contract_tx(tx)?;
                receipts.push(receipt);
                
                // Apply state changes
                self.contract_storage.apply_changes(receipt.state_changes)?;
            }
        }
        
        Ok(receipts)
    }
}
```

### Phase 4: RPC & SDK (1-2 Months)

#### 3.4.1 New RPC Methods

```rust
// New RPC endpoints
pub trait ContractRpc {
    async fn deploy_contract(&self, code: Vec<u8>, init_data: Vec<u8>) -> DeployResult;
    async fn call_contract(&self, address: String, function: String, args: Vec<u8>) -> CallResult;
    async fn estimate_gas(&self, tx: ContractTransaction) -> u64;
    async fn get_contract_code(&self, address: String) -> Vec<u8>;
    async fn get_contract_storage(&self, address: String, key: String) -> Vec<u8>;
    async fn get_contract_logs(&self, filter: LogFilter) -> Vec<Log>;
}
```

#### 3.4.2 WASM SDK Extension

```typescript
// TypeScript SDK for Contracts
interface ContractSDK {
    deployContract(code: Uint8Array, initData: Uint8Array): Promise<DeployResult>;
    callContract(address: string, abi: ABI, method: string, args: any[]): Promise<any>;
    estimateGas(tx: ContractTransaction): Promise<bigint>;
    getContractCode(address: string): Promise<Uint8Array>;
    getStorage(address: string, slot: string): Promise<Uint8Array>;
    getLogs(filter: LogFilter): Promise<Log[]>;
}
```

---

## 4. File System Structure

```
pyrin/
├── contracts/                      # New Contract Modules
│   ├── core/                       # Core Contract Types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── address.rs          # Contract Addresses
│   │       ├── types.rs            # Contract Types
│   │       └── abi.rs              # ABI Definitions
│   │
│   ├── storage/                    # Contract Storage
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs            # State Management
│   │       └── trie.rs             # Merkle-Patricia-Trie
│   │
│   ├── vm/                         # WASM Virtual Machine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── executor.rs         # Contract Execution
│   │       ├── host.rs             # Host Functions
│   │       └── gas.rs              # Gas Metering
│   │
│   ├── runtime/                    # Contract Runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── context.rs          # Execution Context
│   │       └── events.rs           # Event System
│   │
│   └── std/                        # Standard Library for Contracts
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── storage.rs          # Storage macros
│           ├── tokens.rs           # Token interfaces
│           └── prelude.rs          # Common Imports
│
├── consensus/
│   └── core/
│       └── src/
│           └── subnets.rs          # +SUBNETWORK_ID_CONTRACT
│
├── crypto/
│   └── txscript/
│       └── src/
│           └── opcodes/
│               └── mod.rs          # +Contract-Opcodes (optional)
│
└── rpc/
    └── core/
        └── src/
            └── api/
                └── contracts.rs    # Contract RPC API
```

---

## 5. Dependencies

### New Cargo Dependencies

```toml
# contracts/vm/Cargo.toml
[dependencies]
wasmer = "4.2"                      # WASM Runtime
wasmer-compiler-cranelift = "4.2"   # JIT Compiler

# contracts/storage/Cargo.toml
[dependencies]
patricia-trie = "0.5"               # Merkle Patricia Trie
rlp = "0.5"                         # RLP Encoding

# contracts/core/Cargo.toml
[dependencies]
sha3 = "0.10"                       # Keccak for addresses
```

---

## 6. Security Considerations

### 6.1 Reentrancy Protection
```rust
pub struct ReentrancyGuard {
    locked: RefCell<HashSet<ContractAddress>>,
}

impl ReentrancyGuard {
    pub fn enter(&self, addr: &ContractAddress) -> Result<(), ReentrancyError>;
    pub fn exit(&self, addr: &ContractAddress);
}
```

### 6.2 Gas Limits
- Minimum Gas: 21,000 (like Ethereum)
- Maximum Gas per Block: Configurable
- Gas refund for unused gas

### 6.3 Code Validation
```rust
pub fn validate_contract_code(code: &[u8]) -> Result<(), ValidationError> {
    // 1. Validate WASM module
    wasmparser::validate(code)?;
    
    // 2. Check for forbidden imports
    check_forbidden_imports(code)?;
    
    // 3. Check code size
    if code.len() > MAX_CONTRACT_SIZE {
        return Err(ValidationError::CodeTooLarge);
    }
    
    Ok(())
}
```

---

## 7. Migration Strategy

### 7.1 Testnet Deployment
1. Activate Smart Contract feature on testnet
2. Conduct community testing
3. Bug fixes and optimizations

### 7.2 Mainnet Activation
1. Hard-fork at a specific block/DAA score
2. Activation of `SUBNETWORK_ID_CONTRACT`
3. Gradual gas limit increase

### 7.3 Backward Compatibility
- Existing transactions remain unchanged
- Smart Contracts are an add-on, not a replacement

---

## 8. Timeline

| Phase | Description | Duration | Dependencies |
|-------|-------------|----------|--------------|
| 1.1 | Contract Storage Layer | 4 weeks | - |
| 1.2 | Contract Subnetwork | 2 weeks | 1.1 |
| 1.3 | Gas Metering | 2 weeks | - |
| 2.1 | WASM VM Integration | 6 weeks | 1.1, 1.3 |
| 2.2 | Interface Standards | 2 weeks | 2.1 |
| 3.1 | Transaction Validator | 4 weeks | 2.1 |
| 3.2 | Block Processor | 4 weeks | 3.1 |
| 4.1 | RPC Extensions | 3 weeks | 3.2 |
| 4.2 | SDK Updates | 3 weeks | 4.1 |
| 5 | Testing & Audit | 8 weeks | All |

**Total Duration: ~8-12 months**

---

## 9. Alternative Approaches

### 9.1 Move VM (from Sui/Aptos)
- Advantages: Formal verification, secure
- Disadvantages: New language, fewer developers

### 9.2 EVM Compatibility
- Advantages: Existing tools, large developer community
- Disadvantages: More complex, legacy issues

### 9.3 Cairo (from Starknet)
- Advantages: ZK-proof friendly
- Disadvantages: Very new, limited tooling

---

## 10. Next Steps

1. **Immediately**: Technical discussion in the team
2. **Week 1-2**: Proof-of-concept for Storage Layer
3. **Week 3-4**: WASM VM evaluation (wasmer vs. wasmtime)
4. **Month 2**: First integration tests
5. **Month 3**: Gather community feedback

---

## Appendix A: Code Examples

### A.1 Simple Token Contract (Rust/WASM)

```rust
#![no_std]

use pyrin_contract_std::*;

#[pyrin_contract]
pub struct Token {
    balances: Storage<Address, u128>,
    total_supply: Storage<u128>,
    name: Storage<String>,
    symbol: Storage<String>,
}

#[pyrin_methods]
impl Token {
    #[init]
    pub fn new(name: String, symbol: String, initial_supply: u128) {
        self.name.set(name);
        self.symbol.set(symbol);
        self.total_supply.set(initial_supply);
        self.balances.insert(caller(), initial_supply);
    }
    
    pub fn transfer(&mut self, to: Address, amount: u128) -> bool {
        let sender = caller();
        let sender_balance = self.balances.get(&sender).unwrap_or(0);
        
        require!(sender_balance >= amount, "Insufficient balance");
        
        self.balances.insert(sender, sender_balance - amount);
        let to_balance = self.balances.get(&to).unwrap_or(0);
        self.balances.insert(to, to_balance + amount);
        
        emit!(Transfer { from: sender, to, amount });
        
        true
    }
    
    pub fn balance_of(&self, owner: Address) -> u128 {
        self.balances.get(&owner).unwrap_or(0)
    }
}
```

### A.2 Contract Deployment via CLI

```bash
# Compile contract
cargo build --target wasm32-unknown-unknown --release

# Deploy contract
pyrin-cli contract deploy \
    --code target/wasm32-unknown-unknown/release/token.wasm \
    --init-data '{"name":"MyToken","symbol":"MTK","initial_supply":1000000}'

# Call contract
pyrin-cli contract call \
    --address pyrin:qp... \
    --function transfer \
    --args '{"to":"pyrin:qr...","amount":100}'
```

---

## Appendix B: References

1. [Wasmer Documentation](https://docs.wasmer.io/)
2. [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
3. [Near Protocol Contracts](https://docs.near.org/concepts/basics/protocol)
4. [Solana Programs](https://docs.solana.com/developing/on-chain-programs/overview)
5. [Pyrin GitHub Repository](https://github.com/pyrin-network/pyrin)

---

*Document created: November 2024*
*Version: 1.0*
