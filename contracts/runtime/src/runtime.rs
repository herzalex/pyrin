//! Main contract runtime

use pyrin_contracts_core::{ContractAddress, ContractError, ContractResult, Gas, ReentrancyGuard, InputValidator};
use pyrin_contracts_storage::{AccountStorage, CodeStorage, ContractStorage};
use pyrin_contracts_vm::{ContractExecutor, ExecutionContext, ExecutionResult};
use std::sync::Arc;
use parking_lot::RwLock;

/// Configuration for the contract runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum gas per block
    pub block_gas_limit: Gas,
    /// Maximum gas per transaction
    pub tx_gas_limit: Gas,
    /// Maximum contract code size
    pub max_code_size: usize,
    /// Maximum call depth
    pub max_call_depth: u32,
    /// Whether to enable contract creation
    pub enable_create: bool,
    /// Whether to enable contract calls
    pub enable_call: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            block_gas_limit: 30_000_000, // 30M gas per block
            tx_gas_limit: 10_000_000,    // 10M gas per tx
            max_code_size: 24 * 1024,    // 24KB
            max_call_depth: 1024,
            enable_create: true,
            enable_call: true,
        }
    }
}

/// Main contract runtime
pub struct ContractRuntime {
    /// Configuration
    config: RuntimeConfig,
    /// Contract storage
    storage: Arc<ContractStorage>,
    /// Code storage
    code: Arc<CodeStorage>,
    /// Account storage
    accounts: Arc<AccountStorage>,
    /// Contract executor
    executor: ContractExecutor,
    /// Current block gas used
    block_gas_used: RwLock<Gas>,
    /// Reentrancy guard for security
    reentrancy_guard: ReentrancyGuard,
}

impl ContractRuntime {
    /// Creates a new contract runtime
    pub fn new(config: RuntimeConfig) -> Self {
        let storage = Arc::new(ContractStorage::new());
        let code = Arc::new(CodeStorage::new());
        let accounts = Arc::new(AccountStorage::new());
        
        let executor = ContractExecutor::new(
            Arc::clone(&storage),
            Arc::clone(&code),
            Arc::clone(&accounts),
        );

        Self {
            config,
            storage,
            code,
            accounts,
            executor,
            block_gas_used: RwLock::new(0),
            reentrancy_guard: ReentrancyGuard::new(),
        }
    }



    /// Returns the configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Deploys a new contract
    pub fn deploy(
        &self,
        deployer: ContractAddress,
        code: Vec<u8>,
        init_data: Vec<u8>,
        value: u64,
        gas_limit: Gas,
        block_info: BlockInfo,
    ) -> ContractResult<DeployResult> {
        if !self.config.enable_create {
            return Err(ContractError::DeploymentFailed("contract creation disabled".to_string()));
        }

        // Check code size
        if code.len() > self.config.max_code_size {
            return Err(ContractError::code_size_exceeded(code.len(), self.config.max_code_size));
        }

        // Check gas limit
        let gas_limit = gas_limit.min(self.config.tx_gas_limit);
        
        // Check block gas limit
        {
            let block_used = *self.block_gas_used.read();
            if block_used + gas_limit > self.config.block_gas_limit {
                return Err(ContractError::out_of_gas(block_used + gas_limit, self.config.block_gas_limit));
            }
        }

        // Create execution context
        let ctx = ExecutionContext::new(
            block_info.number,
            block_info.timestamp,
            block_info.daa_score,
            deployer,
            deployer,
            ContractAddress::zero(), // Will be calculated
            value,
            init_data.clone(),
            gas_limit,
        );

        // Deploy
        let address = self.executor.deploy(&ctx, code, init_data)?;

        // Update block gas used
        {
            let mut block_used = self.block_gas_used.write();
            *block_used += ctx.gas.used();
        }

        Ok(DeployResult {
            address,
            gas_used: ctx.gas.used(),
        })
    }

    /// Calls a contract
    pub fn call(
        &self,
        caller: ContractAddress,
        to: ContractAddress,
        data: Vec<u8>,
        value: u64,
        gas_limit: Gas,
        block_info: BlockInfo,
    ) -> ContractResult<ExecutionResult> {
        if !self.config.enable_call {
            return Err(ContractError::Revert("contract calls disabled".to_string()));
        }

        // Validate input data size
        InputValidator::validate_input_size(&data)?;

        // Check gas limit
        let gas_limit = gas_limit.min(self.config.tx_gas_limit);

        // Check block gas limit
        {
            let block_used = *self.block_gas_used.read();
            if block_used + gas_limit > self.config.block_gas_limit {
                return Err(ContractError::out_of_gas(block_used + gas_limit, self.config.block_gas_limit));
            }
        }

        // Acquire reentrancy lock - this prevents the same contract from being called
        // while it's already executing
        let _lock = self.reentrancy_guard.acquire(&to)?;

        // Create execution context
        let ctx = ExecutionContext::new(
            block_info.number,
            block_info.timestamp,
            block_info.daa_score,
            caller,
            caller,
            to,
            value,
            data,
            gas_limit,
        );

        // Execute
        let result = self.executor.execute(&ctx);

        // Update block gas used
        {
            let mut block_used = self.block_gas_used.write();
            *block_used += result.gas_used;
        }

        // Commit or rollback based on result
        if result.success {
            self.storage.commit();
            self.accounts.commit();
        } else {
            self.storage.rollback();
            self.accounts.rollback();
        }

        Ok(result)
    }

    /// Simulates a call without modifying state
    pub fn static_call(
        &self,
        caller: ContractAddress,
        to: ContractAddress,
        data: Vec<u8>,
        gas_limit: Gas,
        block_info: BlockInfo,
    ) -> ContractResult<ExecutionResult> {
        // Create execution context
        let mut ctx = ExecutionContext::new(
            block_info.number,
            block_info.timestamp,
            block_info.daa_score,
            caller,
            caller,
            to,
            0, // No value in static calls
            data,
            gas_limit,
        );
        ctx.is_static = true;

        // Execute
        let result = self.executor.execute(&ctx);

        // Always rollback for static calls
        self.storage.rollback();
        self.accounts.rollback();

        Ok(result)
    }

    /// Estimates gas for a call
    pub fn estimate_gas(
        &self,
        caller: ContractAddress,
        to: ContractAddress,
        data: Vec<u8>,
        value: u64,
        block_info: BlockInfo,
    ) -> ContractResult<Gas> {
        // Try with max gas
        let result = self.call(
            caller,
            to,
            data,
            value,
            self.config.tx_gas_limit,
            block_info,
        )?;

        if result.success {
            // Add 10% buffer
            Ok(result.gas_used + result.gas_used / 10)
        } else {
            Err(ContractError::Revert(result.error.unwrap_or_default()))
        }
    }

    /// Gets contract code
    pub fn get_code(&self, address: &ContractAddress) -> Option<Vec<u8>> {
        self.code.get(address).map(|c| c.code.clone())
    }

    /// Gets storage value
    pub fn get_storage(&self, address: &ContractAddress, key: &[u8; 32]) -> [u8; 32] {
        self.storage.read(address, key)
    }

    /// Gets account balance
    pub fn get_balance(&self, address: &ContractAddress) -> u64 {
        self.accounts.balance(address)
    }

    /// Gets account nonce
    pub fn get_nonce(&self, address: &ContractAddress) -> u64 {
        self.accounts.nonce(address)
    }

    /// Checks if an address is a contract
    pub fn is_contract(&self, address: &ContractAddress) -> bool {
        self.code.exists(address)
    }

    /// Starts a new block
    pub fn begin_block(&self) {
        let mut block_used = self.block_gas_used.write();
        *block_used = 0;
        self.storage.clear_warm();
    }

    /// Ends a block
    pub fn end_block(&self) {
        // Commit all pending changes
        self.storage.commit();
        self.accounts.commit();
    }

    /// Gets the current block gas used
    pub fn block_gas_used(&self) -> Gas {
        *self.block_gas_used.read()
    }

    /// Gets the remaining block gas
    pub fn block_gas_remaining(&self) -> Gas {
        self.config.block_gas_limit.saturating_sub(*self.block_gas_used.read())
    }
}

impl Default for ContractRuntime {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

/// Block information
#[derive(Debug, Clone, Copy)]
pub struct BlockInfo {
    /// Block number
    pub number: u64,
    /// Block timestamp
    pub timestamp: u64,
    /// DAA score
    pub daa_score: u64,
}

impl BlockInfo {
    /// Creates new block info
    pub fn new(number: u64, timestamp: u64, daa_score: u64) -> Self {
        Self {
            number,
            timestamp,
            daa_score,
        }
    }
}

/// Result of contract deployment
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// Address of the deployed contract
    pub address: ContractAddress,
    /// Gas used for deployment
    pub gas_used: Gas,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = ContractRuntime::default();
        assert_eq!(runtime.config().block_gas_limit, 30_000_000);
        assert_eq!(runtime.block_gas_used(), 0);
    }

    #[test]
    fn test_block_lifecycle() {
        let runtime = ContractRuntime::default();
        
        runtime.begin_block();
        assert_eq!(runtime.block_gas_used(), 0);
        
        // Block gas tracking would happen during execution
        runtime.end_block();
    }

    #[test]
    fn test_is_not_contract() {
        let runtime = ContractRuntime::default();
        let addr = ContractAddress::new([1u8; 20]);
        
        assert!(!runtime.is_contract(&addr));
    }
}
