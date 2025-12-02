//! Account state storage

use pyrin_contracts_core::ContractAddress;
use pyrin_hashes::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use parking_lot::RwLock;
use std::collections::HashMap;

/// Account state (for contracts and EOAs)
#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct AccountState {
    /// Account nonce (for replay protection)
    pub nonce: u64,
    /// Account balance in sompis
    pub balance: u64,
    /// Code hash (zero for EOAs)
    pub code_hash: Hash,
    /// Storage root (Merkle root of storage trie)
    pub storage_root: Hash,
}

impl AccountState {
    /// Creates a new account state
    pub fn new(nonce: u64, balance: u64, code_hash: Hash, storage_root: Hash) -> Self {
        Self {
            nonce,
            balance,
            code_hash,
            storage_root,
        }
    }

    /// Creates an empty account state
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an EOA (externally owned account) state
    pub fn eoa(balance: u64) -> Self {
        Self {
            nonce: 0,
            balance,
            code_hash: Hash::from_bytes([0u8; 32]),
            storage_root: Hash::from_bytes([0u8; 32]),
        }
    }

    /// Returns true if this is a contract account
    pub fn is_contract(&self) -> bool {
        self.code_hash != Hash::from_bytes([0u8; 32])
    }

    /// Returns true if this account is empty (no balance, no nonce, no code)
    pub fn is_empty(&self) -> bool {
        self.nonce == 0
            && self.balance == 0
            && self.code_hash == Hash::from_bytes([0u8; 32])
    }

    /// Increments the nonce
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
    }

    /// Adds to the balance
    pub fn add_balance(&mut self, amount: u64) {
        self.balance = self.balance.saturating_add(amount);
    }

    /// Subtracts from the balance
    pub fn sub_balance(&mut self, amount: u64) -> Result<(), ()> {
        if self.balance >= amount {
            self.balance -= amount;
            Ok(())
        } else {
            Err(())
        }
    }
}

/// Storage for account states
pub struct AccountStorage {
    /// Account states by address
    accounts: RwLock<HashMap<ContractAddress, AccountState>>,
    /// Pending changes (for transaction rollback)
    pending: RwLock<HashMap<ContractAddress, AccountState>>,
}

impl AccountStorage {
    /// Creates a new account storage
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
        }
    }

    /// Gets an account state
    pub fn get(&self, address: &ContractAddress) -> AccountState {
        // Check pending first
        {
            let pending = self.pending.read();
            if let Some(state) = pending.get(address) {
                return state.clone();
            }
        }

        // Then check committed
        let accounts = self.accounts.read();
        accounts.get(address).cloned().unwrap_or_default()
    }

    /// Updates an account state (pending until commit)
    pub fn update(&self, address: ContractAddress, state: AccountState) {
        let mut pending = self.pending.write();
        pending.insert(address, state);
    }

    /// Checks if an account exists
    pub fn exists(&self, address: &ContractAddress) -> bool {
        // Check pending first
        {
            let pending = self.pending.read();
            if pending.contains_key(address) {
                return !pending.get(address).unwrap().is_empty();
            }
        }

        // Then check committed
        let accounts = self.accounts.read();
        accounts.get(address).map(|a| !a.is_empty()).unwrap_or(false)
    }

    /// Gets the balance of an account
    pub fn balance(&self, address: &ContractAddress) -> u64 {
        self.get(address).balance
    }

    /// Gets the nonce of an account
    pub fn nonce(&self, address: &ContractAddress) -> u64 {
        self.get(address).nonce
    }

    /// Transfers value between accounts
    pub fn transfer(
        &self,
        from: &ContractAddress,
        to: &ContractAddress,
        amount: u64,
    ) -> Result<(), ()> {
        let mut from_state = self.get(from);
        from_state.sub_balance(amount)?;

        let mut to_state = self.get(to);
        to_state.add_balance(amount);

        self.update(*from, from_state);
        self.update(*to, to_state);

        Ok(())
    }

    /// Commits pending changes
    pub fn commit(&self) {
        let mut accounts = self.accounts.write();
        let mut pending = self.pending.write();

        for (address, state) in pending.drain() {
            if state.is_empty() {
                accounts.remove(&address);
            } else {
                accounts.insert(address, state);
            }
        }
    }

    /// Reverts pending changes
    pub fn rollback(&self) {
        let mut pending = self.pending.write();
        pending.clear();
    }

    /// Returns the number of accounts
    pub fn count(&self) -> usize {
        let accounts = self.accounts.read();
        let pending = self.pending.read();
        
        let mut count = accounts.len();
        for (addr, state) in pending.iter() {
            if !accounts.contains_key(addr) && !state.is_empty() {
                count += 1;
            } else if accounts.contains_key(addr) && state.is_empty() {
                count -= 1;
            }
        }
        count
    }

    /// Checks if an address is a contract
    pub fn is_contract(&self, address: &ContractAddress) -> bool {
        self.get(address).is_contract()
    }
}

impl Default for AccountStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_state() {
        let mut state = AccountState::eoa(1000);
        
        assert!(!state.is_contract());
        assert!(!state.is_empty());
        assert_eq!(state.balance, 1000);
        
        state.increment_nonce();
        assert_eq!(state.nonce, 1);
        
        state.add_balance(500);
        assert_eq!(state.balance, 1500);
        
        state.sub_balance(200).unwrap();
        assert_eq!(state.balance, 1300);
        
        assert!(state.sub_balance(2000).is_err());
    }

    #[test]
    fn test_account_storage() {
        let storage = AccountStorage::new();
        let addr = ContractAddress::new([1u8; 20]);
        
        assert!(!storage.exists(&addr));
        assert_eq!(storage.balance(&addr), 0);
        
        storage.update(addr, AccountState::eoa(1000));
        
        assert!(storage.exists(&addr));
        assert_eq!(storage.balance(&addr), 1000);
        
        // Not committed yet
        storage.rollback();
        assert!(!storage.exists(&addr));
        
        // Now commit
        storage.update(addr, AccountState::eoa(1000));
        storage.commit();
        assert!(storage.exists(&addr));
    }

    #[test]
    fn test_transfer() {
        let storage = AccountStorage::new();
        let alice = ContractAddress::new([1u8; 20]);
        let bob = ContractAddress::new([2u8; 20]);
        
        storage.update(alice, AccountState::eoa(1000));
        storage.commit();
        
        storage.transfer(&alice, &bob, 300).unwrap();
        storage.commit();
        
        assert_eq!(storage.balance(&alice), 700);
        assert_eq!(storage.balance(&bob), 300);
    }
}
