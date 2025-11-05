//! AST visitor for extracting Go entities

use codegraph_parser_api::{
    ClassEntity, FunctionEntity, ImportRelation, ParserConfig, TraitEntity,
};
use tree_sitter::Node;

pub struct GoVisitor<'a> {
    pub source: &'a [u8],
    pub config: ParserConfig,
    pub functions: Vec<FunctionEntity>,
    pub structs: Vec<ClassEntity>,
    pub interfaces: Vec<TraitEntity>,
    pub imports: Vec<ImportRelation>,
}

impl<'a> GoVisitor<'a> {
    pub fn new(source: &'a [u8], config: ParserConfig) -> Self {
        Self {
            source,
            config,
            functions: Vec::new(),
            structs: Vec::new(),
            interfaces: Vec::new(),
            imports: Vec::new(),
        }
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    pub fn visit_node(&mut self, node: Node) {
        match node.kind() {
            "function_declaration" => self.visit_function(node),
            "method_declaration" => self.visit_method(node),
            "type_declaration" => self.visit_type_declaration(node),
            "import_declaration" => self.visit_import(node),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    fn visit_function(&mut self, node: Node) {
        let name = node.child_by_field_name("name").map(|n| self.node_text(n)).unwrap_or_else(|| "anonymous".to_string());

        let func = FunctionEntity {
            name,
            signature: self.node_text(node).lines().next().unwrap_or("").to_string(),
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
        let name = node.child_by_field_name("name").map(|n| self.node_text(n)).unwrap_or_else(|| "method".to_string());

        let func = FunctionEntity {
            name,
            signature: self.node_text(node).lines().next().unwrap_or("").to_string(),
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

    fn visit_type_declaration(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                let name = child.child_by_field_name("name").map(|n| self.node_text(n)).unwrap_or_else(|| "Type".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_basics() {
        let visitor = GoVisitor::new(b"package main", ParserConfig::default());
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.structs.len(), 0);
        assert_eq!(visitor.interfaces.len(), 0);
    }

    #[test]
    fn test_visitor_function_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc greet(name string) string { return \"Hello\" }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "greet");
    }

    #[test]
    fn test_visitor_struct_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\ntype Person struct { Name string }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.structs.len(), 1);
        assert_eq!(visitor.structs[0].name, "Person");
    }

    #[test]
    fn test_visitor_interface_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\ntype Reader interface { Read() error }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.interfaces.len(), 1);
        assert_eq!(visitor.interfaces[0].name, "Reader");
    }

    #[test]
    fn test_visitor_method_extraction() {
        use tree_sitter::Parser;

        let source = b"package main\nfunc (p Person) String() string { return \"\" }";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
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
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.imports.len(), 1);
    }

    #[test]
    fn test_visitor_multiple_declarations() {
        use tree_sitter::Parser;

        let source = b"package main\ntype User struct {}\ntype Admin struct {}";
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = GoVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.structs.len(), 2);
        assert_eq!(visitor.structs[0].name, "User");
        assert_eq!(visitor.structs[1].name, "Admin");
    }
}
