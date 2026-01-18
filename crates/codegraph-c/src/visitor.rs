//! AST visitor for extracting C entities
//!
//! This visitor traverses the tree-sitter AST and extracts:
//! - Functions (with complexity metrics)
//! - Structs, unions, enums
//! - Include directives
//! - Function calls (for call graph building)

use codegraph_parser_api::{
    ClassEntity, ComplexityBuilder, ComplexityMetrics, Field, FunctionEntity, ImportRelation,
    Parameter, ParserConfig,
};
use tree_sitter::Node;

/// Represents a function call found in the source
#[derive(Debug, Clone)]
pub struct FunctionCall {
    /// Name of the called function
    pub callee: String,
    /// Line number where the call occurs
    pub line: usize,
    /// Name of the calling function (if inside a function)
    pub caller: Option<String>,
}

pub struct CVisitor<'a> {
    pub source: &'a [u8],
    #[allow(dead_code)]
    pub config: ParserConfig,
    pub functions: Vec<FunctionEntity>,
    pub structs: Vec<ClassEntity>,
    pub imports: Vec<ImportRelation>,
    /// Function calls extracted from the source
    pub calls: Vec<FunctionCall>,
    /// Whether to extract function calls
    extract_calls: bool,
    /// Current function being visited (for tracking caller)
    current_function: Option<String>,
}

impl<'a> CVisitor<'a> {
    pub fn new(source: &'a [u8], config: ParserConfig) -> Self {
        Self {
            source,
            config,
            functions: Vec::new(),
            structs: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            extract_calls: false,
            current_function: None,
        }
    }

    /// Enable or disable call extraction
    pub fn set_extract_calls(&mut self, extract: bool) {
        self.extract_calls = extract;
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    pub fn visit_node(&mut self, node: Node) {
        // Skip ERROR nodes - tree-sitter marks unparseable sections as ERROR
        // We continue visiting children to extract what we can
        if node.is_error() {
            // Still visit children of ERROR nodes to extract valid nested content
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.visit_node(child);
            }
            return;
        }

        match node.kind() {
            "function_definition" => self.visit_function(node),
            "struct_specifier" => self.visit_struct(node),
            "union_specifier" => self.visit_union(node),
            "enum_specifier" => self.visit_enum(node),
            "preproc_include" => self.visit_include(node),
            "call_expression" if self.extract_calls => self.visit_call(node),
            _ => {}
        }

        // Don't recurse into function bodies for top-level visits
        // (we handle them specially in visit_function)
        if node.kind() != "function_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.visit_node(child);
            }
        } else {
            // For function definitions, only recurse into the body for calls
            // but not for other top-level entities
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "compound_statement" {
                    self.visit_node(child);
                } else if self.extract_calls {
                    // Visit body for call extraction
                    self.visit_node_for_calls(child);
                }
            }
        }
    }

    /// Visit nodes specifically for call extraction (doesn't extract entities)
    fn visit_node_for_calls(&mut self, node: Node) {
        if node.is_error() {
            return;
        }

        if node.kind() == "call_expression" {
            self.visit_call(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node_for_calls(child);
        }
    }

    /// Extract a function call
    fn visit_call(&mut self, node: Node) {
        // call_expression has a "function" field that contains the callee
        if let Some(function_node) = node.child_by_field_name("function") {
            let callee = match function_node.kind() {
                "identifier" => self.node_text(function_node),
                "field_expression" => {
                    // For method calls like obj->method or obj.method
                    if let Some(field) = function_node.child_by_field_name("field") {
                        self.node_text(field)
                    } else {
                        self.node_text(function_node)
                    }
                }
                "parenthesized_expression" => {
                    // Function pointer call: (*func_ptr)(args)
                    "(*indirect)".to_string()
                }
                _ => self.node_text(function_node),
            };

            if !callee.is_empty() {
                self.calls.push(FunctionCall {
                    callee,
                    line: node.start_position().row + 1,
                    caller: self.current_function.clone(),
                });
            }
        }
    }

    fn visit_function(&mut self, node: Node) {
        let mut name = String::new();
        let mut return_type = String::new();
        let mut parameters = Vec::new();
        let mut is_static = false;

        // Check for storage class specifier (static)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "storage_class_specifier" {
                let text = self.node_text(child);
                if text == "static" {
                    is_static = true;
                }
            }
        }

        // Extract return type
        if let Some(type_node) = node.child_by_field_name("type") {
            return_type = self.extract_type_string(type_node);
        }

        // Extract function name and parameters from declarator
        if let Some(declarator) = node.child_by_field_name("declarator") {
            self.extract_function_declarator(declarator, &mut name, &mut parameters);
        }

        // Set current function for call tracking
        let prev_function = self.current_function.take();
        if !name.is_empty() {
            self.current_function = Some(name.clone());
        }

        // Calculate complexity from function body
        let complexity = node
            .child_by_field_name("body")
            .map(|body| self.calculate_complexity(body));

        let visibility = if is_static { "private" } else { "public" };

        let signature = self
            .node_text(node)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        let func = FunctionEntity {
            name,
            signature,
            visibility: visibility.to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_async: false,
            is_test: false,
            is_static,
            is_abstract: false,
            parameters,
            return_type: if return_type.is_empty() {
                None
            } else {
                Some(return_type)
            },
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: None,
            complexity,
        };

        self.functions.push(func);

        // Restore previous function context
        self.current_function = prev_function;
    }

    fn extract_function_declarator(
        &self,
        node: Node,
        name: &mut String,
        parameters: &mut Vec<Parameter>,
    ) {
        match node.kind() {
            "function_declarator" => {
                // Get function name from nested declarator
                if let Some(decl) = node.child_by_field_name("declarator") {
                    *name = self.extract_identifier(decl);
                }
                // Get parameters
                if let Some(params) = node.child_by_field_name("parameters") {
                    self.extract_parameters(params, parameters);
                }
            }
            "pointer_declarator" => {
                // Handle pointer return type: int *func()
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "function_declarator" {
                        self.extract_function_declarator(child, name, parameters);
                        return;
                    }
                }
            }
            "identifier" => {
                *name = self.node_text(node);
            }
            _ => {}
        }
    }

    fn extract_identifier(&self, node: Node) -> String {
        match node.kind() {
            "identifier" => self.node_text(node),
            "pointer_declarator" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let id = self.extract_identifier(child);
                    if !id.is_empty() {
                        return id;
                    }
                }
                String::new()
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let id = self.extract_identifier(child);
                    if !id.is_empty() {
                        return id;
                    }
                }
                String::new()
            }
        }
    }

    fn extract_parameters(&self, node: Node, parameters: &mut Vec<Parameter>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                if let Some(param) = self.extract_parameter(child) {
                    parameters.push(param);
                }
            } else if child.kind() == "variadic_parameter" {
                parameters.push(Parameter {
                    name: "...".to_string(),
                    type_annotation: Some("...".to_string()),
                    default_value: None,
                    is_variadic: true,
                });
            }
        }
    }

    fn extract_parameter(&self, node: Node) -> Option<Parameter> {
        let mut type_str = String::new();
        let mut name = String::new();

        // Extract type
        if let Some(type_node) = node.child_by_field_name("type") {
            type_str = self.extract_type_string(type_node);
        }

        // Extract name from declarator
        if let Some(declarator) = node.child_by_field_name("declarator") {
            let (decl_name, pointer_prefix) = self.extract_declarator_info(declarator);
            name = decl_name;
            if !pointer_prefix.is_empty() {
                type_str = format!("{type_str}{pointer_prefix}");
            }
        }

        // Handle case where there's no declarator (just type)
        if name.is_empty() {
            name = "param".to_string();
        }

        Some(Parameter {
            name,
            type_annotation: if type_str.is_empty() {
                None
            } else {
                Some(type_str)
            },
            default_value: None,
            is_variadic: false,
        })
    }

    fn extract_declarator_info(&self, node: Node) -> (String, String) {
        match node.kind() {
            "identifier" | "field_identifier" => (self.node_text(node), String::new()),
            "pointer_declarator" => {
                let mut pointer_count = 0;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "*" {
                        pointer_count += 1;
                    } else {
                        let (name, extra_ptrs) = self.extract_declarator_info(child);
                        if !name.is_empty() {
                            return (name, "*".repeat(pointer_count) + &extra_ptrs);
                        }
                    }
                }
                (String::new(), "*".repeat(pointer_count))
            }
            "array_declarator" => {
                if let Some(decl) = node.child_by_field_name("declarator") {
                    let (name, _) = self.extract_declarator_info(decl);
                    return (name, "[]".to_string());
                }
                (String::new(), "[]".to_string())
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let (name, ptrs) = self.extract_declarator_info(child);
                    if !name.is_empty() {
                        return (name, ptrs);
                    }
                }
                (String::new(), String::new())
            }
        }
    }

    fn extract_type_string(&self, node: Node) -> String {
        let text = self.node_text(node);
        text.trim().to_string()
    }

    fn visit_struct(&mut self, node: Node) {
        // Only extract structs that have a body (not forward declarations)
        let has_body = node.child_by_field_name("body").is_some();
        if !has_body {
            return;
        }

        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| format!("__anon_struct_{}", node.start_position().row + 1));

        let fields = self.extract_struct_fields(node);

        let struct_entity = ClassEntity {
            name,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields,
            doc_comment: None,
            attributes: vec!["struct".to_string()],
            type_parameters: Vec::new(),
        };

        self.structs.push(struct_entity);
    }

    fn visit_union(&mut self, node: Node) {
        // Only extract unions that have a body
        let has_body = node.child_by_field_name("body").is_some();
        if !has_body {
            return;
        }

        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| format!("__anon_union_{}", node.start_position().row + 1));

        let fields = self.extract_struct_fields(node);

        let union_entity = ClassEntity {
            name,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields,
            doc_comment: None,
            attributes: vec!["union".to_string()],
            type_parameters: Vec::new(),
        };

        self.structs.push(union_entity);
    }

    fn visit_enum(&mut self, node: Node) {
        // Only extract enums that have a body
        let has_body = node.child_by_field_name("body").is_some();
        if !has_body {
            return;
        }

        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| format!("__anon_enum_{}", node.start_position().row + 1));

        // Extract enum constants as fields
        let mut fields = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "enumerator" {
                    let enumerator_name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(n))
                        .unwrap_or_default();

                    let default_value = child
                        .child_by_field_name("value")
                        .map(|n| self.node_text(n));

                    if !enumerator_name.is_empty() {
                        fields.push(Field {
                            name: enumerator_name,
                            type_annotation: Some("int".to_string()),
                            visibility: "public".to_string(),
                            is_static: true,
                            is_constant: true,
                            default_value,
                        });
                    }
                }
            }
        }

        let enum_entity = ClassEntity {
            name,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields,
            doc_comment: None,
            attributes: vec!["enum".to_string()],
            type_parameters: Vec::new(),
        };

        self.structs.push(enum_entity);
    }

    fn extract_struct_fields(&self, node: Node) -> Vec<Field> {
        let mut fields = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let field_type = child
                        .child_by_field_name("type")
                        .map(|n| self.extract_type_string(n))
                        .unwrap_or_default();

                    // Try to get declarator - field can have multiple declarators
                    // e.g., "int x, y;" has two declarators
                    let mut field_cursor = child.walk();
                    for field_child in child.children(&mut field_cursor) {
                        // Handle different declarator types
                        match field_child.kind() {
                            "field_identifier" => {
                                // Simple field: int x;
                                let field_name = self.node_text(field_child);
                                if !field_name.is_empty() {
                                    fields.push(Field {
                                        name: field_name,
                                        type_annotation: Some(field_type.clone()),
                                        visibility: "public".to_string(),
                                        is_static: false,
                                        is_constant: false,
                                        default_value: None,
                                    });
                                }
                            }
                            "pointer_declarator" | "array_declarator" => {
                                // Pointer/array field: char *name; int arr[10];
                                let (field_name, pointer_suffix) =
                                    self.extract_declarator_info(field_child);
                                let full_type = if pointer_suffix.is_empty() {
                                    field_type.clone()
                                } else {
                                    format!("{field_type}{pointer_suffix}")
                                };

                                if !field_name.is_empty() {
                                    fields.push(Field {
                                        name: field_name,
                                        type_annotation: Some(full_type),
                                        visibility: "public".to_string(),
                                        is_static: false,
                                        is_constant: false,
                                        default_value: None,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        fields
    }

    fn visit_include(&mut self, node: Node) {
        let mut path = String::new();
        let mut is_system = false;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "system_lib_string" => {
                    // Remove < and >
                    path = self.node_text(child);
                    path = path
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string();
                    is_system = true;
                }
                "string_literal" => {
                    // Remove quotes
                    path = self.node_text(child);
                    path = path.trim_matches('"').to_string();
                    is_system = false;
                }
                _ => {}
            }
        }

        if !path.is_empty() {
            let import = ImportRelation {
                importer: "current_file".to_string(),
                imported: path,
                symbols: Vec::new(),
                is_wildcard: true, // C includes always import everything
                alias: if is_system {
                    Some("system".to_string())
                } else {
                    None
                },
            };
            self.imports.push(import);
        }
    }

    fn calculate_complexity(&self, body: Node) -> ComplexityMetrics {
        let mut builder = ComplexityBuilder::new();
        self.visit_for_complexity(body, &mut builder);
        builder.build()
    }

    fn visit_for_complexity(&self, node: Node, builder: &mut ComplexityBuilder) {
        match node.kind() {
            "if_statement" => {
                builder.add_branch();
                builder.enter_scope();
            }
            "else_clause" => {
                builder.add_branch();
            }
            "for_statement" => {
                builder.add_loop();
                builder.enter_scope();
            }
            "while_statement" => {
                builder.add_loop();
                builder.enter_scope();
            }
            "do_statement" => {
                builder.add_loop();
                builder.enter_scope();
            }
            "switch_statement" => {
                builder.enter_scope();
            }
            "case_statement" => {
                builder.add_branch();
            }
            "default_statement" => {
                builder.add_branch();
            }
            "conditional_expression" => {
                // Ternary operator ?:
                builder.add_branch();
            }
            "goto_statement" => {
                // C-specific: goto adds complexity
                builder.add_branch();
            }
            "binary_expression" => {
                // Check for && and ||
                if let Some(op) = node.child_by_field_name("operator") {
                    let op_text = self.node_text(op);
                    if op_text == "&&" || op_text == "||" {
                        builder.add_logical_operator();
                    }
                }
            }
            "return_statement" => {
                // Early return detection (not at the end of function)
                // For now, count all returns except the last one would require more context
                // Simplified: just count as potential early return
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_for_complexity(child, builder);
        }

        // Exit scope for control structures
        match node.kind() {
            "if_statement" | "for_statement" | "while_statement" | "do_statement"
            | "switch_statement" => {
                builder.exit_scope();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_and_visit(source: &[u8]) -> CVisitor<'_> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_c::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = CVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());
        visitor
    }

    #[test]
    fn test_visitor_basics() {
        let visitor = CVisitor::new(b"int main() {}", ParserConfig::default());
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.structs.len(), 0);
        assert_eq!(visitor.imports.len(), 0);
    }

    #[test]
    fn test_visitor_function_extraction() {
        let source = b"int greet(char *name) { return 0; }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "greet");
        assert_eq!(visitor.functions[0].return_type, Some("int".to_string()));
    }

    #[test]
    fn test_visitor_static_function() {
        let source = b"static void helper() {}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].visibility, "private");
        assert!(visitor.functions[0].is_static);
    }

    #[test]
    fn test_visitor_struct_extraction() {
        let source = b"struct Person { char *name; int age; };";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.structs.len(), 1);
        assert_eq!(visitor.structs[0].name, "Person");
        assert_eq!(visitor.structs[0].fields.len(), 2);
        assert!(visitor.structs[0]
            .attributes
            .contains(&"struct".to_string()));
    }

    #[test]
    fn test_visitor_union_extraction() {
        let source = b"union Data { int i; float f; };";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.structs.len(), 1);
        assert_eq!(visitor.structs[0].name, "Data");
        assert!(visitor.structs[0].attributes.contains(&"union".to_string()));
    }

    #[test]
    fn test_visitor_enum_extraction() {
        let source = b"enum Color { RED, GREEN, BLUE };";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.structs.len(), 1);
        assert_eq!(visitor.structs[0].name, "Color");
        assert!(visitor.structs[0].attributes.contains(&"enum".to_string()));
        assert_eq!(visitor.structs[0].fields.len(), 3);
    }

    #[test]
    fn test_visitor_system_include() {
        let source = b"#include <stdio.h>";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "stdio.h");
        assert_eq!(visitor.imports[0].alias, Some("system".to_string()));
    }

    #[test]
    fn test_visitor_local_include() {
        let source = b"#include \"myheader.h\"";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "myheader.h");
        assert_eq!(visitor.imports[0].alias, None);
    }

    #[test]
    fn test_visitor_multiple_includes() {
        let source = b"#include <stdio.h>\n#include <stdlib.h>\n#include \"myheader.h\"";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.imports.len(), 3);
    }

    #[test]
    fn test_visitor_function_with_params() {
        let source = b"int add(int a, int b) { return a + b; }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].parameters.len(), 2);
        assert_eq!(visitor.functions[0].parameters[0].name, "a");
        assert_eq!(visitor.functions[0].parameters[1].name, "b");
    }

    #[test]
    fn test_visitor_pointer_params() {
        let source = b"void process(int *arr, char **argv) {}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].parameters.len(), 2);
        // Pointer types should be captured
        let param1 = &visitor.functions[0].parameters[0];
        assert!(param1
            .type_annotation
            .as_ref()
            .map(|t| t.contains("*"))
            .unwrap_or(false));
    }

    #[test]
    fn test_visitor_variadic_function() {
        let source = b"int printf(const char *fmt, ...) { return 0; }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let params = &visitor.functions[0].parameters;
        assert!(params.iter().any(|p| p.is_variadic));
    }

    #[test]
    fn test_visitor_complexity_if() {
        let source = b"void test() { if (1) {} }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let complexity = visitor.functions[0].complexity.as_ref().unwrap();
        assert!(complexity.branches >= 1);
    }

    #[test]
    fn test_visitor_complexity_loop() {
        let source = b"void test() { for (int i = 0; i < 10; i++) {} while(1) {} }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let complexity = visitor.functions[0].complexity.as_ref().unwrap();
        assert!(complexity.loops >= 2);
    }

    #[test]
    fn test_visitor_complexity_switch() {
        let source =
            b"void test(int x) { switch(x) { case 1: break; case 2: break; default: break; } }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let complexity = visitor.functions[0].complexity.as_ref().unwrap();
        assert!(complexity.branches >= 3);
    }

    #[test]
    fn test_visitor_complexity_logical_operators() {
        let source = b"void test(int a, int b) { if (a && b || a) {} }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let complexity = visitor.functions[0].complexity.as_ref().unwrap();
        assert!(complexity.logical_operators >= 2);
    }

    #[test]
    fn test_visitor_complexity_goto() {
        let source = b"void test() { label: goto label; }";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        let complexity = visitor.functions[0].complexity.as_ref().unwrap();
        // goto should add to branches
        assert!(complexity.branches >= 1);
    }

    #[test]
    fn test_visitor_forward_declaration_ignored() {
        let source = b"struct Forward;";
        let visitor = parse_and_visit(source);

        // Forward declarations should not be extracted
        assert_eq!(visitor.structs.len(), 0);
    }

    #[test]
    fn test_visitor_anonymous_struct() {
        let source = b"struct { int x; int y; } point;";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.structs.len(), 1);
        // Anonymous struct should get a generated name
        assert!(visitor.structs[0].name.starts_with("__anon_struct_"));
    }

    #[test]
    fn test_visitor_enum_with_values() {
        let source = b"enum Size { SMALL = 1, MEDIUM = 5, LARGE = 10 };";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.structs.len(), 1);
        let enum_entity = &visitor.structs[0];
        assert_eq!(enum_entity.fields.len(), 3);

        // Check that values are captured
        let small = enum_entity.fields.iter().find(|f| f.name == "SMALL");
        assert!(small.is_some());
        assert_eq!(small.unwrap().default_value, Some("1".to_string()));
    }
}
