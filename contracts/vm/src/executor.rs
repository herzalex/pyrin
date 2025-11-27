//! Contract executor - WASM runtime for contract execution

use pyrin_contracts_core::{ContractAddress, ContractError, ContractResult, Gas, GasMeter};
use pyrin_contracts_storage::{AccountStorage, CodeStorage, ContractStorage};
use std::sync::Arc;
use parking_lot::RwLock;

/// Maximum call depth for nested contract calls
pub const MAX_CALL_DEPTH: u32 = 1024;

/// Maximum memory pages (64KB each) a contract can use
pub const MAX_MEMORY_PAGES: u32 = 256; // 16MB

/// Execution context passed to contracts
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current block number
    pub block_number: u64,
    /// Current block timestamp
    pub block_timestamp: u64,
    /// Block's DAA score
    pub daa_score: u64,
    /// Transaction origin (original sender)
    pub origin: ContractAddress,
    /// Current caller (may differ from origin in nested calls)
    pub caller: ContractAddress,
    /// Current contract address
    pub address: ContractAddress,
    /// Value sent with this call (in sompis)
    pub value: u64,
    /// Call data
    pub data: Vec<u8>,
    /// Gas meter
    pub gas: Arc<GasMeter>,
    /// Current call depth
    pub depth: u32,
    /// Whether this is a static call (read-only)
    pub is_static: bool,
}

impl ExecutionContext {
    /// Creates a new execution context
    pub fn new(
        block_number: u64,
        block_timestamp: u64,
        daa_score: u64,
        origin: ContractAddress,
        caller: ContractAddress,
        address: ContractAddress,
        value: u64,
        data: Vec<u8>,
        gas_limit: Gas,
    ) -> Self {
        Self {
            block_number,
            block_timestamp,
            daa_score,
            origin,
            caller,
            address,
            value,
            data,
            gas: Arc::new(GasMeter::new(gas_limit)),
            depth: 0,
            is_static: false,
        }
    }

    /// Creates a child context for a nested call
    pub fn child(
        &self,
        caller: ContractAddress,
        address: ContractAddress,
        value: u64,
        data: Vec<u8>,
        gas_limit: Gas,
        is_static: bool,
    ) -> ContractResult<Self> {
        if self.depth >= MAX_CALL_DEPTH {
            return Err(ContractError::CallDepthExceeded(self.depth));
        }

        Ok(Self {
            block_number: self.block_number,
            block_timestamp: self.block_timestamp,
            daa_score: self.daa_score,
            origin: self.origin,
            caller,
            address,
            value,
            data,
            gas: Arc::new(self.gas.child(gas_limit)),
            depth: self.depth + 1,
            is_static: is_static || self.is_static,
        })
    }

    /// Returns true if modifications are allowed
    pub fn can_modify(&self) -> bool {
        !self.is_static
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> Gas {
        self.gas.remaining()
    }

    /// Consumes gas
    pub fn consume_gas(&self, amount: Gas) -> ContractResult<()> {
        self.gas.consume(amount)
    }
}

/// Result of contract execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Gas used
    pub gas_used: Gas,
    /// Return data
    pub return_data: Vec<u8>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Logs emitted
    pub logs: Vec<pyrin_contracts_core::Log>,
}

impl ExecutionResult {
    /// Creates a successful result
    pub fn success(gas_used: Gas, return_data: Vec<u8>, logs: Vec<pyrin_contracts_core::Log>) -> Self {
        Self {
            success: true,
            gas_used,
            return_data,
            error: None,
            logs,
        }
    }

    /// Creates a failed result
    pub fn failure(gas_used: Gas, error: String) -> Self {
        Self {
            success: false,
            gas_used,
            return_data: Vec::new(),
            error: Some(error),
            logs: Vec::new(),
        }
    }

    /// Creates a revert result
    pub fn revert(gas_used: Gas, return_data: Vec<u8>) -> Self {
        Self {
            success: false,
            gas_used,
            return_data,
            error: Some("execution reverted".to_string()),
            logs: Vec::new(),
        }
    }
}

/// Contract executor using wasmi WASM runtime
pub struct ContractExecutor {
    /// Contract storage
    storage: Arc<ContractStorage>,
    /// Code storage
    code: Arc<CodeStorage>,
    /// Account storage
    accounts: Arc<AccountStorage>,
    /// Logs collected during execution
    logs: RwLock<Vec<pyrin_contracts_core::Log>>,
}

impl ContractExecutor {
    /// Creates a new contract executor
    pub fn new(
        storage: Arc<ContractStorage>,
        code: Arc<CodeStorage>,
        accounts: Arc<AccountStorage>,
    ) -> Self {
        Self {
            storage,
            code,
            accounts,
            logs: RwLock::new(Vec::new()),
        }
    }

    /// Executes a contract call
    pub fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult {
        // Check if contract exists
        let contract = match self.code.get(&ctx.address) {
            Some(c) => c,
            None => {
                return ExecutionResult::failure(
                    ctx.gas.used(),
                    format!("Contract not found: {}", ctx.address),
                );
            }
        };

        // Validate the contract code
        if let Err(e) = contract.validate() {
            return ExecutionResult::failure(ctx.gas.used(), format!("Invalid contract: {}", e));
        }

        // Execute the contract
        match self.execute_wasm(ctx, &contract.code) {
            Ok(return_data) => {
                let logs = std::mem::take(&mut *self.logs.write());
                ExecutionResult::success(ctx.gas.used(), return_data, logs)
            }
            Err(e) => {
                // Clear logs on failure
                self.logs.write().clear();
                ExecutionResult::failure(ctx.gas.used(), e.to_string())
            }
        }
    }

    /// Executes WASM bytecode
    fn execute_wasm(&self, ctx: &ExecutionContext, code: &[u8]) -> ContractResult<Vec<u8>> {
        // Parse WASM module
        let engine = wasmi::Engine::default();
        let module = wasmi::Module::new(&engine, code)
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Create linker with host functions
        let mut linker = wasmi::Linker::new(&engine);
        self.register_host_functions(&mut linker)?;

        // Create store with execution context
        let mut store = wasmi::Store::new(&engine, ctx.clone());

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| ContractError::WasmError(e.to_string()))?
            .start(&mut store)
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Find and call the entry function
        let func = instance
            .get_typed_func::<(), i32>(&store, "call")
            .or_else(|_| instance.get_typed_func::<(), i32>(&store, "main"))
            .map_err(|e| ContractError::FunctionNotFound(e.to_string()))?;

        // Execute
        let result = func.call(&mut store, ())
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Get return data from memory if needed
        if result == 0 {
            Ok(Vec::new())
        } else {
            // Return data would be read from WASM memory
            // For now, return empty
            Ok(Vec::new())
        }
    }

    /// Registers host functions with the linker
    fn register_host_functions<T>(&self, linker: &mut wasmi::Linker<T>) -> ContractResult<()> {
        // Storage functions
        linker
            .func_wrap("env", "storage_read", |_caller: wasmi::Caller<T>, key_ptr: i32, value_ptr: i32| -> i32 {
                // Implementation would read from storage
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        linker
            .func_wrap("env", "storage_write", |_caller: wasmi::Caller<T>, key_ptr: i32, value_ptr: i32| -> i32 {
                // Implementation would write to storage
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Caller/value functions
        linker
            .func_wrap("env", "get_caller", |_caller: wasmi::Caller<T>, ptr: i32| -> i32 {
                // Implementation would return caller address
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        linker
            .func_wrap("env", "get_value", |_caller: wasmi::Caller<T>| -> i64 {
                // Implementation would return sent value
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Block info functions
        linker
            .func_wrap("env", "get_block_number", |_caller: wasmi::Caller<T>| -> i64 {
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        linker
            .func_wrap("env", "get_timestamp", |_caller: wasmi::Caller<T>| -> i64 {
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Logging
        linker
            .func_wrap("env", "log", |_caller: wasmi::Caller<T>, data_ptr: i32, data_len: i32, topics_ptr: i32, topics_count: i32| -> i32 {
                // Implementation would emit a log
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Contract calls
        linker
            .func_wrap("env", "call", |_caller: wasmi::Caller<T>, addr_ptr: i32, value: i64, data_ptr: i32, data_len: i32, gas: i64| -> i32 {
                // Implementation would call another contract
                0
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        // Revert
        linker
            .func_wrap("env", "revert", |_caller: wasmi::Caller<T>, data_ptr: i32, data_len: i32| {
                // Implementation would trigger a revert
            })
            .map_err(|e| ContractError::WasmError(e.to_string()))?;

        Ok(())
    }

    /// Deploys a new contract
    pub fn deploy(
        &self,
        ctx: &ExecutionContext,
        code: Vec<u8>,
        init_data: Vec<u8>,
    ) -> ContractResult<ContractAddress> {
        // Calculate contract address
        let deployer_nonce = self.accounts.nonce(&ctx.caller);
        let address = ContractAddress::create(ctx.caller.as_ref(), deployer_nonce);

        // Check if address is already taken
        if self.code.exists(&address) {
            return Err(ContractError::ContractExists(address.to_string()));
        }

        // Validate code
        let contract_code = pyrin_contracts_storage::ContractCode::new(
            code.clone(),
            ctx.caller,
            ctx.block_number,
        );
        contract_code.validate().map_err(|e| ContractError::InvalidBytecode(e))?;

        // Store the code
        self.code.store(address, contract_code);

        // Create account for contract
        let account_state = pyrin_contracts_storage::AccountState::new(
            0,
            ctx.value,
            pyrin_contracts_storage::ContractCode::compute_hash(&code),
            pyrin_hashes::Hash::from_bytes([0u8; 32]),
        );
        self.accounts.update(address, account_state);

        // Increment deployer's nonce
        let mut deployer_state = self.accounts.get(&ctx.caller);
        deployer_state.increment_nonce();
        self.accounts.update(ctx.caller, deployer_state);

        // Execute constructor if init_data is provided
        if !init_data.is_empty() {
            let init_ctx = ctx.child(
                ctx.caller,
                address,
                0,
                init_data,
                ctx.remaining_gas(),
                false,
            )?;
            
            let result = self.execute(&init_ctx);
            if !result.success {
                // Rollback deployment
                self.code.remove(&address);
                self.accounts.rollback();
                return Err(ContractError::DeploymentFailed(
                    result.error.unwrap_or_else(|| "constructor failed".to_string())
                ));
            }
        }

        Ok(address)
    }

    /// Adds a log entry
    pub fn emit_log(&self, log: pyrin_contracts_core::Log) {
        self.logs.write().push(log);
    }

    /// Gets the storage interface
    pub fn storage(&self) -> &Arc<ContractStorage> {
        &self.storage
    }

    /// Gets the code storage interface
    pub fn code_storage(&self) -> &Arc<CodeStorage> {
        &self.code
    }

    /// Gets the account storage interface
    pub fn account_storage(&self) -> &Arc<AccountStorage> {
        &self.accounts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context() {
        let ctx = ExecutionContext::new(
            100,
            1234567890,
            50,
            ContractAddress::zero(),
            ContractAddress::zero(),
            ContractAddress::new([1u8; 20]),
            0,
            vec![],
            100_000,
        );

        assert_eq!(ctx.block_number, 100);
        assert_eq!(ctx.depth, 0);
        assert!(ctx.can_modify());
    }

    #[test]
    fn test_child_context() {
        let ctx = ExecutionContext::new(
            100,
            1234567890,
            50,
            ContractAddress::zero(),
            ContractAddress::zero(),
            ContractAddress::new([1u8; 20]),
            0,
            vec![],
            100_000,
        );

        let child = ctx.child(
            ContractAddress::new([1u8; 20]),
            ContractAddress::new([2u8; 20]),
            100,
            vec![0x01, 0x02],
            50_000,
            false,
        ).unwrap();

        assert_eq!(child.depth, 1);
        assert_eq!(child.caller, ContractAddress::new([1u8; 20]));
        assert_eq!(child.address, ContractAddress::new([2u8; 20]));
    }

    #[test]
    fn test_execution_result() {
        let success = ExecutionResult::success(50_000, vec![0x01], vec![]);
        assert!(success.success);
        assert!(success.error.is_none());

        let failure = ExecutionResult::failure(100_000, "out of gas".to_string());
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }
}
