//! AST extraction for TypeScript/JavaScript source code

use codegraph_parser_api::{CodeIR, ModuleEntity, ParserConfig, ParserError};
use std::path::Path;
use tree_sitter::{Language, Parser};

use crate::visitor::TypeScriptVisitor;

extern "C" {
    fn tree_sitter_typescript() -> Language;
}

/// Extract code entities and relationships from TypeScript/JavaScript source code
pub fn extract(
    source: &str,
    file_path: &Path,
    config: &ParserConfig,
) -> Result<CodeIR, ParserError> {
    // Create tree-sitter parser
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_typescript() };
    parser
        .set_language(language)
        .map_err(|e| ParserError::ParseError(file_path.to_path_buf(), e.to_string()))?;

    // Parse the source code
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| {
            ParserError::ParseError(file_path.to_path_buf(), "Failed to parse".to_string())
        })?;

    // Check for syntax errors
    let root_node = tree.root_node();
    if root_node.has_error() {
        return Err(ParserError::SyntaxError(
            file_path.to_path_buf(),
            0,
            0,
            "Syntax error in source code".to_string(),
        ));
    }

    // Create IR for this file
    let mut ir = CodeIR::new(file_path.to_path_buf());

    // Create module entity for the file
    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let module = ModuleEntity {
        name: module_name.clone(),
        path: file_path.display().to_string(),
        language: "typescript".to_string(),
        line_count: source.lines().count(),
        doc_comment: None,
        attributes: Vec::new(),
    };

    ir.module = Some(module);

    // Create visitor and walk the AST
    let mut visitor = TypeScriptVisitor::new(source.as_bytes(), config.clone());
    visitor.visit_node(root_node);

    // Transfer extracted entities to IR
    ir.functions = visitor.functions;
    ir.classes = visitor.classes;
    ir.traits = visitor.interfaces;
    ir.imports = visitor.imports;
    ir.calls = visitor.calls;
    ir.implementations = visitor.implementations;
    ir.inheritance = visitor.inheritance;

    Ok(ir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function() {
        let source = r#"
function hello() {
    console.log("Hello, world!");
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.ts"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "hello");
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
class Person {
    name: string;
    age: number;
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.ts"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Person");
    }
}
