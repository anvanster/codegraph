use serde::{Deserialize, Serialize};

/// Represents trait/interface implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImplementationRelation {
    /// Implementing class
    pub implementor: String,

    /// Trait/interface being implemented
    pub trait_name: String,
}

impl ImplementationRelation {
    pub fn new(implementor: impl Into<String>, trait_name: impl Into<String>) -> Self {
        Self {
            implementor: implementor.into(),
            trait_name: trait_name.into(),
        }
    }
}
