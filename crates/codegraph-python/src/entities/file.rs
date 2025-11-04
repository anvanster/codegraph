use serde::{Deserialize, Serialize};

/// Represents a Python module/file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEntity {
    /// Module name (derived from file path)
    pub name: String,

    /// Visibility (always "public" for modules)
    pub visibility: String,

    /// Starting line number (always 1 for modules)
    pub line_start: usize,

    /// Ending line number (last line of file)
    pub line_end: usize,

    /// Module docstring
    pub doc_comment: Option<String>,
}

impl ModuleEntity {
    /// Create a new module entity
    pub fn new(name: impl Into<String>, line_end: usize) -> Self {
        Self {
            name: name.into(),
            visibility: "public".to_string(),
            line_start: 1,
            line_end,
            doc_comment: None,
        }
    }

    /// Set the module docstring
    pub fn set_doc_comment(mut self, doc: Option<String>) -> Self {
        self.doc_comment = doc;
        self
    }

    /// Get the total number of lines in the module
    pub fn line_count(&self) -> usize {
        self.line_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_entity_new() {
        let module = ModuleEntity::new("my_module", 100);
        assert_eq!(module.name, "my_module");
        assert_eq!(module.visibility, "public");
        assert_eq!(module.line_start, 1);
        assert_eq!(module.line_end, 100);
        assert_eq!(module.line_count(), 100);
    }

    #[test]
    fn test_module_entity_with_docstring() {
        let module = ModuleEntity::new("my_module", 50)
            .set_doc_comment(Some("This is a module".to_string()));

        assert_eq!(module.doc_comment, Some("This is a module".to_string()));
    }
}
