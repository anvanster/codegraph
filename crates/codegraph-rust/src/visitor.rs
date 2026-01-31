//! AST visitor for extracting Rust entities using tree-sitter
//!
//! This module implements a tree-sitter based visitor that walks the Rust AST
//! and extracts functions, structs, enums, traits, and their relationships.

use codegraph_parser_api::{
    CallRelation, ClassEntity, Field, FunctionEntity, ImplementationRelation, ImportRelation,
    InheritanceRelation, Parameter, ParserConfig, TraitEntity,
};
use tree_sitter::Node;

/// Visitor that extracts entities and relationships from Rust AST
pub struct RustVisitor<'a> {
    pub source: &'a [u8],
    pub config: ParserConfig,
    pub functions: Vec<FunctionEntity>,
    pub classes: Vec<ClassEntity>,
    pub traits: Vec<TraitEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,
    pub implementations: Vec<ImplementationRelation>,
    pub inheritance: Vec<InheritanceRelation>,
    current_class: Option<String>,
}

impl<'a> RustVisitor<'a> {
    pub fn new(source: &'a [u8], config: ParserConfig) -> Self {
        Self {
            source,
            config,
            functions: Vec::new(),
            classes: Vec::new(),
            traits: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            implementations: Vec::new(),
            inheritance: Vec::new(),
            current_class: None,
        }
    }

    /// Get text from a node
    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    /// Main visitor entry point
    pub fn visit_node(&mut self, node: Node) {
        match node.kind() {
            "function_item" => {
                // Only visit top-level functions (not inside impl/trait blocks)
                if self.current_class.is_none() {
                    self.visit_function(node);
                }
            }
            "struct_item" => self.visit_struct(node),
            "enum_item" => self.visit_enum(node),
            "trait_item" => {
                self.visit_trait(node);
                // Don't recurse into trait body - methods already extracted
                return;
            }
            "impl_item" => {
                self.visit_impl(node);
                // Don't recurse into impl body - methods already extracted
                return;
            }
            "use_declaration" => self.visit_use(node),
            _ => {}
        }

        // Recursively visit children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    /// Extract visibility from a visibility_modifier node
    fn extract_visibility(&self, node: Node) -> String {
        // Look for visibility_modifier child
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                let text = self.node_text(child);
                if text.starts_with("pub(crate)") {
                    return "internal".to_string();
                } else if text.starts_with("pub(super)") {
                    return "protected".to_string();
                } else if text.starts_with("pub") {
                    return "public".to_string();
                }
            }
        }
        "private".to_string()
    }

    /// Check if a function has the #[test] attribute
    fn has_test_attribute(&self, node: Node) -> bool {
        // First, check for attributes as previous siblings (e.g., #[test] before fn)
        let mut current = node;
        while let Some(prev) = current.prev_sibling() {
            if prev.kind() == "attribute_item" {
                let attr_text = self.node_text(prev);
                if attr_text.contains("test") {
                    return true;
                }
            } else {
                // Stop when we hit a non-attribute node
                break;
            }
            current = prev;
        }

        // Also check children (for inner attributes)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "attribute_item" || child.kind() == "attribute" {
                let attr_text = self.node_text(child);
                if attr_text.contains("test") {
                    return true;
                }
            }
        }
        false
    }

    /// Check if function has async keyword
    fn is_async(&self, node: Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "async" || self.node_text(child) == "async" {
                return true;
            }
        }
        false
    }

    /// Extract doc comments (/// or //!) from preceding nodes
    fn extract_doc_comment(&self, node: Node) -> Option<String> {
        let mut docs = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "attribute_item" {
                let text = self.node_text(child);
                // Check for #[doc = "..."] style attributes
                if text.contains("doc") {
                    if let Some(start) = text.find('"') {
                        if let Some(end) = text.rfind('"') {
                            if start < end {
                                docs.push(text[start + 1..end].to_string());
                            }
                        }
                    }
                }
            } else if child.kind() == "line_comment" {
                let text = self.node_text(child);
                if text.starts_with("///") {
                    docs.push(text[3..].trim().to_string());
                } else if text.starts_with("//!") {
                    docs.push(text[3..].trim().to_string());
                }
            }
        }

        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n"))
        }
    }

    /// Extract parameters from a function's parameter list
    fn extract_parameters(&self, node: Node) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "self_parameter" => {
                        params.push(Parameter {
                            name: "self".to_string(),
                            type_annotation: Some("Self".to_string()),
                            default_value: None,
                            is_variadic: false,
                        });
                    }
                    "parameter" => {
                        let name = child
                            .child_by_field_name("pattern")
                            .map(|n| self.node_text(n))
                            .unwrap_or_else(|| "unknown".to_string());

                        let type_annotation = child
                            .child_by_field_name("type")
                            .map(|n| self.node_text(n));

                        params.push(Parameter {
                            name,
                            type_annotation,
                            default_value: None,
                            is_variadic: false,
                        });
                    }
                    _ => {}
                }
            }
        }

        params
    }

    /// Extract return type from function signature
    fn extract_return_type(&self, node: Node) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|n| self.node_text(n).trim_start_matches("->").trim().to_string())
    }

    /// Extract the first line as signature
    fn extract_signature(&self, node: Node) -> String {
        self.node_text(node)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// Extract type parameters from generics
    fn extract_type_parameters(&self, node: Node) -> Vec<String> {
        let mut params = Vec::new();

        if let Some(type_params) = node.child_by_field_name("type_parameters") {
            let mut cursor = type_params.walk();
            for child in type_params.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    params.push(self.node_text(child));
                } else if child.kind() == "constrained_type_parameter" {
                    // Get just the type name from T: Trait
                    if let Some(name) = child.child_by_field_name("left") {
                        params.push(self.node_text(name));
                    }
                }
            }
        }

        params
    }

    /// Visit a function declaration
    fn visit_function(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "anonymous".to_string());

        let visibility = self.extract_visibility(node);

        // Skip private functions if configured
        if self.config.skip_private && visibility == "private" {
            return;
        }

        let is_test = self.has_test_attribute(node);

        // Skip test functions if configured
        if self.config.skip_tests && is_test {
            return;
        }

        let func = FunctionEntity {
            name: name.clone(),
            signature: self.extract_signature(node),
            visibility,
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_async: self.is_async(node),
            is_test,
            is_static: false,
            is_abstract: false,
            parameters: self.extract_parameters(node),
            return_type: self.extract_return_type(node),
            doc_comment: self.extract_doc_comment(node),
            attributes: Vec::new(),
            parent_class: self.current_class.clone(),
            complexity: None,
        };

        self.functions.push(func);
    }

    /// Visit a struct declaration
    fn visit_struct(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "Struct".to_string());

        let visibility = self.extract_visibility(node);

        // Skip private structs if configured
        if self.config.skip_private && visibility == "private" {
            return;
        }

        // Extract fields from field_declaration_list
        let mut fields = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let field_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(n))
                        .unwrap_or_else(|| "unnamed".to_string());

                    let field_type = child
                        .child_by_field_name("type")
                        .map(|n| self.node_text(n));

                    let field_vis = self.extract_visibility(child);

                    fields.push(Field {
                        name: field_name,
                        type_annotation: field_type,
                        visibility: field_vis,
                        is_static: false,
                        is_constant: false,
                        default_value: None,
                    });
                }
            }
        }

        let class = ClassEntity {
            name: name.clone(),
            visibility,
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields,
            doc_comment: self.extract_doc_comment(node),
            attributes: Vec::new(),
            type_parameters: self.extract_type_parameters(node),
        };

        self.classes.push(class);
    }

    /// Visit an enum declaration
    fn visit_enum(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "Enum".to_string());

        let visibility = self.extract_visibility(node);

        // Skip private enums if configured
        if self.config.skip_private && visibility == "private" {
            return;
        }

        // Treat enums as classes with an "enum" attribute
        let class = ClassEntity {
            name: name.clone(),
            visibility,
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment: self.extract_doc_comment(node),
            attributes: vec!["enum".to_string()],
            type_parameters: self.extract_type_parameters(node),
        };

        self.classes.push(class);
    }

    /// Visit a trait declaration
    fn visit_trait(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "Trait".to_string());

        let visibility = self.extract_visibility(node);

        // Skip private traits if configured
        if self.config.skip_private && visibility == "private" {
            return;
        }

        // Extract required methods from the trait body
        let mut required_methods = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_signature_item" || child.kind() == "function_item" {
                    let method_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(n))
                        .unwrap_or_else(|| "method".to_string());

                    let func = FunctionEntity {
                        name: method_name,
                        signature: self.extract_signature(child),
                        visibility: "public".to_string(),
                        line_start: child.start_position().row + 1,
                        line_end: child.end_position().row + 1,
                        is_async: self.is_async(child),
                        is_test: false,
                        is_static: false,
                        is_abstract: true,
                        parameters: self.extract_parameters(child),
                        return_type: self.extract_return_type(child),
                        doc_comment: self.extract_doc_comment(child),
                        attributes: Vec::new(),
                        parent_class: Some(name.clone()),
                        complexity: None,
                    };

                    required_methods.push(func);
                }
            }
        }

        // Extract parent traits (supertraits)
        let mut parent_traits = Vec::new();
        if let Some(bounds) = node.child_by_field_name("bounds") {
            let mut cursor = bounds.walk();
            for child in bounds.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    parent_traits.push(self.node_text(child));
                }
            }
        }

        let trait_entity = TraitEntity {
            name: name.clone(),
            visibility,
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            required_methods,
            parent_traits,
            doc_comment: self.extract_doc_comment(node),
            attributes: Vec::new(),
        };

        self.traits.push(trait_entity);
    }

    /// Visit an impl block
    fn visit_impl(&mut self, node: Node) {
        // Extract the implementing type
        let implementor = node
            .child_by_field_name("type")
            .map(|n| {
                // Handle generic types like Type<T> - extract just the base name
                let text = self.node_text(n);
                text.split('<').next().unwrap_or(&text).trim().to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        // Check if this is a trait implementation
        if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = self.node_text(trait_node);
            let trait_name = trait_name.split('<').next().unwrap_or(&trait_name).trim().to_string();

            let impl_rel = ImplementationRelation {
                implementor: implementor.clone(),
                trait_name,
            };

            self.implementations.push(impl_rel);
        }

        // Set current class context for methods
        let previous_class = self.current_class.clone();
        self.current_class = Some(implementor.clone());

        // Extract methods from impl block body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_item" {
                    let method_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(n))
                        .unwrap_or_else(|| "method".to_string());

                    let visibility = self.extract_visibility(child);
                    let parameters = self.extract_parameters(child);

                    // Check if it's a static method (no self parameter)
                    let is_static = !parameters.iter().any(|p| p.name == "self");

                    let func = FunctionEntity {
                        name: method_name,
                        signature: self.extract_signature(child),
                        visibility,
                        line_start: child.start_position().row + 1,
                        line_end: child.end_position().row + 1,
                        is_async: self.is_async(child),
                        is_test: false,
                        is_static,
                        is_abstract: false,
                        parameters,
                        return_type: self.extract_return_type(child),
                        doc_comment: self.extract_doc_comment(child),
                        attributes: Vec::new(),
                        parent_class: Some(implementor.clone()),
                        complexity: None,
                    };

                    self.functions.push(func);
                }
            }
        }

        // Restore previous class context
        self.current_class = previous_class;
    }

    /// Visit a use declaration
    fn visit_use(&mut self, node: Node) {
        // Extract the use tree
        if let Some(use_tree) = node.child_by_field_name("argument") {
            let import_path = self.node_text(use_tree);

            let import = ImportRelation {
                importer: "current_module".to_string(),
                imported: import_path,
                symbols: Vec::new(),
                is_wildcard: false,
                alias: None,
            };

            self.imports.push(import);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_and_visit(source: &str) -> RustVisitor {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = RustVisitor::new(source.as_bytes(), ParserConfig::default());
        visitor.visit_node(tree.root_node());
        visitor
    }

    #[test]
    fn test_visitor_function() {
        let source = r#"
fn hello() {
    println!("Hello");
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "hello");
        assert_eq!(visitor.functions[0].line_start, 2);
        assert_eq!(visitor.functions[0].line_end, 4);
    }

    #[test]
    fn test_visitor_struct() {
        let source = r#"
pub struct MyStruct {
    pub field1: String,
    field2: i32,
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "MyStruct");
        assert_eq!(visitor.classes[0].visibility, "public");
        assert_eq!(visitor.classes[0].fields.len(), 2);
        assert_eq!(visitor.classes[0].line_start, 2);
    }

    #[test]
    fn test_visitor_trait() {
        let source = r#"
pub trait MyTrait {
    fn method(&self);
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.traits.len(), 1);
        assert_eq!(visitor.traits[0].name, "MyTrait");
        assert_eq!(visitor.traits[0].required_methods.len(), 1);
    }

    #[test]
    fn test_visitor_enum() {
        let source = r#"
pub enum Status {
    Active,
    Inactive,
    Pending,
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "Status");
        assert!(visitor.classes[0].attributes.contains(&"enum".to_string()));
    }

    #[test]
    fn test_visitor_impl_block() {
        let source = r#"
struct MyStruct;

impl MyStruct {
    fn new() -> Self {
        MyStruct
    }

    fn method(&self) {}
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.classes.len(), 1);
        // Should extract 2 methods from the impl block
        let impl_methods: Vec<_> = visitor.functions.iter()
            .filter(|f| f.parent_class == Some("MyStruct".to_string()))
            .collect();
        assert_eq!(impl_methods.len(), 2);
    }

    #[test]
    fn test_visitor_async_function() {
        let source = r#"
async fn fetch() -> String {
    "data".to_string()
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0].is_async);
    }

    #[test]
    fn test_visitor_use_statements() {
        let source = r#"
use std::collections::HashMap;
use std::io::{self, Read};
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.imports.len(), 2);
    }

    #[test]
    fn test_visitor_generic_struct() {
        let source = r#"
pub struct Wrapper<T> {
    value: T,
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "Wrapper");
        assert!(!visitor.classes[0].type_parameters.is_empty());
    }

    #[test]
    fn test_visitor_trait_impl() {
        let source = r#"
pub trait Display {
    fn display(&self);
}

pub struct Item;

impl Display for Item {
    fn display(&self) {}
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.traits.len(), 1);
        assert_eq!(visitor.classes.len(), 1);
        assert!(!visitor.implementations.is_empty());
        assert_eq!(visitor.implementations[0].implementor, "Item");
        assert_eq!(visitor.implementations[0].trait_name, "Display");
    }

    #[test]
    fn test_visitor_function_with_attributes() {
        let source = r#"
#[test]
#[ignore]
fn test_something() {}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0].is_test);
    }

    #[test]
    fn test_visitor_visibility_modifiers() {
        let source = r#"
pub fn public_fn() {}
fn private_fn() {}
pub(crate) fn crate_fn() {}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.functions.len(), 3);

        let public_count = visitor.functions.iter()
            .filter(|f| f.visibility == "public")
            .count();
        assert!(public_count >= 1);

        let internal_count = visitor.functions.iter()
            .filter(|f| f.visibility == "internal")
            .count();
        assert!(internal_count >= 1);
    }

    #[test]
    fn test_visitor_multiple_items() {
        let source = r#"
use std::fmt;

pub trait Trait1 {
    fn method1(&self);
}

pub struct Struct1 {
    field: i32,
}

pub enum Enum1 {
    Variant1,
    Variant2,
}

pub fn function1() {}

impl Struct1 {
    fn new() -> Self {
        Struct1 { field: 0 }
    }
}
"#;
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.traits.len(), 1);
        assert!(visitor.classes.len() >= 2); // Struct1 and Enum1
        assert!(!visitor.functions.is_empty());
        assert!(!visitor.imports.is_empty());
    }

    #[test]
    fn test_accurate_line_numbers() {
        let source = "fn first() {}\n\nfn second() {}";
        let visitor = parse_and_visit(source);
        assert_eq!(visitor.functions.len(), 2);
        assert_eq!(visitor.functions[0].name, "first");
        assert_eq!(visitor.functions[0].line_start, 1);
        assert_eq!(visitor.functions[1].name, "second");
        assert_eq!(visitor.functions[1].line_start, 3);
    }
}
