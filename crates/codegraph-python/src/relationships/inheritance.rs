use serde::{Deserialize, Serialize};

/// Represents a class inheritance relationship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InheritanceRelation {
    /// Name of the child class
    pub child: String,

    /// Name of the parent class
    pub parent: String,

    /// Line number where inheritance is declared
    pub line: usize,
}

impl InheritanceRelation {
    /// Create a new inheritance relation
    pub fn new(child: impl Into<String>, parent: impl Into<String>, line: usize) -> Self {
        Self {
            child: child.into(),
            parent: parent.into(),
            line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inheritance_relation_new() {
        let inheritance = InheritanceRelation::new("Dog", "Animal", 5);
        assert_eq!(inheritance.child, "Dog");
        assert_eq!(inheritance.parent, "Animal");
        assert_eq!(inheritance.line, 5);
    }
}
