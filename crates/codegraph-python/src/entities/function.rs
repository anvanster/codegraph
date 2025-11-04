use serde::{Deserialize, Serialize};

/// Represents a function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Type annotation (if present)
    pub type_annotation: Option<String>,

    /// Default value as string (if present)
    pub default_value: Option<String>,
}

impl Parameter {
    /// Create a new parameter with just a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: None,
            default_value: None,
        }
    }

    /// Create a parameter with type annotation
    pub fn with_type(name: impl Into<String>, type_annotation: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: Some(type_annotation.into()),
            default_value: None,
        }
    }

    /// Add a default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }
}

/// Represents a function or method in Python source code
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionEntity {
    /// Function name
    pub name: String,

    /// Full signature (including parameters and return type)
    pub signature: String,

    /// Visibility: "public" (no underscore) or "private" (starts with _)
    pub visibility: String,

    /// Starting line number (1-indexed)
    pub line_start: usize,

    /// Ending line number (1-indexed)
    pub line_end: usize,

    /// Is this an async def function?
    pub is_async: bool,

    /// Is this a test function? (name starts with "test_" or "Test")
    pub is_test: bool,

    /// Function parameters
    pub parameters: Vec<Parameter>,

    /// Return type annotation (if present)
    pub return_type: Option<String>,

    /// Documentation string (docstring)
    pub doc_comment: Option<String>,

    /// Parent class name (for methods)
    pub parent: Option<String>,

    /// Decorators applied to the function (e.g., ["@staticmethod", "@cache"])
    pub attributes: Vec<String>,
}

impl FunctionEntity {
    /// Create a new function entity with required fields
    pub fn new(name: impl Into<String>, line_start: usize, line_end: usize) -> Self {
        let name = name.into();
        let visibility = if name.starts_with('_') {
            "private".to_string()
        } else {
            "public".to_string()
        };

        let is_test = name.starts_with("test_") || name.starts_with("Test");

        Self {
            name: name.clone(),
            signature: format!("def {name}"),
            visibility,
            line_start,
            line_end,
            is_async: false,
            is_test,
            parameters: Vec::new(),
            return_type: None,
            doc_comment: None,
            parent: None,
            attributes: Vec::new(),
        }
    }

    /// Check if this is a method (has parent class)
    pub fn is_method(&self) -> bool {
        self.parent.is_some()
    }

    /// Check if this is a static method
    pub fn is_static(&self) -> bool {
        self.attributes.iter().any(|a| a == "@staticmethod")
    }

    /// Check if this is a class method
    pub fn is_classmethod(&self) -> bool {
        self.attributes.iter().any(|a| a == "@classmethod")
    }

    /// Check if this is a property
    pub fn is_property(&self) -> bool {
        self.attributes.iter().any(|a| a == "@property")
    }

    /// Set the async flag
    pub fn set_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    /// Set the return type
    pub fn set_return_type(mut self, return_type: Option<String>) -> Self {
        self.return_type = return_type;
        self
    }

    /// Set the docstring
    pub fn set_doc_comment(mut self, doc: Option<String>) -> Self {
        self.doc_comment = doc;
        self
    }

    /// Set the parent class (for methods)
    pub fn set_parent(mut self, parent: Option<String>) -> Self {
        self.parent = parent;
        self
    }

    /// Add a decorator
    pub fn add_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attributes.push(attr.into());
        self
    }

    /// Add a parameter
    pub fn add_parameter(mut self, param: Parameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Set the signature
    pub fn set_signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = sig.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_new() {
        let param = Parameter::new("arg");
        assert_eq!(param.name, "arg");
        assert!(param.type_annotation.is_none());
        assert!(param.default_value.is_none());
    }

    #[test]
    fn test_parameter_with_type() {
        let param = Parameter::with_type("arg", "str");
        assert_eq!(param.name, "arg");
        assert_eq!(param.type_annotation, Some("str".to_string()));
    }

    #[test]
    fn test_parameter_with_default() {
        let param = Parameter::with_type("arg", "int").with_default("42");
        assert_eq!(param.default_value, Some("42".to_string()));
    }

    #[test]
    fn test_function_entity_visibility() {
        let public_func = FunctionEntity::new("my_func", 1, 10);
        assert_eq!(public_func.visibility, "public");

        let private_func = FunctionEntity::new("_my_func", 1, 10);
        assert_eq!(private_func.visibility, "private");
    }

    #[test]
    fn test_function_entity_test_detection() {
        let test_func = FunctionEntity::new("test_something", 1, 10);
        assert!(test_func.is_test);

        let regular_func = FunctionEntity::new("do_something", 1, 10);
        assert!(!regular_func.is_test);
    }

    #[test]
    fn test_function_entity_method_detection() {
        let mut func = FunctionEntity::new("method", 1, 10);
        assert!(!func.is_method());

        func = func.set_parent(Some("MyClass".to_string()));
        assert!(func.is_method());
    }

    #[test]
    fn test_function_entity_decorators() {
        let func = FunctionEntity::new("method", 1, 10).add_attribute("@staticmethod");

        assert!(func.is_static());
        assert!(!func.is_classmethod());
        assert!(!func.is_property());
    }
}
