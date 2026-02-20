//! AST visitor for extracting Go entities

use codegraph_parser_api::{
    CallRelation, ClassEntity, FunctionEntity, ImportRelation, TraitEntity,
};
use tree_sitter::Node;

pub struct GoVisitor<'a> {
    pub source: &'a [u8],
    pub functions: Vec<FunctionEntity>,
    pub structs: Vec<ClassEntity>,
    pub interfaces: Vec<TraitEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,
    current_function: Option<String>,
}

impl<'a> GoVisitor<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            functions: Vec::new(),
            structs: Vec::new(),
            interfaces: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            current_function: None,
        }
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    pub fn visit_node(&mut self, node: Node) {
        match node.kind() {
            "function_declaration" => {
                self.visit_function(node);
                // Don't recurse — visit_function handles body for call extraction
                return;
            }
            "method_declaration" => {
                self.visit_method(node);
                // Don't recurse — visit_method handles body for call extraction
                return;
            }
            "type_declaration" => self.visit_type_declaration(node),
            "import_declaration" => self.visit_import(node),
            "call_expression" => self.visit_call_expression(node),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    fn visit_function(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "anonymous".to_string());

        let previous_function = self.current_function.clone();
        self.current_function = Some(name.clone());

        let func = FunctionEntity {
            name,
            signature: self
                .node_text(node)
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
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
            complexity: None,
        };

        self.functions.push(func);

        if let Some(body) = node.child_by_field_name("body") {
            self.visit_body_for_calls(body);
        }

        self.current_function = previous_function;
    }

    fn visit_method(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(n))
            .unwrap_or_else(|| "method".to_string());

        let previous_function = self.current_function.clone();
        self.current_function = Some(name.clone());

        let func = FunctionEntity {
            name,
            signature: self
                .node_text(node)
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
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
            complexity: None,
        };

        self.functions.push(func);

        if let Some(body) = node.child_by_field_name("body") {
            self.visit_body_for_calls(body);
        }

        self.current_function = previous_function;
    }

    /// Visit a call expression and record the caller→callee relationship
    fn visit_call_expression(&mut self, node: Node) {
        let caller = match &self.current_function {
            Some(name) => name.clone(),
            None => return,
        };

        let callee = self.extract_callee_name(node);
        if callee.is_empty() {
            return;
        }

        self.calls.push(CallRelation::new(
            caller,
            callee,
            node.start_position().row + 1,
        ));
    }

    /// Extract the callee function name from a call expression node
    fn extract_callee_name(&self, node: Node) -> String {
        if let Some(func_node) = node.child_by_field_name("function") {
            match func_node.kind() {
                "identifier" => self.node_text(func_node),
                "selector_expression" => {
                    // e.g., fmt.Println, obj.Method
                    if let Some(field) = func_node.child_by_field_name("field") {
                        self.node_text(field)
                    } else {
                        self.node_text(func_node)
                    }
                }
                _ => self.node_text(func_node),
            }
        } else {
            String::new()
        }
    }

    /// Recursively visit a node's children looking for call expressions
    fn visit_body_for_calls(&mut self, node: Node) {
        if node.kind() == "call_expression" {
            self.visit_call_expression(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_body_for_calls(child);
        }
    }

    fn visit_type_declaration(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| self.node_text(n))
                    .unwrap_or_else(|| "Type".to_string());
                let type_node = child.child_by_field_name("type");

                if let Some(type_node) = type_node {
                    match type_node.kind() {
                        "struct_type" => {
                            let struct_entity = ClassEntity {
                                name,
                                visibility: "public".to_string(),
                                line_start: child.start_position().row + 1,
                                line_end: child.end_position().row + 1,
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
                            self.structs.push(struct_entity);
                        }
                        "interface_type" => {
                            let interface_entity = TraitEntity {
                                name,
                                visibility: "public".to_string(),
                                line_start: child.start_position().row + 1,
                                line_end: child.end_position().row + 1,
                                required_methods: Vec::new(),
                                parent_traits: Vec::new(),
                                doc_comment: None,
                                attributes: Vec::new(),
                            };
                            self.interfaces.push(interface_entity);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn visit_import(&mut self, node: Node) {
        // Check if this is an import block or single import
        let mut cursor = node.walk();
        let mut found_specs = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_spec_list" => {
                    // Import block: import ( ... )
                    found_specs = true;
                    let mut spec_cursor = child.walk();
                    for spec in child.children(&mut spec_cursor) {
                        if spec.kind() == "import_spec" {
                            self.extract_import_spec(spec);
                        }
                    }
                }
                "import_spec" => {
                    // Single import: import "fmt" or import f "fmt"
                    found_specs = true;
                    self.extract_import_spec(child);
                }
                _ => {}
            }
        }

        // Fallback for unexpected format
        if !found_specs {
            let import_text = self.node_text(node);
            let import = ImportRelation {
                importer: "current_package".to_string(),
                imported: import_text,
                symbols: Vec::new(),
                is_wildcard: false,
                alias: None,
            };
            self.imports.push(import);
        }
    }

    fn extract_import_spec(&mut self, node: Node) {
        let mut alias = None;
        let mut is_wildcard = false;
        let mut path = String::new();

        // Extract path and optional name (alias)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let text = self.node_text(child);

            match kind {
                "interpreted_string_literal" => {
                    // This is the import path
                    // Remove quotes
                    path = text.trim_matches('"').to_string();
                }
                "package_identifier" | "identifier" | "dot" | "." => {
                    // This is the alias/name or special marker
                    if text == "." {
                        is_wildcard = true;
                    } else if text != "_" {
                        alias = Some(text);
                    }
                    // If it's "_", we ignore it (blank identifier)
                }
                _ => {
                    // Check text content for special cases
                    if text == "." {
                        is_wildcard = true;
                    } else if text != "_"
                        && !text.trim().is_empty()
                        && kind != "("
                        && kind != ")"
                        && kind != "\""
                    {
                        // Might be an unrecognized alias format
                        if !path.is_empty() {
                            // Only set alias if we haven't found the path yet would mean this comes before
                            // Actually in Go, alias comes before path
                        }
                    }
                }
            }
        }

        let import = ImportRelation {
            importer: "current_package".to_string(),
            imported: path,
            symbols: Vec::new(), // Go doesn't have named imports like TypeScript
            is_wildcard,
            alias,
        };
        self.imports.push(import);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_basics() {
        let visitor = GoVisitor::new(b"package main");
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.structs.len(), 0);
        assert_eq!(visitor.interfaces.len(), 0);
    }

    #[test]
    fn test_visitor_function_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc greet(name string) string { return \"Hello\" }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "greet");
    }

    #[test]
    fn test_visitor_struct_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\ntype Person struct { Name string }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.structs.len(), 1);
        assert_eq!(visitor.structs[0].name, "Person");
    }

    #[test]
    fn test_visitor_interface_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\ntype Reader interface { Read() error }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.interfaces.len(), 1);
        assert_eq!(visitor.interfaces[0].name, "Reader");
    }

    #[test]
    fn test_visitor_method_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc (p Person) String() string { return \"\" }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        // Methods are extracted as functions
        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "String");
    }

    #[test]
    fn test_visitor_import_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nimport \"fmt\"";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
    }

    #[test]
    fn test_visitor_multiple_declarations() {
        use tree_sitter::Parser;

        let source = b"package main\ntype User struct {}\ntype Admin struct {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.structs.len(), 2);
        assert_eq!(visitor.structs[0].name, "User");
        assert_eq!(visitor.structs[1].name, "Admin");
    }

    // TDD: New tests for individual import extraction
    #[test]
    fn test_visitor_import_block_multiple_imports() {
        use tree_sitter::Parser;

        let source = b"package main\nimport (\n\t\"fmt\"\n\t\"os\"\n\t\"io\"\n)";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        // Should extract 3 individual imports, not 1 block
        assert_eq!(
            visitor.imports.len(),
            3,
            "Should extract 3 individual imports"
        );
        assert_eq!(visitor.imports[0].imported, "fmt");
        assert_eq!(visitor.imports[1].imported, "os");
        assert_eq!(visitor.imports[2].imported, "io");
    }

    #[test]
    fn test_visitor_import_with_alias() {
        use tree_sitter::Parser;

        let source = b"package main\nimport f \"fmt\"";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "fmt");
        assert_eq!(visitor.imports[0].alias, Some("f".to_string()));
        assert!(!visitor.imports[0].is_wildcard);
    }

    #[test]
    fn test_visitor_import_with_dot_wildcard() {
        use tree_sitter::Parser;

        let source = b"package main\nimport . \"fmt\"";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "fmt");
        assert!(visitor.imports[0].is_wildcard);
        assert_eq!(visitor.imports[0].alias, None);
    }

    #[test]
    fn test_visitor_import_with_blank_identifier() {
        use tree_sitter::Parser;

        let source = b"package main\nimport _ \"database/sql\"";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
        assert_eq!(visitor.imports[0].imported, "database/sql");
        assert_eq!(visitor.imports[0].alias, None); // _ is ignored
        assert!(!visitor.imports[0].is_wildcard);
    }

    #[test]
    fn test_visitor_import_block_with_aliases() {
        use tree_sitter::Parser;

        let source = b"package main\nimport (\n\tf \"fmt\"\n\t. \"os\"\n\t_ \"encoding/json\"\n)";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 3);

        // Import with alias
        assert_eq!(visitor.imports[0].imported, "fmt");
        assert_eq!(visitor.imports[0].alias, Some("f".to_string()));
        assert!(!visitor.imports[0].is_wildcard);

        // Import with dot (wildcard)
        assert_eq!(visitor.imports[1].imported, "os");
        assert!(visitor.imports[1].is_wildcard);
        assert_eq!(visitor.imports[1].alias, None);

        // Import with blank identifier
        assert_eq!(visitor.imports[2].imported, "encoding/json");
        assert_eq!(visitor.imports[2].alias, None);
        assert!(!visitor.imports[2].is_wildcard);
    }

    #[test]
    fn test_visitor_call_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc caller() {\n\tcallee()\n\tfmt.Println(\"hello\")\n}\nfunc callee() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.calls.len(), 2, "Should extract 2 calls");
        assert_eq!(visitor.calls[0].caller, "caller");
        assert_eq!(visitor.calls[0].callee, "callee");
        assert_eq!(visitor.calls[1].caller, "caller");
        assert_eq!(visitor.calls[1].callee, "Println");
    }

    #[test]
    fn test_visitor_method_call_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\ntype Foo struct{}\nfunc (f Foo) Method() {\n\thelper()\n}\nfunc helper() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.calls.len(), 1);
        assert_eq!(visitor.calls[0].caller, "Method");
        assert_eq!(visitor.calls[0].callee, "helper");
    }

    #[test]
    fn test_visitor_no_calls_outside_function() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc standalone() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.calls.len(), 0);
    }
}
