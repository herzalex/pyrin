//! Contract transaction processing

use pyrin_contracts_core::{
    ContractAddress, ContractCall, ContractDeploy, ContractError, 
    ContractPayload, ContractReceipt, ContractResult, Gas,
};
use pyrin_hashes::Hash;
use crate::runtime::{BlockInfo, ContractRuntime};

/// A processed contract transaction
#[derive(Debug, Clone)]
pub struct ContractTransaction {
    /// Transaction hash
    pub hash: Hash,
    /// Sender address
    pub from: ContractAddress,
    /// Transaction nonce
    pub nonce: u64,
    /// Gas price (in sompis per gas)
    pub gas_price: u64,
    /// Gas limit
    pub gas_limit: Gas,
    /// Contract payload
    pub payload: ContractPayload,
}

impl ContractTransaction {
    /// Creates a new deployment transaction
    pub fn deploy(
        hash: Hash,
        from: ContractAddress,
        nonce: u64,
        gas_price: u64,
        gas_limit: Gas,
        code: Vec<u8>,
        init_data: Vec<u8>,
        value: u64,
    ) -> Self {
        Self {
            hash,
            from,
            nonce,
            gas_price,
            gas_limit,
            payload: ContractPayload::Deploy(ContractDeploy::new(code, init_data, gas_limit, value)),
        }
    }

    /// Creates a new call transaction
    pub fn call(
        hash: Hash,
        from: ContractAddress,
        nonce: u64,
        gas_price: u64,
        gas_limit: Gas,
        to: ContractAddress,
        data: Vec<u8>,
        value: u64,
    ) -> Self {
        Self {
            hash,
            from,
            nonce,
            gas_price,
            gas_limit,
            payload: ContractPayload::Call(ContractCall::new(to, data, gas_limit, value)),
        }
    }

    /// Returns the target address (None for deployments)
    pub fn to(&self) -> Option<ContractAddress> {
        match &self.payload {
            ContractPayload::Deploy(_) => None,
            ContractPayload::Call(call) => Some(call.to),
        }
    }

    /// Returns the value being sent
    pub fn value(&self) -> u64 {
        match &self.payload {
            ContractPayload::Deploy(d) => d.value,
            ContractPayload::Call(c) => c.value,
        }
    }

    /// Returns the maximum gas fee
    pub fn max_fee(&self) -> u64 {
        self.gas_price.saturating_mul(self.gas_limit)
    }

    /// Validates the transaction
    pub fn validate(&self, sender_balance: u64, sender_nonce: u64) -> ContractResult<()> {
        // Check nonce
        if self.nonce != sender_nonce {
            return Err(ContractError::InvalidArguments(format!(
                "Invalid nonce: expected {}, got {}",
                sender_nonce, self.nonce
            )));
        }

        // Check balance
        let required = self.max_fee().saturating_add(self.value());
        if sender_balance < required {
            return Err(ContractError::insufficient_balance(required, sender_balance));
        }

        // Check gas limit
        if self.gas_limit == 0 {
            return Err(ContractError::InvalidArguments("Gas limit cannot be zero".to_string()));
        }

        Ok(())
    }
}

/// Transaction processor for contract transactions
pub struct TransactionProcessor {
    /// The contract runtime
    runtime: ContractRuntime,
}

impl TransactionProcessor {
    /// Creates a new transaction processor
    pub fn new(runtime: ContractRuntime) -> Self {
        Self { runtime }
    }

    /// Processes a contract transaction
    pub fn process(
        &self,
        tx: &ContractTransaction,
        block_info: BlockInfo,
    ) -> ContractResult<ContractReceipt> {
        // Validate the transaction
        let sender_balance = self.runtime.get_balance(&tx.from);
        let sender_nonce = self.runtime.get_nonce(&tx.from);
        tx.validate(sender_balance, sender_nonce)?;

        // Execute based on payload type
        match &tx.payload {
            ContractPayload::Deploy(deploy) => {
                self.process_deploy(tx, deploy, block_info)
            }
            ContractPayload::Call(call) => {
                self.process_call(tx, call, block_info)
            }
        }
    }

    /// Processes a deployment transaction
    fn process_deploy(
        &self,
        tx: &ContractTransaction,
        deploy: &ContractDeploy,
        block_info: BlockInfo,
    ) -> ContractResult<ContractReceipt> {
        let result = self.runtime.deploy(
            tx.from,
            deploy.code.clone(),
            deploy.init_data.clone(),
            deploy.value,
            tx.gas_limit,
            block_info,
        );

        match result {
            Ok(deploy_result) => {
                // Calculate actual fee
                let fee = tx.gas_price.saturating_mul(deploy_result.gas_used);
                
                Ok(ContractReceipt::success(
                    tx.hash,
                    deploy_result.gas_used,
                    Some(deploy_result.address),
                    Vec::new(),
                    Vec::new(),
                ))
            }
            Err(e) => {
                // Charge all gas on deployment failure
                Ok(ContractReceipt::failed(tx.hash, tx.gas_limit, e.to_string()))
            }
        }
    }

    /// Processes a call transaction
    fn process_call(
        &self,
        tx: &ContractTransaction,
        call: &ContractCall,
        block_info: BlockInfo,
    ) -> ContractResult<ContractReceipt> {
        let result = self.runtime.call(
            tx.from,
            call.to,
            call.data.clone(),
            call.value,
            tx.gas_limit,
            block_info,
        )?;

        if result.success {
            Ok(ContractReceipt::success(
                tx.hash,
                result.gas_used,
                None,
                result.return_data,
                result.logs,
            ))
        } else if result.error.as_ref().map(|e| e.contains("reverted")).unwrap_or(false) {
            Ok(ContractReceipt::reverted(
                tx.hash,
                result.gas_used,
                result.error.unwrap_or_default(),
                result.logs,
            ))
        } else {
            Ok(ContractReceipt::failed(
                tx.hash,
                result.gas_used,
                result.error.unwrap_or_default(),
            ))
        }
    }

    /// Processes a batch of transactions
    pub fn process_batch(
        &self,
        txs: &[ContractTransaction],
        block_info: BlockInfo,
    ) -> Vec<ContractResult<ContractReceipt>> {
        self.runtime.begin_block();
        
        let results: Vec<_> = txs
            .iter()
            .map(|tx| self.process(tx, block_info))
            .collect();
        
        self.runtime.end_block();
        results
    }

    /// Returns a reference to the runtime
    pub fn runtime(&self) -> &ContractRuntime {
        &self.runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_processor() -> TransactionProcessor {
        TransactionProcessor::new(ContractRuntime::default())
    }

    #[test]
    fn test_contract_transaction() {
        let tx = ContractTransaction::call(
            Hash::from_bytes([0u8; 32]),
            ContractAddress::new([1u8; 20]),
            0,
            1,
            100_000,
            ContractAddress::new([2u8; 20]),
            vec![0xa9, 0x05, 0x9c, 0xbb], // transfer selector
            1000,
        );

        assert_eq!(tx.to(), Some(ContractAddress::new([2u8; 20])));
        assert_eq!(tx.value(), 1000);
        assert_eq!(tx.max_fee(), 100_000);
    }

    #[test]
    fn test_deploy_transaction() {
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let tx = ContractTransaction::deploy(
            Hash::from_bytes([0u8; 32]),
            ContractAddress::new([1u8; 20]),
            0,
            1,
            500_000,
            wasm,
            vec![],
            0,
        );

        assert!(tx.to().is_none());
        assert!(tx.payload.is_deploy());
    }

    #[test]
    fn test_validate_nonce() {
        let tx = ContractTransaction::call(
            Hash::from_bytes([0u8; 32]),
            ContractAddress::new([1u8; 20]),
            5, // Wrong nonce
            1,
            100_000,
            ContractAddress::new([2u8; 20]),
            vec![],
            0,
        );

        let result = tx.validate(1_000_000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_balance() {
        let tx = ContractTransaction::call(
            Hash::from_bytes([0u8; 32]),
            ContractAddress::new([1u8; 20]),
            0,
            1,
            100_000,
            ContractAddress::new([2u8; 20]),
            vec![],
            50_000, // Value + gas exceeds balance
        );

        let result = tx.validate(100_000, 0);
        assert!(result.is_err());
    }
}
