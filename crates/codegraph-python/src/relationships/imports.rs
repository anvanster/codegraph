use serde::{Deserialize, Serialize};

/// Represents an import statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEntity {
    /// List of imported items (names, modules, etc.)
    pub imported_items: Vec<String>,

    /// Source module being imported from
    pub from_module: String,

    /// Line number of the import statement
    pub line: usize,

    /// Is this a wildcard import (from module import *)?
    pub is_wildcard: bool,
}

impl ImportEntity {
    /// Create a regular import
    pub fn new(from_module: impl Into<String>, imported_items: Vec<String>, line: usize) -> Self {
        Self {
            imported_items,
            from_module: from_module.into(),
            line,
            is_wildcard: false,
        }
    }

    /// Create a wildcard import
    pub fn wildcard(from_module: impl Into<String>, line: usize) -> Self {
        Self {
            imported_items: vec![],
            from_module: from_module.into(),
            line,
            is_wildcard: true,
        }
    }

    /// Check if this imports a specific name
    pub fn imports_name(&self, name: &str) -> bool {
        if self.is_wildcard {
            true // Wildcard imports everything
        } else {
            self.imported_items.iter().any(|item| item == name)
        }
    }

    /// Add an imported item
    pub fn add_item(mut self, item: impl Into<String>) -> Self {
        self.imported_items.push(item.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_entity_new() {
        let import = ImportEntity::new("os", vec!["path".to_string()], 1);
        assert_eq!(import.from_module, "os");
        assert_eq!(import.imported_items, vec!["path"]);
        assert_eq!(import.line, 1);
        assert!(!import.is_wildcard);
    }

    #[test]
    fn test_wildcard_import() {
        let import = ImportEntity::wildcard("os", 1);
        assert_eq!(import.from_module, "os");
        assert!(import.imported_items.is_empty());
        assert!(import.is_wildcard);
    }

    #[test]
    fn test_imports_name() {
        let import = ImportEntity::new("os", vec!["path".to_string(), "environ".to_string()], 1);
        assert!(import.imports_name("path"));
        assert!(import.imports_name("environ"));
        assert!(!import.imports_name("getcwd"));

        let wildcard = ImportEntity::wildcard("os", 1);
        assert!(wildcard.imports_name("anything"));
    }
}
