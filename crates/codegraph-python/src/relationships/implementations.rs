use serde::{Deserialize, Serialize};

/// Represents a protocol/trait implementation relationship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImplementationRelation {
    /// Name of the implementing class
    pub implementer: String,

    /// Name of the trait/protocol being implemented
    pub trait_name: String,

    /// Line number where implementation is declared
    pub line: usize,
}

impl ImplementationRelation {
    /// Create a new implementation relation
    pub fn new(implementer: impl Into<String>, trait_name: impl Into<String>, line: usize) -> Self {
        Self {
            implementer: implementer.into(),
            trait_name: trait_name.into(),
            line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_relation_new() {
        let implementation = ImplementationRelation::new("MyClass", "Serializable", 10);
        assert_eq!(implementation.implementer, "MyClass");
        assert_eq!(implementation.trait_name, "Serializable");
        assert_eq!(implementation.line, 10);
    }
}
