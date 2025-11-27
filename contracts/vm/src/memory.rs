//! Contract memory management

use pyrin_contracts_core::{ContractError, ContractResult, Gas, GasMeter};
use std::sync::Arc;

/// Maximum memory size in bytes (16MB)
pub const MAX_MEMORY_SIZE: usize = 16 * 1024 * 1024;

/// Page size (64KB as per WebAssembly spec)
pub const PAGE_SIZE: usize = 64 * 1024;

/// Contract memory for WASM execution
pub struct ContractMemory {
    /// Memory data
    data: Vec<u8>,
    /// Current size in pages
    pages: u32,
    /// Maximum pages
    max_pages: u32,
    /// Gas meter for charging memory expansion costs
    gas: Arc<GasMeter>,
}

impl ContractMemory {
    /// Creates new contract memory with initial pages
    pub fn new(initial_pages: u32, max_pages: u32, gas: Arc<GasMeter>) -> ContractResult<Self> {
        let initial_pages = initial_pages.min(max_pages);
        let initial_size = (initial_pages as usize) * PAGE_SIZE;
        
        if initial_size > MAX_MEMORY_SIZE {
            return Err(ContractError::InvalidMemoryAccess {
                offset: 0,
                size: initial_size as u64,
            });
        }

        Ok(Self {
            data: vec![0u8; initial_size],
            pages: initial_pages,
            max_pages,
            gas,
        })
    }

    /// Creates memory with default settings
    pub fn default_with_gas(gas: Arc<GasMeter>) -> ContractResult<Self> {
        Self::new(1, 256, gas) // 1 page initial, 256 pages max (16MB)
    }

    /// Returns the current size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns the current size in pages
    pub fn pages(&self) -> u32 {
        self.pages
    }

    /// Grows memory by the specified number of pages
    pub fn grow(&mut self, additional_pages: u32) -> ContractResult<u32> {
        let new_pages = self.pages.saturating_add(additional_pages);
        
        if new_pages > self.max_pages {
            return Ok(u32::MAX); // Failure indicator per WASM spec
        }

        let new_size = (new_pages as usize) * PAGE_SIZE;
        if new_size > MAX_MEMORY_SIZE {
            return Ok(u32::MAX);
        }

        // Charge gas for memory expansion
        let current_size = self.data.len() as u64;
        let expansion_cost = self.gas.costs().memory_expansion_cost(current_size, new_size as u64);
        self.gas.consume(expansion_cost)?;

        // Expand memory
        let old_pages = self.pages;
        self.data.resize(new_size, 0);
        self.pages = new_pages;

        Ok(old_pages)
    }

    /// Reads a byte from memory
    pub fn read_byte(&self, offset: u32) -> ContractResult<u8> {
        let offset = offset as usize;
        if offset >= self.data.len() {
            return Err(ContractError::InvalidMemoryAccess {
                offset: offset as u64,
                size: 1,
            });
        }
        Ok(self.data[offset])
    }

    /// Writes a byte to memory
    pub fn write_byte(&mut self, offset: u32, value: u8) -> ContractResult<()> {
        let offset = offset as usize;
        if offset >= self.data.len() {
            return Err(ContractError::InvalidMemoryAccess {
                offset: offset as u64,
                size: 1,
            });
        }
        self.data[offset] = value;
        Ok(())
    }

    /// Reads bytes from memory
    pub fn read(&self, offset: u32, size: u32) -> ContractResult<&[u8]> {
        let offset = offset as usize;
        let size = size as usize;
        let end = offset.checked_add(size).ok_or(ContractError::InvalidMemoryAccess {
            offset: offset as u64,
            size: size as u64,
        })?;

        if end > self.data.len() {
            return Err(ContractError::InvalidMemoryAccess {
                offset: offset as u64,
                size: size as u64,
            });
        }

        Ok(&self.data[offset..end])
    }

    /// Writes bytes to memory
    pub fn write(&mut self, offset: u32, data: &[u8]) -> ContractResult<()> {
        let offset = offset as usize;
        let end = offset.checked_add(data.len()).ok_or(ContractError::InvalidMemoryAccess {
            offset: offset as u64,
            size: data.len() as u64,
        })?;

        if end > self.data.len() {
            return Err(ContractError::InvalidMemoryAccess {
                offset: offset as u64,
                size: data.len() as u64,
            });
        }

        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// Reads a 32-bit little-endian integer
    pub fn read_u32_le(&self, offset: u32) -> ContractResult<u32> {
        let bytes = self.read(offset, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Writes a 32-bit little-endian integer
    pub fn write_u32_le(&mut self, offset: u32, value: u32) -> ContractResult<()> {
        self.write(offset, &value.to_le_bytes())
    }

    /// Reads a 64-bit little-endian integer
    pub fn read_u64_le(&self, offset: u32) -> ContractResult<u64> {
        let bytes = self.read(offset, 8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Writes a 64-bit little-endian integer
    pub fn write_u64_le(&mut self, offset: u32, value: u64) -> ContractResult<()> {
        self.write(offset, &value.to_le_bytes())
    }

    /// Copies data within memory
    pub fn copy_within(&mut self, src: u32, dst: u32, len: u32) -> ContractResult<()> {
        let src = src as usize;
        let dst = dst as usize;
        let len = len as usize;

        // Validate source
        if src.checked_add(len).map_or(true, |end| end > self.data.len()) {
            return Err(ContractError::InvalidMemoryAccess {
                offset: src as u64,
                size: len as u64,
            });
        }

        // Validate destination
        if dst.checked_add(len).map_or(true, |end| end > self.data.len()) {
            return Err(ContractError::InvalidMemoryAccess {
                offset: dst as u64,
                size: len as u64,
            });
        }

        // Charge gas for copy
        let words = (len + 31) / 32;
        let cost = (words as Gas) * self.gas.costs().copy_per_word;
        self.gas.consume(cost)?;

        // Perform copy
        self.data.copy_within(src..src + len, dst);
        Ok(())
    }

    /// Fills memory with a value
    pub fn fill(&mut self, offset: u32, value: u8, len: u32) -> ContractResult<()> {
        let offset = offset as usize;
        let len = len as usize;
        let end = offset.checked_add(len).ok_or(ContractError::InvalidMemoryAccess {
            offset: offset as u64,
            size: len as u64,
        })?;

        if end > self.data.len() {
            return Err(ContractError::InvalidMemoryAccess {
                offset: offset as u64,
                size: len as u64,
            });
        }

        self.data[offset..end].fill(value);
        Ok(())
    }

    /// Returns a reference to the raw memory data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns a mutable reference to the raw memory data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_memory() -> ContractMemory {
        ContractMemory::new(1, 16, Arc::new(GasMeter::new(1_000_000))).unwrap()
    }

    #[test]
    fn test_memory_creation() {
        let mem = create_memory();
        assert_eq!(mem.pages(), 1);
        assert_eq!(mem.size(), PAGE_SIZE);
    }

    #[test]
    fn test_memory_grow() {
        let mut mem = create_memory();
        let old_pages = mem.grow(2).unwrap();
        
        assert_eq!(old_pages, 1);
        assert_eq!(mem.pages(), 3);
        assert_eq!(mem.size(), 3 * PAGE_SIZE);
    }

    #[test]
    fn test_read_write() {
        let mut mem = create_memory();
        
        mem.write(0, &[1, 2, 3, 4]).unwrap();
        let read = mem.read(0, 4).unwrap();
        assert_eq!(read, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_u32_le() {
        let mut mem = create_memory();
        
        mem.write_u32_le(0, 0x12345678).unwrap();
        let read = mem.read_u32_le(0).unwrap();
        assert_eq!(read, 0x12345678);
    }

    #[test]
    fn test_u64_le() {
        let mut mem = create_memory();
        
        mem.write_u64_le(0, 0x123456789ABCDEF0).unwrap();
        let read = mem.read_u64_le(0).unwrap();
        assert_eq!(read, 0x123456789ABCDEF0);
    }

    #[test]
    fn test_copy_within() {
        let mut mem = create_memory();
        
        mem.write(0, &[1, 2, 3, 4]).unwrap();
        mem.copy_within(0, 10, 4).unwrap();
        
        let read = mem.read(10, 4).unwrap();
        assert_eq!(read, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_fill() {
        let mut mem = create_memory();
        
        mem.fill(0, 0xAB, 10).unwrap();
        let read = mem.read(0, 10).unwrap();
        assert!(read.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_out_of_bounds() {
        let mem = create_memory();
        
        let result = mem.read(PAGE_SIZE as u32, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_grow() {
        let mut mem = ContractMemory::new(1, 2, Arc::new(GasMeter::new(1_000_000))).unwrap();
        
        // Should succeed
        let result = mem.grow(1).unwrap();
        assert_eq!(result, 1);
        
        // Should fail (would exceed max)
        let result = mem.grow(1).unwrap();
        assert_eq!(result, u32::MAX);
    }
}
