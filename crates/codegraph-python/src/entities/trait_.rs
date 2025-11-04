use serde::{Deserialize, Serialize};

/// Represents a Python Protocol or Abstract Base Class (trait-like entity)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitEntity {
    /// Trait/Protocol name
    pub name: String,

    /// Visibility: "public" or "private"
    pub visibility: String,

    /// Starting line number (1-indexed)
    pub line_start: usize,

    /// Ending line number (1-indexed)
    pub line_end: usize,

    /// Is this a Protocol (typing.Protocol)?
    pub is_protocol: bool,

    /// Is this an Abstract Base Class (abc.ABC)?
    pub is_abc: bool,

    /// Method signatures required by this trait
    pub required_methods: Vec<String>,

    /// Documentation string (docstring)
    pub doc_comment: Option<String>,

    /// Decorators applied
    pub attributes: Vec<String>,
}

impl TraitEntity {
    /// Create a new trait entity
    pub fn new(name: impl Into<String>, line_start: usize, line_end: usize) -> Self {
        let name = name.into();
        let visibility = if name.starts_with('_') {
            "private".to_string()
        } else {
            "public".to_string()
        };

        Self {
            name,
            visibility,
            line_start,
            line_end,
            is_protocol: false,
            is_abc: false,
            required_methods: Vec::new(),
            doc_comment: None,
            attributes: Vec::new(),
        }
    }

    /// Set as a Protocol
    pub fn set_protocol(mut self, is_protocol: bool) -> Self {
        self.is_protocol = is_protocol;
        self
    }

    /// Set as an ABC
    pub fn set_abc(mut self, is_abc: bool) -> Self {
        self.is_abc = is_abc;
        self
    }

    /// Add a required method
    pub fn add_required_method(mut self, method: impl Into<String>) -> Self {
        self.required_methods.push(method.into());
        self
    }

    /// Set the docstring
    pub fn set_doc_comment(mut self, doc: Option<String>) -> Self {
        self.doc_comment = doc;
        self
    }

    /// Add a decorator
    pub fn add_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attributes.push(attr.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_entity_new() {
        let trait_entity = TraitEntity::new("Serializable", 1, 10);
        assert_eq!(trait_entity.name, "Serializable");
        assert_eq!(trait_entity.visibility, "public");
        assert!(!trait_entity.is_protocol);
        assert!(!trait_entity.is_abc);
    }

    #[test]
    fn test_trait_entity_protocol() {
        let protocol = TraitEntity::new("Drawable", 1, 10)
            .set_protocol(true)
            .add_required_method("draw");

        assert!(protocol.is_protocol);
        assert_eq!(protocol.required_methods, vec!["draw"]);
    }

    #[test]
    fn test_trait_entity_abc() {
        let abc = TraitEntity::new("BaseClass", 1, 10)
            .set_abc(true)
            .add_required_method("abstract_method");

        assert!(abc.is_abc);
        assert_eq!(abc.required_methods, vec!["abstract_method"]);
    }
}
