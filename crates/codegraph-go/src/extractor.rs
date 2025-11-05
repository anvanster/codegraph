//! AST extraction for Go source code

use codegraph_parser_api::{CodeIR, ModuleEntity, ParserConfig, ParserError};
use std::path::Path;
use tree_sitter::{Language, Parser};

use crate::visitor::GoVisitor;

extern "C" {
    fn tree_sitter_go() -> Language;
}

/// Extract code entities and relationships from Go source code
pub fn extract(source: &str, file_path: &Path, config: &ParserConfig) -> Result<CodeIR, ParserError> {
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_go() };
    parser.set_language(language).map_err(|e| ParserError::ParseError(file_path.to_path_buf(), e.to_string()))?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        ParserError::ParseError(file_path.to_path_buf(), "Failed to parse".to_string())
    })?;

    let root_node = tree.root_node();
    if root_node.has_error() {
        return Err(ParserError::SyntaxError(file_path.to_path_buf(), 0, 0, "Syntax error".to_string()));
    }

    let mut ir = CodeIR::new(file_path.to_path_buf());

    let module_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
    ir.module = Some(ModuleEntity {
        name: module_name,
        path: file_path.display().to_string(),
        language: "go".to_string(),
        line_count: source.lines().count(),
        doc_comment: None,
        attributes: Vec::new(),
    });

    let mut visitor = GoVisitor::new(source.as_bytes(), config.clone());
    visitor.visit_node(root_node);

    ir.functions = visitor.functions;
    ir.classes = visitor.structs;
    ir.traits = visitor.interfaces;
    ir.imports = visitor.imports;

    Ok(ir)
}
