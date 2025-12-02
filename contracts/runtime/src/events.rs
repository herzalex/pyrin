//! Contract event handling

use pyrin_contracts_core::{ContractAddress, Log, LogTopic};
use pyrin_hashes::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A contract event with additional context
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ContractEvent {
    /// Block number where the event occurred
    pub block_number: u64,
    /// Transaction hash that emitted the event
    pub tx_hash: Hash,
    /// Transaction index within the block
    pub tx_index: u32,
    /// Log index within the transaction
    pub log_index: u32,
    /// The actual log data
    pub log: Log,
}

impl ContractEvent {
    /// Creates a new contract event
    pub fn new(
        block_number: u64,
        tx_hash: Hash,
        tx_index: u32,
        log_index: u32,
        log: Log,
    ) -> Self {
        Self {
            block_number,
            tx_hash,
            tx_index,
            log_index,
            log,
        }
    }

    /// Returns the contract address that emitted this event
    pub fn address(&self) -> &ContractAddress {
        &self.log.address
    }

    /// Returns the event topics
    pub fn topics(&self) -> &[LogTopic] {
        &self.log.topics
    }

    /// Returns the event data
    pub fn data(&self) -> &[u8] {
        &self.log.data
    }

    /// Returns the event signature (first topic)
    pub fn signature(&self) -> Option<&LogTopic> {
        self.log.event_signature()
    }
}

/// Filter for querying events
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Start block (inclusive)
    pub from_block: Option<u64>,
    /// End block (inclusive)
    pub to_block: Option<u64>,
    /// Contract addresses to filter
    pub addresses: Vec<ContractAddress>,
    /// Topics to filter (each position can have multiple options)
    pub topics: Vec<Vec<LogTopic>>,
}

#[allow(dead_code)]
impl EventFilter {
    /// Creates a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the start block (builder pattern - consumes self)
    pub fn with_from_block(mut self, block: u64) -> Self {
        self.from_block = Some(block);
        self
    }

    /// Sets the end block (builder pattern - consumes self)
    pub fn with_to_block(mut self, block: u64) -> Self {
        self.to_block = Some(block);
        self
    }

    /// Adds an address to filter (builder pattern - consumes self)
    pub fn with_address(mut self, address: ContractAddress) -> Self {
        self.addresses.push(address);
        self
    }

    /// Adds a topic filter for a specific position (builder pattern - consumes self)
    pub fn with_topic(mut self, position: usize, topic: LogTopic) -> Self {
        while self.topics.len() <= position {
            self.topics.push(Vec::new());
        }
        self.topics[position].push(topic);
        self
    }

    /// Checks if an event matches this filter
    pub fn matches(&self, event: &ContractEvent) -> bool {
        // Check block range
        if let Some(from) = self.from_block {
            if event.block_number < from {
                return false;
            }
        }
        if let Some(to) = self.to_block {
            if event.block_number > to {
                return false;
            }
        }

        // Check addresses
        if !self.addresses.is_empty() && !self.addresses.contains(event.address()) {
            return false;
        }

        // Check topics
        for (i, topic_options) in self.topics.iter().enumerate() {
            if topic_options.is_empty() {
                continue; // Empty means match any
            }
            
            match event.topics().get(i) {
                Some(event_topic) => {
                    if !topic_options.contains(event_topic) {
                        return false;
                    }
                }
                None => return false, // Event doesn't have this topic
            }
        }

        true
    }
}

/// Event emitter for collecting events during execution
pub struct EventEmitter {
    /// Pending events
    events: Vec<ContractEvent>,
    /// Current transaction info
    tx_hash: Hash,
    tx_index: u32,
    block_number: u64,
}

impl EventEmitter {
    /// Creates a new event emitter
    pub fn new(block_number: u64, tx_hash: Hash, tx_index: u32) -> Self {
        Self {
            events: Vec::new(),
            tx_hash,
            tx_index,
            block_number,
        }
    }

    /// Emits a log as an event
    pub fn emit(&mut self, log: Log) {
        let log_index = self.events.len() as u32;
        let event = ContractEvent::new(
            self.block_number,
            self.tx_hash,
            self.tx_index,
            log_index,
            log,
        );
        self.events.push(event);
    }

    /// Emits multiple logs
    pub fn emit_all(&mut self, logs: Vec<Log>) {
        for log in logs {
            self.emit(log);
        }
    }

    /// Clears all pending events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Returns all pending events
    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Takes all pending events
    pub fn take_events(&mut self) -> Vec<ContractEvent> {
        std::mem::take(&mut self.events)
    }

    /// Returns the number of pending events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if there are no pending events
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Simple in-memory event store
#[allow(dead_code)]
pub struct EventStore {
    /// Events indexed by block number
    by_block: HashMap<u64, Vec<ContractEvent>>,
    /// Events indexed by address
    by_address: HashMap<ContractAddress, Vec<ContractEvent>>,
}

#[allow(dead_code)]
impl EventStore {
    /// Creates a new event store
    pub fn new() -> Self {
        Self {
            by_block: HashMap::new(),
            by_address: HashMap::new(),
        }
    }

    /// Stores an event
    pub fn store(&mut self, event: ContractEvent) {
        let block = event.block_number;
        let address = *event.address();

        self.by_block.entry(block).or_default().push(event.clone());
        self.by_address.entry(address).or_default().push(event);
    }

    /// Stores multiple events
    pub fn store_all(&mut self, events: Vec<ContractEvent>) {
        for event in events {
            self.store(event);
        }
    }

    /// Gets events for a block
    pub fn get_by_block(&self, block: u64) -> Vec<&ContractEvent> {
        self.by_block
            .get(&block)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Gets events for an address
    pub fn get_by_address(&self, address: &ContractAddress) -> Vec<&ContractEvent> {
        self.by_address
            .get(address)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Queries events with a filter
    pub fn query(&self, filter: &EventFilter) -> Vec<&ContractEvent> {
        // Determine block range
        let from = filter.from_block.unwrap_or(0);
        let to = filter.to_block.unwrap_or(u64::MAX);

        let mut results = Vec::new();
        
        for block in from..=to {
            if let Some(events) = self.by_block.get(&block) {
                for event in events {
                    if filter.matches(event) {
                        results.push(event);
                    }
                }
            }
        }

        results
    }

    /// Returns the total number of events
    pub fn len(&self) -> usize {
        self.by_block.values().map(|v| v.len()).sum()
    }

    /// Returns true if the store is empty
    pub fn is_empty(&self) -> bool {
        self.by_block.is_empty()
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(block: u64, address: ContractAddress) -> ContractEvent {
        ContractEvent::new(
            block,
            Hash::from_bytes([0u8; 32]),
            0,
            0,
            Log::new(address, vec![[1u8; 32]], vec![]),
        )
    }

    #[test]
    fn test_event_filter() {
        let event = create_test_event(100, ContractAddress::new([1u8; 20]));
        
        // Match block range
        let filter = EventFilter::new().with_from_block(50).with_to_block(150);
        assert!(filter.matches(&event));
        
        // Doesn't match block range
        let filter = EventFilter::new().with_from_block(101);
        assert!(!filter.matches(&event));
        
        // Match address
        let filter = EventFilter::new().with_address(ContractAddress::new([1u8; 20]));
        assert!(filter.matches(&event));
        
        // Doesn't match address
        let filter = EventFilter::new().with_address(ContractAddress::new([2u8; 20]));
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_event_emitter() {
        let mut emitter = EventEmitter::new(100, Hash::from_bytes([0u8; 32]), 0);
        
        assert!(emitter.is_empty());
        
        emitter.emit(Log::new(ContractAddress::zero(), vec![], vec![]));
        emitter.emit(Log::new(ContractAddress::zero(), vec![], vec![]));
        
        assert_eq!(emitter.len(), 2);
        
        let events = emitter.take_events();
        assert_eq!(events.len(), 2);
        assert!(emitter.is_empty());
    }

    #[test]
    fn test_event_store() {
        let mut store = EventStore::new();
        
        let addr1 = ContractAddress::new([1u8; 20]);
        let addr2 = ContractAddress::new([2u8; 20]);
        
        store.store(create_test_event(100, addr1));
        store.store(create_test_event(100, addr2));
        store.store(create_test_event(101, addr1));
        
        assert_eq!(store.len(), 3);
        assert_eq!(store.get_by_block(100).len(), 2);
        assert_eq!(store.get_by_address(&addr1).len(), 2);
        
        let filter = EventFilter::new().with_from_block(100).with_to_block(100);
        assert_eq!(store.query(&filter).len(), 2);
    }
}
