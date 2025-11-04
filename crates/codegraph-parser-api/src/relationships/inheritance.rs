use serde::{Deserialize, Serialize};

/// Represents class inheritance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InheritanceRelation {
    /// Child class
    pub child: String,

    /// Parent class
    pub parent: String,

    /// Inheritance order (for multiple inheritance)
    pub order: usize,
}

impl InheritanceRelation {
    pub fn new(child: impl Into<String>, parent: impl Into<String>) -> Self {
        Self {
            child: child.into(),
            parent: parent.into(),
            order: 0,
        }
    }

    pub fn with_order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }
}
