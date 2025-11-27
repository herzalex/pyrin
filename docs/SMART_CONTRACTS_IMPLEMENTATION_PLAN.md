# Pyrin Smart Contracts Implementation Plan

## Executive Summary

Dieses Dokument beschreibt einen detaillierten Plan zur Implementierung von Smart Contracts in der Pyrin-Blockchain. Die Analyse des bestehenden Codebase zeigt, dass Pyrin bereits über eine solide Grundlage verfügt, die erweitert werden kann, um Smart Contract-Funktionalität zu unterstützen.

## 1. Analyse der bestehenden Architektur

### 1.1 Kernkomponenten

Die Pyrin-Blockchain basiert auf einer Rust-Implementierung mit folgenden Hauptkomponenten:

| Komponente | Pfad | Beschreibung |
|------------|------|--------------|
| **Consensus** | `/consensus` | Konsens-Engine mit GHOSTDAG-Protokoll |
| **TxScript** | `/crypto/txscript` | Bitcoin-ähnliche Skriptsprache |
| **Core** | `/core` | Grundlegende Datenstrukturen |
| **Wallet** | `/wallet` | Wallet-Funktionalität |
| **RPC** | `/rpc` | Remote Procedure Calls |
| **WASM** | `/wasm` | WebAssembly-Bindings |

### 1.2 Aktuelle Script-Engine

Die aktuelle `txscript`-Engine unterstützt:
- **Stack-basierte Operationen**: OpDup, OpSwap, OpRot, etc.
- **Kryptografische Operationen**: OpSHA256, OpBlake3, OpCheckSig
- **Kontrollfluss**: OpIf, OpElse, OpEndIf, OpReturn
- **Arithmetik**: OpAdd, OpSub, Op1Add, etc.
- **Vergleiche**: OpEqual, OpLessThan, OpGreaterThan
- **Zeitbasierte Locks**: OpCheckLockTimeVerify, OpCheckSequenceVerify

### 1.3 Transaktionsstruktur

```rust
pub struct Transaction {
    pub version: u16,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u64,
    pub subnetwork_id: SubnetworkId,
    pub gas: u64,                    // Bereits vorhanden!
    pub payload: Vec<u8>,            // Kann für Contract-Daten genutzt werden
}
```

**Wichtige Beobachtung**: Das `gas`-Feld und `payload`-Feld sind bereits in der Transaktionsstruktur vorhanden, was auf eine geplante Smart Contract-Unterstützung hindeutet.

### 1.4 Subnetworks

Das Subnetwork-System ermöglicht verschiedene Transaktionstypen:
- `SUBNETWORK_ID_NATIVE` (0): Standard-Transaktionen
- `SUBNETWORK_ID_COINBASE` (1): Coinbase-Transaktionen
- `SUBNETWORK_ID_REGISTRY` (2): Registry für neue Subnetworks

**Dies bietet eine natürliche Erweiterungsmöglichkeit für Smart Contracts.**

---

## 2. Empfohlene Smart Contract-Architektur

### 2.1 Option A: WASM-basierte Smart Contracts (Empfohlen)

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

**Vorteile:**
- Deterministische Ausführung
- Sandboxed und sicher
- Unterstützt mehrere Programmiersprachen (Rust, AssemblyScript, C/C++)
- Hohe Performance
- Pyrin hat bereits WASM-Erfahrung (siehe `/wasm` Verzeichnis)

### 2.2 Option B: Erweiterte TxScript-basierte Contracts

Erweiterung der bestehenden Script-Engine um zusätzliche Opcodes:

```rust
// Neue Opcodes für Smart Contracts
opcode OpContractCreate<0xc0, ...>(self, vm) { ... }
opcode OpContractCall<0xc1, ...>(self, vm) { ... }
opcode OpStateRead<0xc2, ...>(self, vm) { ... }
opcode OpStateWrite<0xc3, ...>(self, vm) { ... }
opcode OpEmitEvent<0xc4, ...>(self, vm) { ... }
opcode OpGetBlockInfo<0xc5, ...>(self, vm) { ... }
opcode OpGetTxInfo<0xc6, ...>(self, vm) { ... }
```

**Vorteile:**
- Einfachere Integration
- Geringerer Implementierungsaufwand
- Bitcoin Script-Kompatibilität bleibt erhalten

---

## 3. Detaillierter Implementierungsplan

### Phase 1: Grundlagen (2-3 Monate)

#### 3.1.1 Contract Storage Layer

**Neue Crate: `/contracts/storage`**

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

**Erweiterung: `/consensus/core/src/subnets.rs`**

```rust
// Neue Subnetwork ID für Contracts
pub const SUBNETWORK_ID_CONTRACT: SubnetworkId = SubnetworkId::from_byte(3);

// Contract-Payload-Struktur
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

**Neue Crate: `/contracts/gas`**

```rust
pub struct GasMeter {
    gas_limit: u64,
    gas_used: u64,
}

impl GasMeter {
    pub fn consume(&mut self, amount: u64) -> Result<(), OutOfGasError>;
    pub fn remaining(&self) -> u64;
}

// Gas-Kosten-Tabelle
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

### Phase 2: WASM VM Integration (2-3 Monate)

#### 3.2.1 WASM Runtime

**Neue Crate: `/contracts/vm`**

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

// Host-Funktionen für Contracts
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
// Token-Interface (ähnlich ERC-20)
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

### Phase 3: Konsensus-Integration (2-3 Monate)

#### 3.3.1 Transaction Validator Erweiterung

**Erweiterung: `/consensus/src/processes/transaction_validator`**

```rust
// Neue Datei: contract_validator.rs
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

#### 3.3.2 Block Processor Erweiterung

```rust
// Erweiterung der Block-Verarbeitung
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

### Phase 4: RPC & SDK (1-2 Monate)

#### 3.4.1 Neue RPC-Methoden

```rust
// Neue RPC-Endpunkte
pub trait ContractRpc {
    async fn deploy_contract(&self, code: Vec<u8>, init_data: Vec<u8>) -> DeployResult;
    async fn call_contract(&self, address: String, function: String, args: Vec<u8>) -> CallResult;
    async fn estimate_gas(&self, tx: ContractTransaction) -> u64;
    async fn get_contract_code(&self, address: String) -> Vec<u8>;
    async fn get_contract_storage(&self, address: String, key: String) -> Vec<u8>;
    async fn get_contract_logs(&self, filter: LogFilter) -> Vec<Log>;
}
```

#### 3.4.2 WASM SDK Erweiterung

```typescript
// TypeScript SDK für Contracts
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

## 4. Dateisystem-Struktur

```
pyrin/
├── contracts/                      # Neue Contract-Module
│   ├── core/                       # Core Contract-Typen
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── address.rs          # Contract-Adressen
│   │       ├── types.rs            # Contract-Typen
│   │       └── abi.rs              # ABI-Definitionen
│   │
│   ├── storage/                    # Contract-Speicher
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs            # State-Management
│   │       └── trie.rs             # Merkle-Patricia-Trie
│   │
│   ├── vm/                         # WASM Virtual Machine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── executor.rs         # Contract-Ausführung
│   │       ├── host.rs             # Host-Funktionen
│   │       └── gas.rs              # Gas-Metering
│   │
│   ├── runtime/                    # Contract-Runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── context.rs          # Execution-Context
│   │       └── events.rs           # Event-System
│   │
│   └── std/                        # Standard-Library für Contracts
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── storage.rs          # Storage-Makros
│           ├── tokens.rs           # Token-Interfaces
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

## 5. Abhängigkeiten

### Neue Cargo-Dependencies

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
sha3 = "0.10"                       # Keccak für Adressen
```

---

## 6. Sicherheitsüberlegungen

### 6.1 Reentrancy-Schutz
```rust
pub struct ReentrancyGuard {
    locked: RefCell<HashSet<ContractAddress>>,
}

impl ReentrancyGuard {
    pub fn enter(&self, addr: &ContractAddress) -> Result<(), ReentrancyError>;
    pub fn exit(&self, addr: &ContractAddress);
}
```

### 6.2 Gas-Limits
- Minimum Gas: 21.000 (wie Ethereum)
- Maximum Gas pro Block: Konfigurierbar
- Gas-Rückerstattung bei unbenutztem Gas

### 6.3 Code-Validierung
```rust
pub fn validate_contract_code(code: &[u8]) -> Result<(), ValidationError> {
    // 1. WASM Modul validieren
    wasmparser::validate(code)?;
    
    // 2. Prüfen auf verbotene Imports
    check_forbidden_imports(code)?;
    
    // 3. Code-Größe prüfen
    if code.len() > MAX_CONTRACT_SIZE {
        return Err(ValidationError::CodeTooLarge);
    }
    
    Ok(())
}
```

---

## 7. Migrations-Strategie

### 7.1 Testnet-Deployment
1. Smart Contract-Feature auf Testnet aktivieren
2. Community-Testing durchführen
3. Bug-Fixes und Optimierungen

### 7.2 Mainnet-Aktivierung
1. Hard-Fork zu einem bestimmten Block/DAA-Score
2. Aktivierung des `SUBNETWORK_ID_CONTRACT`
3. Graduelle Gas-Limit-Erhöhung

### 7.3 Rückwärtskompatibilität
- Bestehende Transaktionen bleiben unverändert
- Smart Contracts sind ein Add-on, kein Ersatz

---

## 8. Zeitplan

| Phase | Beschreibung | Dauer | Abhängigkeiten |
|-------|-------------|-------|----------------|
| 1.1 | Contract Storage Layer | 4 Wochen | - |
| 1.2 | Contract Subnetwork | 2 Wochen | 1.1 |
| 1.3 | Gas Metering | 2 Wochen | - |
| 2.1 | WASM VM Integration | 6 Wochen | 1.1, 1.3 |
| 2.2 | Interface Standards | 2 Wochen | 2.1 |
| 3.1 | Transaction Validator | 4 Wochen | 2.1 |
| 3.2 | Block Processor | 4 Wochen | 3.1 |
| 4.1 | RPC Erweiterungen | 3 Wochen | 3.2 |
| 4.2 | SDK Updates | 3 Wochen | 4.1 |
| 5 | Testing & Audit | 8 Wochen | Alle |

**Gesamtdauer: ~8-12 Monate**

---

## 9. Alternative Ansätze

### 9.1 Move VM (von Sui/Aptos)
- Vorteile: Formale Verifikation, sicher
- Nachteile: Neue Sprache, weniger Entwickler

### 9.2 EVM-Kompatibilität
- Vorteile: Bestehende Tools, große Entwickler-Community
- Nachteile: Komplexer, Legacy-Probleme

### 9.3 Cairo (von Starknet)
- Vorteile: ZK-Proof-freundlich
- Nachteile: Noch sehr neu, begrenzte Tooling

---

## 10. Nächste Schritte

1. **Sofort**: Technische Diskussion im Team
2. **Woche 1-2**: Proof-of-Concept für Storage Layer
3. **Woche 3-4**: WASM VM Evaluation (wasmer vs. wasmtime)
4. **Monat 2**: Erste Integration Tests
5. **Monat 3**: Community-Feedback einholen

---

## Anhang A: Code-Beispiele

### A.1 Einfacher Token-Contract (Rust/WASM)

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

### A.2 Contract-Deployment via CLI

```bash
# Contract kompilieren
cargo build --target wasm32-unknown-unknown --release

# Contract deployen
pyrin-cli contract deploy \
    --code target/wasm32-unknown-unknown/release/token.wasm \
    --init-data '{"name":"MyToken","symbol":"MTK","initial_supply":1000000}'

# Contract aufrufen
pyrin-cli contract call \
    --address pyrin:qp... \
    --function transfer \
    --args '{"to":"pyrin:qr...","amount":100}'
```

---

## Anhang B: Referenzen

1. [Wasmer Documentation](https://docs.wasmer.io/)
2. [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
3. [Near Protocol Contracts](https://docs.near.org/concepts/basics/protocol)
4. [Solana Programs](https://docs.solana.com/developing/on-chain-programs/overview)
5. [Pyrin GitHub Repository](https://github.com/pyrin-network/pyrin)

---

*Dokument erstellt: November 2024*
*Version: 1.0*
