//! Contract state storage implementation

use pyrin_contracts_core::ContractAddress;
use borsh::{BorshDeserialize, BorshSerialize};
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Storage key (32 bytes)
pub type StorageKey = [u8; 32];

/// Storage value (32 bytes)
pub type StorageValue = [u8; 32];

/// Zero value for storage
pub const ZERO_VALUE: StorageValue = [0u8; 32];

/// Storage slot for a contract
#[derive(Debug, Clone, Default)]
pub struct StorageSlot {
    /// Current value
    pub value: StorageValue,
    /// Original value (for gas refund calculation)
    pub original: StorageValue,
    /// Whether this slot has been modified
    pub dirty: bool,
}

impl StorageSlot {
    /// Creates a new storage slot with the given value
    pub fn new(value: StorageValue) -> Self {
        Self {
            value,
            original: value,
            dirty: false,
        }
    }

    /// Creates a new empty storage slot
    pub fn empty() -> Self {
        Self::new(ZERO_VALUE)
    }

    /// Sets a new value and marks as dirty
    pub fn set(&mut self, value: StorageValue) {
        if self.value != value {
            self.value = value;
            self.dirty = true;
        }
    }

    /// Returns true if the current value is zero
    pub fn is_zero(&self) -> bool {
        self.value == ZERO_VALUE
    }

    /// Returns true if the original value was zero
    pub fn was_zero(&self) -> bool {
        self.original == ZERO_VALUE
    }
}

/// Contract storage state (in-memory cache)
#[derive(Debug, Clone, Default)]
pub struct ContractState {
    /// Storage slots
    slots: IndexMap<StorageKey, StorageSlot>,
    /// Whether any slot has been modified
    dirty: bool,
}

impl ContractState {
    /// Creates a new empty contract state
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a storage value
    pub fn get(&self, key: &StorageKey) -> StorageValue {
        self.slots
            .get(key)
            .map(|slot| slot.value)
            .unwrap_or(ZERO_VALUE)
    }

    /// Sets a storage value
    pub fn set(&mut self, key: StorageKey, value: StorageValue) {
        if let Some(slot) = self.slots.get_mut(&key) {
            slot.set(value);
        } else {
            let mut slot = StorageSlot::empty();
            slot.set(value);
            self.slots.insert(key, slot);
        }
        self.dirty = true;
    }

    /// Loads a value from persistent storage
    pub fn load(&mut self, key: StorageKey, value: StorageValue) {
        self.slots.insert(key, StorageSlot::new(value));
    }

    /// Returns all dirty slots
    pub fn dirty_slots(&self) -> impl Iterator<Item = (&StorageKey, &StorageSlot)> {
        self.slots.iter().filter(|(_, slot)| slot.dirty)
    }

    /// Returns true if any slot has been modified
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clears the dirty flag on all slots
    pub fn clear_dirty(&mut self) {
        for slot in self.slots.values_mut() {
            slot.original = slot.value;
            slot.dirty = false;
        }
        self.dirty = false;
    }

    /// Reverts all changes to original values
    pub fn revert(&mut self) {
        for slot in self.slots.values_mut() {
            slot.value = slot.original;
            slot.dirty = false;
        }
        self.dirty = false;
    }
}

/// Main contract storage interface
pub struct ContractStorage {
    /// In-memory cache of contract states
    cache: RwLock<HashMap<ContractAddress, ContractState>>,
    /// Warm storage access set (for EIP-2929 style gas accounting)
    warm_slots: RwLock<HashMap<ContractAddress, std::collections::HashSet<StorageKey>>>,
}

impl ContractStorage {
    /// Creates a new contract storage
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            warm_slots: RwLock::new(HashMap::new()),
        }
    }

    /// Reads a storage value
    pub fn read(&self, address: &ContractAddress, key: &StorageKey) -> StorageValue {
        let cache = self.cache.read();
        cache
            .get(address)
            .map(|state| state.get(key))
            .unwrap_or(ZERO_VALUE)
    }

    /// Writes a storage value
    pub fn write(&self, address: &ContractAddress, key: StorageKey, value: StorageValue) {
        let mut cache = self.cache.write();
        let state = cache.entry(*address).or_insert_with(ContractState::new);
        state.set(key, value);
    }

    /// Checks if a storage slot is warm (has been accessed)
    pub fn is_warm(&self, address: &ContractAddress, key: &StorageKey) -> bool {
        let warm = self.warm_slots.read();
        warm.get(address).map(|set| set.contains(key)).unwrap_or(false)
    }

    /// Marks a storage slot as warm
    pub fn mark_warm(&self, address: &ContractAddress, key: StorageKey) {
        let mut warm = self.warm_slots.write();
        warm.entry(*address)
            .or_insert_with(std::collections::HashSet::new)
            .insert(key);
    }

    /// Gets the contract state for an address
    pub fn get_state(&self, address: &ContractAddress) -> Option<ContractState> {
        let cache = self.cache.read();
        cache.get(address).cloned()
    }

    /// Sets the contract state for an address
    pub fn set_state(&self, address: ContractAddress, state: ContractState) {
        let mut cache = self.cache.write();
        cache.insert(address, state);
    }

    /// Commits all pending changes (to be called after successful execution)
    pub fn commit(&self) {
        let mut cache = self.cache.write();
        for state in cache.values_mut() {
            state.clear_dirty();
        }
    }

    /// Reverts all pending changes (to be called after failed execution)
    pub fn rollback(&self) {
        let mut cache = self.cache.write();
        for state in cache.values_mut() {
            state.revert();
        }
    }

    /// Clears the warm storage set (at the start of a new transaction)
    pub fn clear_warm(&self) {
        let mut warm = self.warm_slots.write();
        warm.clear();
    }

    /// Returns all dirty states
    pub fn dirty_states(&self) -> Vec<(ContractAddress, ContractState)> {
        let cache = self.cache.read();
        cache
            .iter()
            .filter(|(_, state)| state.is_dirty())
            .map(|(addr, state)| (*addr, state.clone()))
            .collect()
    }
}

impl Default for ContractStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_slot() {
        let mut slot = StorageSlot::empty();
        assert!(slot.is_zero());
        assert!(slot.was_zero());
        assert!(!slot.dirty);

        slot.set([1u8; 32]);
        assert!(!slot.is_zero());
        assert!(slot.was_zero());
        assert!(slot.dirty);
    }

    #[test]
    fn test_contract_state() {
        let mut state = ContractState::new();
        let key = [1u8; 32];
        
        assert_eq!(state.get(&key), ZERO_VALUE);
        
        state.set(key, [2u8; 32]);
        assert_eq!(state.get(&key), [2u8; 32]);
        assert!(state.is_dirty());
        
        state.revert();
        assert_eq!(state.get(&key), ZERO_VALUE);
    }

    #[test]
    fn test_contract_storage() {
        let storage = ContractStorage::new();
        let addr = ContractAddress::zero();
        let key = [1u8; 32];
        
        assert_eq!(storage.read(&addr, &key), ZERO_VALUE);
        
        storage.write(&addr, key, [2u8; 32]);
        assert_eq!(storage.read(&addr, &key), [2u8; 32]);
        
        storage.rollback();
        assert_eq!(storage.read(&addr, &key), ZERO_VALUE);
    }

    #[test]
    fn test_warm_slots() {
        let storage = ContractStorage::new();
        let addr = ContractAddress::zero();
        let key = [1u8; 32];
        
        assert!(!storage.is_warm(&addr, &key));
        
        storage.mark_warm(&addr, key);
        assert!(storage.is_warm(&addr, &key));
        
        storage.clear_warm();
        assert!(!storage.is_warm(&addr, &key));
    }
}
