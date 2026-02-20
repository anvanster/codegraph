//! AST visitor for extracting Python entities using tree-sitter
//!
//! This module implements a tree-sitter based visitor that walks the Python AST
//! and extracts functions, classes, and their relationships.

use tree_sitter::Node;

/// Context for tracking the current scope during AST traversal
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct VisitorContext {
    /// Current class name (if we're inside a class)
    pub current_class: Option<String>,

    /// Current function name (if we're inside a function)
    pub current_function: Option<String>,

    /// Nesting depth (for tracking nested definitions)
    pub depth: usize,
}

#[allow(dead_code)]
impl VisitorContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Enter a class scope
    pub fn enter_class(&mut self, class_name: String) {
        self.current_class = Some(class_name);
        self.depth += 1;
    }

    /// Exit a class scope
    pub fn exit_class(&mut self) {
        self.current_class = None;
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Enter a function scope
    pub fn enter_function(&mut self, func_name: String) {
        self.current_function = Some(func_name);
        self.depth += 1;
    }

    /// Exit a function scope
    pub fn exit_function(&mut self) {
        self.current_function = None;
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Check if we're at top level (not inside any definition)
    pub fn is_top_level(&self) -> bool {
        self.depth == 0
    }
}

/// Extract the first docstring from a block node
pub fn extract_docstring(source: &[u8], node: Node) -> Option<String> {
    // Look for the first expression_statement that contains a string
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut expr_cursor = child.walk();
            for expr_child in child.children(&mut expr_cursor) {
                if expr_child.kind() == "string" {
                    let text = expr_child.utf8_text(source).unwrap_or("");
                    // Remove quotes
                    let text = text.trim();
                    if text.starts_with("\"\"\"") || text.starts_with("'''") {
                        let inner = &text[3..text.len().saturating_sub(3)];
                        return Some(inner.trim().to_string());
                    } else if text.starts_with('"') || text.starts_with('\'') {
                        let inner = &text[1..text.len().saturating_sub(1)];
                        return Some(inner.trim().to_string());
                    }
                }
            }
        } else if child.kind() != "comment" {
            // Stop looking after non-docstring statements
            break;
        }
    }
    None
}

/// Extract decorator names from a decorated definition
pub fn extract_decorators(source: &[u8], node: Node) -> Vec<String> {
    let mut decorators = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" {
            let text = child.utf8_text(source).unwrap_or("");
            // Handle @decorator and @decorator(args) patterns
            let name = text.trim_start_matches('@');
            // Get just the decorator name (before any parentheses)
            let name = name.split('(').next().unwrap_or(name);
            decorators.push(format!("@{}", name.trim()));
        }
    }

    decorators
}

/// Extract type annotation as string
#[allow(dead_code)]
pub fn type_to_string(source: &[u8], node: Node) -> String {
    node.utf8_text(source).unwrap_or("Any").to_string()
}

/// Extract parameter default value as string
#[allow(dead_code)]
pub fn default_to_string(source: &[u8], node: Node) -> String {
    node.utf8_text(source).unwrap_or("...").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_context() {
        let mut ctx = VisitorContext::new();
        assert!(ctx.is_top_level());

        ctx.enter_class("MyClass".to_string());
        assert!(!ctx.is_top_level());
        assert_eq!(ctx.current_class, Some("MyClass".to_string()));

        ctx.exit_class();
        assert!(ctx.is_top_level());
        assert_eq!(ctx.current_class, None);
    }

    #[test]
    fn test_context_depth() {
        let mut ctx = VisitorContext::new();
        assert_eq!(ctx.depth, 0);

        ctx.enter_class("Outer".to_string());
        assert_eq!(ctx.depth, 1);

        ctx.enter_class("Inner".to_string());
        assert_eq!(ctx.depth, 2);

        ctx.exit_class();
        assert_eq!(ctx.depth, 1);

        ctx.exit_class();
        assert_eq!(ctx.depth, 0);
    }
}
