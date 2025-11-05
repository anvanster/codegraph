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
        match node.kind() {
            // Only match declaration nodes to avoid duplicates
            "function_declaration" => self.visit_function(node),
            "arrow_function" => self.visit_arrow_function(node),
            "class_declaration" => self.visit_class(node),
            "interface_declaration" => self.visit_interface(node),
            "import_statement" => self.visit_import(node),
            _ => {}
        }

        // Recursively visit children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
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
        let import_text = self.node_text(node);

        let import = ImportRelation {
            importer: "current_module".to_string(),
            imported: import_text,
            symbols: Vec::new(),
            is_wildcard: false,
            alias: None,
        };

        self.imports.push(import);
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
}
