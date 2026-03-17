//! AST visitor for extracting Verilog entities

use codegraph_parser_api::{
    CallRelation, ClassEntity, ComplexityBuilder, ComplexityMetrics, FunctionEntity, ImportRelation,
};
use tree_sitter::Node;

pub struct VerilogVisitor<'a> {
    pub source: &'a [u8],
    pub modules: Vec<ClassEntity>,
    pub functions: Vec<FunctionEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,
    current_module: Option<String>,
    current_function: Option<String>,
}

impl<'a> VerilogVisitor<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            modules: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            current_module: None,
            current_function: None,
        }
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    /// Find the first simple_identifier or escaped_identifier child
    fn find_identifier_in(&self, node: Node) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "simple_identifier" | "escaped_identifier" => {
                    return Some(self.node_text(child));
                }
                _ => {}
            }
        }
        None
    }

    /// Recursively search for the first simple_identifier descendant (BFS up to depth 4)
    fn find_identifier_recursive(&self, node: Node, depth: usize) -> Option<String> {
        if depth == 0 {
            return None;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "simple_identifier" | "escaped_identifier" => {
                    return Some(self.node_text(child));
                }
                _ => {}
            }
        }
        // Second pass: recurse
        let mut cursor2 = node.walk();
        for child in node.children(&mut cursor2) {
            if let Some(name) = self.find_identifier_recursive(child, depth - 1) {
                return Some(name);
            }
        }
        None
    }

    pub fn visit_node(&mut self, node: Node) {
        match node.kind() {
            "module_declaration" => {
                self.visit_module(node);
                return;
            }
            "function_declaration" => {
                self.visit_function(node);
                return;
            }
            "task_declaration" => {
                self.visit_task(node);
                return;
            }
            "include_compiler_directive" => {
                self.visit_include(node);
            }
            "package_import_declaration" => {
                self.visit_package_import(node);
            }
            "module_instantiation" => {
                self.visit_module_instantiation(node);
            }
            "checker_instantiation" => {
                // The grammar sometimes parses module instantiations as checker_instantiations
                // due to Verilog parsing ambiguity (both use the same named port syntax)
                self.visit_checker_instantiation(node);
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    fn visit_module(&mut self, node: Node) {
        // module_declaration -> module_header -> simple_identifier (module name)
        let name = {
            let mut cursor = node.walk();
            let header = node
                .children(&mut cursor)
                .find(|c| c.kind() == "module_header");
            header
                .and_then(|h| self.find_identifier_in(h))
                .or_else(|| self.find_identifier_recursive(node, 3))
                .unwrap_or_else(|| "unknown".to_string())
        };

        let prev_module = self.current_module.clone();
        self.current_module = Some(name.clone());

        let module_entity = ClassEntity {
            name,
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
        self.modules.push(module_entity);

        // Visit children for functions/tasks/instantiations inside this module
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }

        self.current_module = prev_module;
    }

    fn visit_function(&mut self, node: Node) {
        // function_declaration -> function_body_declaration -> function_identifier -> simple_identifier
        let name = {
            let mut cursor = node.walk();
            let body = node
                .children(&mut cursor)
                .find(|c| c.kind() == "function_body_declaration");
            body.and_then(|b| {
                let mut bc = b.walk();
                let func_id = b
                    .children(&mut bc)
                    .find(|c| c.kind() == "function_identifier");
                func_id.and_then(|fi| self.find_identifier_in(fi))
            })
            .or_else(|| self.find_identifier_recursive(node, 4))
            .unwrap_or_else(|| "unknown_function".to_string())
        };

        let prev_function = self.current_function.clone();
        self.current_function = Some(name.clone());

        let complexity = self.calculate_complexity(node);

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
            parent_class: self.current_module.clone(),
            complexity: Some(complexity),
        };

        self.functions.push(func);
        self.current_function = prev_function;
    }

    fn visit_task(&mut self, node: Node) {
        // task_declaration -> task_body_declaration -> task_identifier -> simple_identifier
        let name = {
            let mut cursor = node.walk();
            let body = node
                .children(&mut cursor)
                .find(|c| c.kind() == "task_body_declaration");
            body.and_then(|b| {
                let mut bc = b.walk();
                let task_id = b.children(&mut bc).find(|c| c.kind() == "task_identifier");
                task_id.and_then(|ti| self.find_identifier_in(ti))
            })
            .or_else(|| self.find_identifier_recursive(node, 4))
            .unwrap_or_else(|| "unknown_task".to_string())
        };

        let prev_function = self.current_function.clone();
        self.current_function = Some(name.clone());

        let complexity = self.calculate_complexity(node);

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
            parent_class: self.current_module.clone(),
            complexity: Some(complexity),
        };

        self.functions.push(func);
        self.current_function = prev_function;
    }

    fn visit_include(&mut self, node: Node) {
        // include_compiler_directive -> double_quoted_string
        let path = {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "double_quoted_string")
                .map(|n| {
                    let text = self.node_text(n);
                    text.trim_matches('"').to_string()
                });
            found.unwrap_or_default()
        };

        if !path.is_empty() {
            self.imports.push(ImportRelation {
                importer: self
                    .current_module
                    .clone()
                    .unwrap_or_else(|| "file".to_string()),
                imported: path,
                symbols: Vec::new(),
                is_wildcard: false,
                alias: None,
            });
        }
    }

    fn visit_package_import(&mut self, node: Node) {
        // package_import_declaration -> package_import_item -> package_identifier
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "package_import_item" {
                // package_import_item has package_identifier and optional simple_identifier
                let mut ic = child.walk();
                let mut pkg_name = String::new();
                let mut is_wildcard = false;

                for item_child in child.children(&mut ic) {
                    match item_child.kind() {
                        "package_identifier" => {
                            pkg_name = self
                                .find_identifier_in(item_child)
                                .unwrap_or_else(|| self.node_text(item_child));
                        }
                        "simple_identifier" => {
                            // specific symbol import
                        }
                        "*" => {
                            is_wildcard = true;
                        }
                        _ => {
                            let text = self.node_text(item_child);
                            if text == "*" {
                                is_wildcard = true;
                            }
                        }
                    }
                }

                if !pkg_name.is_empty() {
                    self.imports.push(ImportRelation {
                        importer: self
                            .current_module
                            .clone()
                            .unwrap_or_else(|| "file".to_string()),
                        imported: pkg_name,
                        symbols: Vec::new(),
                        is_wildcard,
                        alias: None,
                    });
                }
            }
        }
    }

    fn visit_module_instantiation(&mut self, node: Node) {
        // module_instantiation -> simple_identifier (module type being instantiated)
        let module_type = {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "simple_identifier" || c.kind() == "escaped_identifier")
                .map(|n| self.node_text(n));
            found.unwrap_or_default()
        };

        if !module_type.is_empty() {
            let caller = self
                .current_module
                .clone()
                .unwrap_or_else(|| "file".to_string());
            self.calls.push(CallRelation::new(
                caller,
                module_type,
                node.start_position().row + 1,
            ));
        }
    }

    fn visit_checker_instantiation(&mut self, node: Node) {
        // checker_instantiation -> checker_identifier -> simple_identifier
        // The grammar uses checker_instantiation for what are often module instantiations
        // due to Verilog parsing ambiguity with named port connections
        let module_type = {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| c.kind() == "checker_identifier")
                .and_then(|ci| self.find_identifier_in(ci));
            found
                .or_else(|| {
                    // Fallback: look for simple_identifier directly
                    let mut c2 = node.walk();
                    let f = node
                        .children(&mut c2)
                        .find(|c| {
                            c.kind() == "simple_identifier" || c.kind() == "escaped_identifier"
                        })
                        .map(|n| self.node_text(n));
                    f
                })
                .unwrap_or_default()
        };

        if !module_type.is_empty() {
            let caller = self
                .current_module
                .clone()
                .unwrap_or_else(|| "file".to_string());
            self.calls.push(CallRelation::new(
                caller,
                module_type,
                node.start_position().row + 1,
            ));
        }
    }

    fn calculate_complexity(&self, node: Node) -> ComplexityMetrics {
        let mut builder = ComplexityBuilder::new();
        self.visit_for_complexity(node, &mut builder);
        builder.build()
    }

    fn visit_for_complexity(&self, node: Node, builder: &mut ComplexityBuilder) {
        match node.kind() {
            "conditional_statement" | "case_statement" => {
                builder.add_branch();
                builder.enter_scope();
            }
            "case_item" => {
                builder.add_branch();
            }
            "loop_statement" | "for_step_assignment" => {
                builder.add_loop();
                builder.enter_scope();
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_for_complexity(child, builder);
        }

        match node.kind() {
            "conditional_statement" | "case_statement" | "loop_statement" => {
                builder.exit_scope();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_basics() {
        let visitor = VerilogVisitor::new(b"module top(); endmodule");
        assert_eq!(visitor.modules.len(), 0);
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.imports.len(), 0);
    }

    #[test]
    fn test_visitor_module_extraction() {
        use tree_sitter::Parser;
        let source = b"module counter (input clk); endmodule";
        let mut parser = Parser::new();
        parser.set_language(&crate::ts_verilog::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = VerilogVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert_eq!(visitor.modules.len(), 1);
        assert_eq!(visitor.modules[0].name, "counter");
    }

    #[test]
    fn test_visitor_function_extraction() {
        use tree_sitter::Parser;
        let source =
            b"module top(); function integer add; input a, b; add = a + b; endfunction endmodule";
        let mut parser = Parser::new();
        parser.set_language(&crate::ts_verilog::language()).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = VerilogVisitor::new(source);
        visitor.visit_node(tree.root_node());

        assert!(
            !visitor.functions.is_empty(),
            "Expected at least one function"
        );
    }
}
