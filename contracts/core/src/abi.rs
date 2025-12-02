//! Contract ABI (Application Binary Interface) definitions

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// ABI type definitions for contract function parameters and return values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiType {
    /// Unsigned 8-bit integer
    Uint8,
    /// Unsigned 16-bit integer
    Uint16,
    /// Unsigned 32-bit integer
    Uint32,
    /// Unsigned 64-bit integer
    Uint64,
    /// Unsigned 128-bit integer
    Uint128,
    /// Unsigned 256-bit integer
    Uint256,
    /// Signed 8-bit integer
    Int8,
    /// Signed 16-bit integer
    Int16,
    /// Signed 32-bit integer
    Int32,
    /// Signed 64-bit integer
    Int64,
    /// Signed 128-bit integer
    Int128,
    /// Signed 256-bit integer
    Int256,
    /// Boolean
    Bool,
    /// Contract or user address (20 bytes)
    Address,
    /// Fixed-size byte array
    Bytes(usize),
    /// Dynamic byte array
    DynamicBytes,
    /// UTF-8 string
    String,
    /// Fixed-size array of a type
    Array(Box<AbiType>, usize),
    /// Dynamic array of a type
    DynamicArray(Box<AbiType>),
    /// Tuple of types (stored as JSON for simplicity)
    Tuple(Vec<String>),
}

impl AbiType {
    /// Returns the size in bytes for fixed-size types, None for dynamic types
    pub fn fixed_size(&self) -> Option<usize> {
        match self {
            AbiType::Uint8 | AbiType::Int8 | AbiType::Bool => Some(1),
            AbiType::Uint16 | AbiType::Int16 => Some(2),
            AbiType::Uint32 | AbiType::Int32 => Some(4),
            AbiType::Uint64 | AbiType::Int64 => Some(8),
            AbiType::Uint128 | AbiType::Int128 => Some(16),
            AbiType::Uint256 | AbiType::Int256 => Some(32),
            AbiType::Address => Some(20),
            AbiType::Bytes(n) => Some(*n),
            AbiType::Array(inner, count) => inner.fixed_size().map(|s| s * count),
            AbiType::DynamicBytes | AbiType::String | AbiType::DynamicArray(_) => None,
            AbiType::Tuple(types) => {
                // For tuple with string types, we can't compute size
                // This is simplified; full impl would parse type strings
                if types.is_empty() {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }

    /// Returns true if this is a dynamic type
    pub fn is_dynamic(&self) -> bool {
        self.fixed_size().is_none()
    }
}

/// 4-byte function selector (first 4 bytes of keccak256(signature))
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionSelector([u8; 4]);

impl FunctionSelector {
    /// Creates a new function selector from raw bytes
    pub fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Computes a function selector from a function signature
    /// 
    /// Example: "transfer(address,uint256)" -> 0xa9059cbb
    pub fn from_signature(signature: &str) -> Self {
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let result = hasher.finalize();
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&result[0..4]);
        Self(selector)
    }

    /// Returns the raw bytes
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Converts to a hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl std::fmt::Display for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.to_hex())
    }
}

/// Represents a function in a contract's ABI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiFunction {
    /// Function name
    pub name: String,
    /// Function selector (computed from signature)
    pub selector: FunctionSelector,
    /// Input parameter types
    pub inputs: Vec<AbiType>,
    /// Output types
    pub outputs: Vec<AbiType>,
    /// Whether this function is payable
    pub payable: bool,
    /// Whether this function is view-only (doesn't modify state)
    pub view: bool,
}

impl AbiFunction {
    /// Creates a new ABI function definition
    pub fn new(name: &str, inputs: Vec<AbiType>, outputs: Vec<AbiType>, payable: bool, view: bool) -> Self {
        let signature = Self::compute_signature(name, &inputs);
        let selector = FunctionSelector::from_signature(&signature);
        
        Self {
            name: name.to_string(),
            selector,
            inputs,
            outputs,
            payable,
            view,
        }
    }

    /// Computes the function signature string
    fn compute_signature(name: &str, inputs: &[AbiType]) -> String {
        let params: Vec<String> = inputs.iter().map(Self::type_to_string).collect();
        format!("{}({})", name, params.join(","))
    }

    /// Converts an ABI type to its canonical string representation
    fn type_to_string(t: &AbiType) -> String {
        match t {
            AbiType::Uint8 => "uint8".to_string(),
            AbiType::Uint16 => "uint16".to_string(),
            AbiType::Uint32 => "uint32".to_string(),
            AbiType::Uint64 => "uint64".to_string(),
            AbiType::Uint128 => "uint128".to_string(),
            AbiType::Uint256 => "uint256".to_string(),
            AbiType::Int8 => "int8".to_string(),
            AbiType::Int16 => "int16".to_string(),
            AbiType::Int32 => "int32".to_string(),
            AbiType::Int64 => "int64".to_string(),
            AbiType::Int128 => "int128".to_string(),
            AbiType::Int256 => "int256".to_string(),
            AbiType::Bool => "bool".to_string(),
            AbiType::Address => "address".to_string(),
            AbiType::Bytes(n) => format!("bytes{}", n),
            AbiType::DynamicBytes => "bytes".to_string(),
            AbiType::String => "string".to_string(),
            AbiType::Array(inner, count) => format!("{}[{}]", Self::type_to_string(inner), count),
            AbiType::DynamicArray(inner) => format!("{}[]", Self::type_to_string(inner)),
            AbiType::Tuple(types) => {
                // types are already string representations
                format!("({})", types.join(","))
            }
        }
    }
}

/// Represents an event in a contract's ABI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiEvent {
    /// Event name
    pub name: String,
    /// Event topic (keccak256 of signature)
    pub topic: [u8; 32],
    /// Whether each parameter is indexed
    pub indexed: Vec<bool>,
    /// Parameter types
    pub inputs: Vec<AbiType>,
}

/// Complete contract ABI
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractAbi {
    /// Contract functions
    pub functions: Vec<AbiFunction>,
    /// Contract events
    pub events: Vec<AbiEvent>,
    /// Constructor parameters (if any)
    pub constructor: Option<Vec<String>>,
}

impl ContractAbi {
    /// Creates a new empty ABI
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a function to the ABI
    pub fn add_function(&mut self, func: AbiFunction) {
        self.functions.push(func);
    }

    /// Adds an event to the ABI
    pub fn add_event(&mut self, event: AbiEvent) {
        self.events.push(event);
    }

    /// Finds a function by its selector
    pub fn find_function(&self, selector: &FunctionSelector) -> Option<&AbiFunction> {
        self.functions.iter().find(|f| &f.selector == selector)
    }

    /// Finds a function by name
    pub fn find_function_by_name(&self, name: &str) -> Option<&AbiFunction> {
        self.functions.iter().find(|f| f.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_selector() {
        // Known ERC-20 function selectors
        let transfer = FunctionSelector::from_signature("transfer(address,uint256)");
        assert_eq!(transfer.to_hex(), "a9059cbb");

        let balance_of = FunctionSelector::from_signature("balanceOf(address)");
        assert_eq!(balance_of.to_hex(), "70a08231");
    }

    #[test]
    fn test_abi_function() {
        let func = AbiFunction::new(
            "transfer",
            vec![AbiType::Address, AbiType::Uint256],
            vec![AbiType::Bool],
            false,
            false,
        );
        
        assert_eq!(func.name, "transfer");
        assert_eq!(func.selector.to_hex(), "a9059cbb");
    }

    #[test]
    fn test_abi_type_size() {
        assert_eq!(AbiType::Uint8.fixed_size(), Some(1));
        assert_eq!(AbiType::Uint256.fixed_size(), Some(32));
        assert_eq!(AbiType::Address.fixed_size(), Some(20));
        assert_eq!(AbiType::String.fixed_size(), None);
        assert!(AbiType::String.is_dynamic());
    }
}
