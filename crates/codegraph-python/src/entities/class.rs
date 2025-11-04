use super::function::FunctionEntity;
use serde::{Deserialize, Serialize};

/// Represents a class field/attribute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Field name
    pub name: String,

    /// Type annotation (if present)
    pub type_annotation: Option<String>,

    /// Visibility: "public", "private", or "protected"
    pub visibility: String,

    /// Is this a class variable (static) vs instance variable?
    pub is_static: bool,
}

impl Field {
    /// Create a new field
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let visibility = if name.starts_with("__") && name.ends_with("__") {
            "public".to_string() // Dunder names are public
        } else if name.starts_with("__") {
            "private".to_string()
        } else if name.starts_with('_') {
            "protected".to_string()
        } else {
            "public".to_string()
        };

        Self {
            name,
            type_annotation: None,
            visibility,
            is_static: false,
        }
    }

    /// Set the type annotation
    pub fn set_type_annotation(mut self, type_ann: Option<String>) -> Self {
        self.type_annotation = type_ann;
        self
    }

    /// Set whether this is a static field
    pub fn set_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }
}

/// Represents a Python class
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassEntity {
    /// Class name
    pub name: String,

    /// Visibility: "public" or "private"
    pub visibility: String,

    /// Starting line number (1-indexed)
    pub line_start: usize,

    /// Ending line number (1-indexed)
    pub line_end: usize,

    /// Is this an abstract class (has ABCMeta or ABC base)?
    pub is_abstract: bool,

    /// Inherited classes (base classes)
    pub base_classes: Vec<String>,

    /// Protocols/traits implemented
    pub implemented_traits: Vec<String>,

    /// Methods in this class (full FunctionEntity objects)
    pub methods: Vec<FunctionEntity>,

    /// Class attributes/fields
    pub fields: Vec<Field>,

    /// Documentation string (docstring)
    pub doc_comment: Option<String>,

    /// Decorators applied to the class (e.g., ["@dataclass"])
    pub attributes: Vec<String>,
}

impl ClassEntity {
    /// Create a new class entity with required fields
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
            is_abstract: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment: None,
            attributes: Vec::new(),
        }
    }

    /// Check if this is a dataclass
    pub fn is_dataclass(&self) -> bool {
        self.attributes.iter().any(|a| a == "@dataclass")
    }

    /// Check if this is abstract
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
            || self
                .base_classes
                .iter()
                .any(|b| b == "ABC" || b == "ABCMeta")
            || self.attributes.iter().any(|a| a.contains("abstractmethod"))
    }

    /// Check if a method exists
    pub fn has_method(&self, method_name: &str) -> bool {
        self.methods.iter().any(|m| m.name == method_name)
    }

    /// Set abstract flag
    pub fn set_abstract(mut self, is_abstract: bool) -> Self {
        self.is_abstract = is_abstract;
        self
    }

    /// Add a base class
    pub fn add_base_class(mut self, base: impl Into<String>) -> Self {
        self.base_classes.push(base.into());
        self
    }

    /// Add an implemented trait/protocol
    pub fn add_trait(mut self, trait_name: impl Into<String>) -> Self {
        self.implemented_traits.push(trait_name.into());
        self
    }

    /// Add a method
    pub fn add_method(mut self, method: FunctionEntity) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a field
    pub fn add_field(mut self, field: Field) -> Self {
        self.fields.push(field);
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
    fn test_field_visibility() {
        let public_field = Field::new("name");
        assert_eq!(public_field.visibility, "public");

        let protected_field = Field::new("_name");
        assert_eq!(protected_field.visibility, "protected");

        let private_field = Field::new("__name");
        assert_eq!(private_field.visibility, "private");

        let dunder_field = Field::new("__init__");
        assert_eq!(dunder_field.visibility, "public"); // Dunder methods are public
    }

    #[test]
    fn test_class_entity_visibility() {
        let public_class = ClassEntity::new("MyClass", 1, 10);
        assert_eq!(public_class.visibility, "public");

        let private_class = ClassEntity::new("_MyClass", 1, 10);
        assert_eq!(private_class.visibility, "private");
    }

    #[test]
    fn test_class_entity_dataclass() {
        let class = ClassEntity::new("User", 1, 10).add_attribute("@dataclass");

        assert!(class.is_dataclass());
    }

    #[test]
    fn test_class_entity_abstract() {
        let class1 = ClassEntity::new("Base", 1, 10).add_base_class("ABC");
        assert!(class1.is_abstract());

        let class2 = ClassEntity::new("Base", 1, 10).set_abstract(true);
        assert!(class2.is_abstract());
    }

    #[test]
    fn test_class_entity_methods() {
        let method1 = FunctionEntity::new("method1", 2, 4);
        let method2 = FunctionEntity::new("method2", 5, 7);

        let class = ClassEntity::new("MyClass", 1, 10)
            .add_method(method1)
            .add_method(method2);

        assert!(class.has_method("method1"));
        assert!(class.has_method("method2"));
        assert!(!class.has_method("method3"));
        assert_eq!(class.methods.len(), 2);
    }
}
