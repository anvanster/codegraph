//! AST visitor for extracting TypeScript/JavaScript entities

use codegraph_parser_api::{
    CallRelation, ClassEntity, FunctionEntity, ImplementationRelation, ImportRelation,
    InheritanceRelation, Parameter, ParserConfig, TraitEntity,
};
use tree_sitter::Node;

/// Visitor that extracts entities and relationships from TypeScript/JavaScript AST
pub struct TypeScriptVisitor<'a> {
    pub source: &'a [u8],
    pub config: ParserConfig,
    pub functions: Vec<FunctionEntity>,
    pub classes: Vec<ClassEntity>,
    pub interfaces: Vec<TraitEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,
    pub implementations: Vec<ImplementationRelation>,
    pub inheritance: Vec<InheritanceRelation>,
    current_class: Option<String>,
}

impl<'a> TypeScriptVisitor<'a> {
    pub fn new(source: &'a [u8], config: ParserConfig) -> Self {
        Self {
            source,
            config,
            functions: Vec::new(),
            classes: Vec::new(),
            interfaces: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            implementations: Vec::new(),
            inheritance: Vec::new(),
            current_class: None,
        }
    }

    /// Get text for a node
    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source)
            .unwrap_or("")
            .to_string()
    }

    /// Visit a tree-sitter node
    pub fn visit_node(&mut self, node: Node) {
        let handled = match node.kind() {
            // Only match declaration nodes to avoid duplicates
            "function_declaration" => {
                self.visit_function(node);
                true
            }
            "arrow_function" => {
                self.visit_arrow_function(node);
                true
            }
            "method_definition" => {
                self.visit_method(node);
                true
            }
            "class_declaration" => {
                self.visit_class(node);
                true
            }
            "interface_declaration" => {
                self.visit_interface(node);
                true
            }
            "import_statement" => {
                self.visit_import(node);
                true
            }
            _ => false,
        };

        // Only recursively visit children for unhandled node types
        if !handled {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.visit_node(child);
            }
        }
    }

    fn visit_function(&mut self, node: Node) {
        // Extract function name
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "anonymous".to_string());

        // Extract parameters
        let parameters = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.extract_parameters(params_node)
        } else {
            Vec::new()
        };

        // Check if async
        let is_async = self.node_text(node).starts_with("async");

        let func = FunctionEntity {
            name: name.clone(),
            signature: self.node_text(node).lines().next().unwrap_or("").to_string(),
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_async,
            is_test: false,
            is_static: false,
            is_abstract: false,
            parameters,
            return_type: None,
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: self.current_class.clone(),
        };

        self.functions.push(func);
    }

    fn visit_arrow_function(&mut self, node: Node) {
        let func = FunctionEntity {
            name: "arrow_function".to_string(),
            signature: "() => {}".to_string(),
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_async: false,
            is_test: false,
            is_static: false,
            is_abstract: false,
            parameters: Vec::new(),
            return_type: None,
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: None,
        };

        self.functions.push(func);
    }

    fn visit_method(&mut self, node: Node) {
        // Extract method name from property_identifier or identifier
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "method".to_string());

        // Extract parameters if available
        let parameters = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.extract_parameters(params_node)
        } else {
            Vec::new()
        };

        // Check if method is static or async
        let node_text = self.node_text(node);
        let is_static = node_text.contains("static ");
        let is_async = node_text.contains("async ");

        // Determine visibility (private/public based on # prefix in name or explicit keyword)
        let visibility = if name.starts_with('#') {
            "private".to_string()
        } else {
            "public".to_string()
        };

        let func = FunctionEntity {
            name,
            signature: node_text.lines().next().unwrap_or("").to_string(),
            visibility,
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_async,
            is_test: false,
            is_static,
            is_abstract: false,
            parameters,
            return_type: None,
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: self.current_class.clone(),
        };

        self.functions.push(func);
    }

    fn visit_class(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "AnonymousClass".to_string());

        // Set current class context
        let previous_class = self.current_class.clone();
        self.current_class = Some(name.clone());

        let class = ClassEntity {
            name: name.clone(),
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment: None,
            attributes: Vec::new(),
            type_parameters: Vec::new(),
        };

        self.classes.push(class);

        // Visit children (methods, properties)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "class_body" {
                let mut body_cursor = child.walk();
                for member in child.children(&mut body_cursor) {
                    self.visit_node(member);
                }
            }
        }

        // Restore previous class context
        self.current_class = previous_class;
    }

    fn visit_interface(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "AnonymousInterface".to_string());

        let interface = TraitEntity {
            name,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            required_methods: Vec::new(),
            parent_traits: Vec::new(),
            doc_comment: None,
            attributes: Vec::new(),
        };

        self.interfaces.push(interface);
    }

    fn visit_import(&mut self, node: Node) {
        // Extract the source (from 'react', './utils', etc.)
        let source = node
            .child_by_field_name("source")
            .map(|n| {
                let text = self.node_text(n);
                // Remove quotes from source
                text.trim_matches(|c| c == '"' || c == '\'').to_string()
            })
            .unwrap_or_default();

        let mut symbols = Vec::new();
        let mut alias = None;
        let mut is_wildcard = false;

        // Parse import_clause to extract specifiers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_clause" {
                // Parse the import_clause
                let mut clause_cursor = child.walk();
                for clause_child in child.children(&mut clause_cursor) {
                    match clause_child.kind() {
                        // Default import: import React from 'react'
                        "identifier" => {
                            symbols.push(self.node_text(clause_child));
                        }
                        // Named imports: { useState, useEffect }
                        "named_imports" => {
                            symbols.extend(self.extract_named_imports(clause_child));
                        }
                        // Namespace import: * as Utils
                        "namespace_import" => {
                            is_wildcard = true;
                            // Extract the identifier after 'as'
                            let mut ns_cursor = clause_child.walk();
                            for ns_child in clause_child.children(&mut ns_cursor) {
                                if ns_child.kind() == "identifier" {
                                    alias = Some(self.node_text(ns_child));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let import = ImportRelation {
            importer: "current_module".to_string(),
            imported: source,
            symbols,
            is_wildcard,
            alias,
        };

        self.imports.push(import);
    }

    fn extract_named_imports(&self, node: Node) -> Vec<String> {
        let mut imports = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "import_specifier" {
                // Handle both "name" and "name as alias" forms
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    if spec_child.kind() == "identifier" {
                        imports.push(self.node_text(spec_child));
                        break; // Only take the first identifier (the imported name, not the alias)
                    }
                }
            }
        }

        imports
    }

    fn extract_parameters(&self, params_node: Node) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                let name = child
                    .child_by_field_name("pattern")
                    .map(|n| self.node_text(n))
                    .unwrap_or_else(|| "param".to_string());

                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|n| self.node_text(n));

                parameters.push(Parameter {
                    name,
                    type_annotation,
                    default_value: None,
                    is_variadic: false,
                });
            }
        }

        parameters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_basics() {
        let visitor = TypeScriptVisitor::new(b"test", ParserConfig::default());
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.classes.len(), 0);
    }

    #[test]
    fn test_visitor_function_parameters() {
        use tree_sitter::Parser;

        let source = b"function greet(name: string, age: number): void {}";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "greet");
        assert_eq!(visitor.functions[0].parameters.len(), 2);
        assert_eq!(visitor.functions[0].parameters[0].name, "name");
        assert_eq!(visitor.functions[0].parameters[1].name, "age");
    }

    #[test]
    fn test_visitor_async_function_detection() {
        use tree_sitter::Parser;

        let source = b"async function loadData() { await fetch(); }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0].is_async);
    }

    #[test]
    fn test_visitor_class_context() {
        use tree_sitter::Parser;

        let source = b"class MyClass { myMethod() {} }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "MyClass");
        // Note: Method extraction not yet implemented
        // Visitor would need to match "method_definition" node type
    }

    #[test]
    fn test_visitor_interface_extraction() {
        use tree_sitter::Parser;

        let source = b"interface IPerson { name: string; age: number; }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.interfaces.len(), 1);
        assert_eq!(visitor.interfaces[0].name, "IPerson");
    }

    #[test]
    fn test_visitor_import_extraction() {
        use tree_sitter::Parser;

        let source = b"import { useState } from 'react';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
    }

    #[test]
    fn test_visitor_named_imports() {
        use tree_sitter::Parser;

        let source = b"import { useState, useEffect } from 'react';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "react");
        assert_eq!(visitor.imports[0].symbols.len(), 2);
        assert_eq!(visitor.imports[0].symbols[0], "useState");
        assert_eq!(visitor.imports[0].symbols[1], "useEffect");
        assert_eq!(visitor.imports[0].is_wildcard, false);
    }

    #[test]
    fn test_visitor_default_import() {
        use tree_sitter::Parser;

        let source = b"import React from 'react';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "react");
        assert_eq!(visitor.imports[0].symbols.len(), 1);
        assert_eq!(visitor.imports[0].symbols[0], "React");
    }

    #[test]
    fn test_visitor_namespace_import() {
        use tree_sitter::Parser;

        let source = b"import * as Utils from './utils';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "./utils");
        assert_eq!(visitor.imports[0].is_wildcard, true);
        assert_eq!(visitor.imports[0].alias, Some("Utils".to_string()));
    }

    #[test]
    fn test_visitor_side_effect_import() {
        use tree_sitter::Parser;

        let source = b"import './styles.css';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "./styles.css");
        assert_eq!(visitor.imports[0].symbols.len(), 0);
    }

    #[test]
    fn test_visitor_mixed_default_and_named_imports() {
        use tree_sitter::Parser;

        let source = b"import React, { useState, useEffect } from 'react';";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "react");
        assert_eq!(visitor.imports[0].symbols.len(), 3);
        assert_eq!(visitor.imports[0].symbols[0], "React");
        assert_eq!(visitor.imports[0].symbols[1], "useState");
        assert_eq!(visitor.imports[0].symbols[2], "useEffect");
    }

    #[test]
    fn test_visitor_arrow_function_extraction() {
        use tree_sitter::Parser;

        let source = b"const func = () => { return 42; };";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        // Arrow functions should be extracted
        assert!(visitor.functions.len() > 0);
    }

    #[test]
    fn test_visitor_method_extraction() {
        use tree_sitter::Parser;

        let source = b"class Calculator { add(a: number, b: number): number { return a + b; } }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "Calculator");
        // Should extract method as a function with parent_class
        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "add");
        assert_eq!(visitor.functions[0].parent_class, Some("Calculator".to_string()));
    }

    #[test]
    fn test_visitor_multiple_methods() {
        use tree_sitter::Parser;

        let source = b"class Math { add(a, b) { return a + b; } subtract(a, b) { return a - b; } multiply(a, b) { return a * b; } }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.classes.len(), 1);
        // Should extract all 3 methods
        assert_eq!(visitor.functions.len(), 3);
        assert_eq!(visitor.functions[0].name, "add");
        assert_eq!(visitor.functions[1].name, "subtract");
        assert_eq!(visitor.functions[2].name, "multiply");
        // All methods should have parent_class set
        assert!(visitor.functions.iter().all(|f| f.parent_class == Some("Math".to_string())));
    }

    #[test]
    fn test_visitor_constructor_extraction() {
        use tree_sitter::Parser;

        let source = b"class Person { constructor(name: string) { this.name = name; } }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.classes.len(), 1);
        // Should extract constructor
        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "constructor");
        assert_eq!(visitor.functions[0].parent_class, Some("Person".to_string()));
    }

    #[test]
    fn test_visitor_static_method() {
        use tree_sitter::Parser;

        let source = b"class Utils { static format(value: string): string { return value; } }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TypeScriptVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.classes.len(), 1);
        // Should extract static method
        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "format");
        assert_eq!(visitor.functions[0].is_static, true);
        assert_eq!(visitor.functions[0].parent_class, Some("Utils".to_string()));
    }
}
