//! AST extraction for C source code
//!
//! This module provides two parsing modes:
//! - Strict mode: Fails on syntax errors (default, for clean code)
//! - Tolerant mode: Extracts what it can even with errors (for real-world code)

use codegraph_parser_api::{CodeIR, ModuleEntity, ParserConfig, ParserError};
use std::path::Path;
use tree_sitter::Parser;

use crate::preprocessor::CPreprocessor;
use crate::visitor::CVisitor;

/// Extraction options for controlling parser behavior
#[derive(Debug, Clone, Default)]
pub struct ExtractionOptions {
    /// If true, extract entities even when the AST has errors
    pub tolerant_mode: bool,
    /// If true, apply preprocessing to help parse kernel/system code
    pub preprocess: bool,
    /// If true, extract function calls for call graph
    pub extract_calls: bool,
}

impl ExtractionOptions {
    /// Create options optimized for kernel/system code
    pub fn for_kernel_code() -> Self {
        Self {
            tolerant_mode: true,
            preprocess: true,
            extract_calls: true,
        }
    }

    /// Create options for tolerant parsing of any code
    pub fn tolerant() -> Self {
        Self {
            tolerant_mode: true,
            preprocess: false,
            extract_calls: true,
        }
    }
}

/// Result of extraction with additional metadata
#[derive(Debug)]
pub struct ExtractionResult {
    pub ir: CodeIR,
    /// Number of syntax errors encountered (0 = clean parse)
    pub error_count: usize,
    /// Whether the file was fully parsed or partially
    pub is_partial: bool,
    /// Macros detected in the source
    pub detected_macros: Vec<String>,
}

/// Extract code entities and relationships from C source code (strict mode)
pub fn extract(
    source: &str,
    file_path: &Path,
    _config: &ParserConfig,
) -> Result<CodeIR, ParserError> {
    let result = extract_with_options(source, file_path, &ExtractionOptions::default())?;

    if result.is_partial {
        return Err(ParserError::SyntaxError(
            file_path.to_path_buf(),
            0,
            0,
            "Syntax error".to_string(),
        ));
    }

    Ok(result.ir)
}

/// Extract with custom options (supports tolerant mode)
pub fn extract_with_options(
    source: &str,
    file_path: &Path,
    options: &ExtractionOptions,
) -> Result<ExtractionResult, ParserError> {
    // Detect macros from original source (before preprocessing)
    let preprocessor = CPreprocessor::new();
    let detected_macros: Vec<String> = preprocessor
        .analyze_macros(source)
        .iter()
        .map(|m| m.name.clone())
        .collect();

    // Optionally preprocess the source
    let processed_source = if options.preprocess {
        preprocessor.preprocess(source)
    } else {
        source.to_string()
    };

    let mut parser = Parser::new();
    let language = tree_sitter_c::language();
    parser
        .set_language(&language)
        .map_err(|e| ParserError::ParseError(file_path.to_path_buf(), e.to_string()))?;

    let tree = parser.parse(&processed_source, None).ok_or_else(|| {
        ParserError::ParseError(file_path.to_path_buf(), "Failed to parse".to_string())
    })?;

    let root_node = tree.root_node();
    let has_error = root_node.has_error();
    let error_count = if has_error {
        count_errors(root_node)
    } else {
        0
    };

    // In strict mode, fail on errors
    if has_error && !options.tolerant_mode {
        return Err(ParserError::SyntaxError(
            file_path.to_path_buf(),
            0,
            0,
            format!("Syntax error ({error_count} error nodes)"),
        ));
    }

    let mut ir = CodeIR::new(file_path.to_path_buf());

    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    ir.module = Some(ModuleEntity {
        name: module_name,
        path: file_path.display().to_string(),
        language: "c".to_string(),
        line_count: source.lines().count(),
        doc_comment: None,
        attributes: Vec::new(),
    });

    // Visit the AST - the visitor will skip ERROR nodes gracefully
    let mut visitor = CVisitor::new(processed_source.as_bytes());
    visitor.set_extract_calls(options.extract_calls);
    visitor.visit_node(root_node);

    ir.functions = visitor.functions;
    ir.classes = visitor.structs;
    ir.imports = visitor.imports;

    // Store call relationships in a custom way (we'll add this to IR later)
    // For now, calls are stored as part of function entities

    Ok(ExtractionResult {
        ir,
        error_count,
        is_partial: has_error,
        detected_macros,
    })
}

/// Count ERROR nodes in the syntax tree
fn count_errors(node: tree_sitter::Node) -> usize {
    let mut count = 0;

    if node.is_error() || node.is_missing() {
        count += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_errors(child);
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function() {
        let source = r#"
int main() {
    return 0;
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "main");
    }

    #[test]
    fn test_extract_function_with_params() {
        let source = r#"
int add(int a, int b) {
    return a + b;
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "add");
        assert_eq!(ir.functions[0].parameters.len(), 2);
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
struct Point {
    int x;
    int y;
};
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Point");
    }

    #[test]
    fn test_extract_enum() {
        let source = r#"
enum Color {
    RED,
    GREEN,
    BLUE
};
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Color");
    }

    #[test]
    fn test_extract_include() {
        let source = r#"
#include <stdio.h>
#include "myheader.h"
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.imports.len(), 2);
    }

    #[test]
    fn test_extract_multiple_functions() {
        let source = r#"
int foo() { return 1; }
int bar() { return 2; }
int baz() { return 3; }
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 3);
    }

    #[test]
    fn test_extract_static_function() {
        let source = r#"
static void helper() {
    // internal function
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].visibility, "private");
    }

    #[test]
    fn test_extract_module_info() {
        let source = r#"
int test() {
    return 42;
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("module.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.module.is_some());
        let module = ir.module.unwrap();
        assert_eq!(module.name, "module");
        assert_eq!(module.language, "c");
        assert!(module.line_count > 0);
    }

    #[test]
    fn test_extract_with_syntax_error_strict() {
        let source = r#"
int broken( {
    // Missing closing brace
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_err());
        match result {
            Err(ParserError::SyntaxError(..)) => (),
            _ => panic!("Expected SyntaxError"),
        }
    }

    #[test]
    fn test_extract_with_syntax_error_tolerant() {
        let source = r#"
int valid_func() { return 0; }
int broken( {
int another_valid() { return 1; }
"#;
        let options = ExtractionOptions::tolerant();
        let result = extract_with_options(source, Path::new("test.c"), &options);

        assert!(result.is_ok());
        let extraction = result.unwrap();
        assert!(extraction.is_partial);
        assert!(extraction.error_count > 0);
        // Should still extract the valid functions
        assert!(!extraction.ir.functions.is_empty());
    }

    #[test]
    fn test_extract_kernel_code_simulation() {
        let source = r#"
static __init int my_module_init(void) {
    return 0;
}

static __exit void my_module_exit(void) {
}

MODULE_LICENSE("GPL");
"#;
        let options = ExtractionOptions::for_kernel_code();
        let result = extract_with_options(source, Path::new("test.c"), &options);

        // With preprocessing, this should parse better
        assert!(result.is_ok());
        let extraction = result.unwrap();
        // Check that macros were detected
        assert!(
            extraction.detected_macros.contains(&"__init".to_string())
                || extraction.detected_macros.contains(&"__exit".to_string())
        );
    }

    #[test]
    fn test_extract_pointer_params() {
        let source = r#"
void process(int *arr, const char *str) {
    // pointer parameters
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].parameters.len(), 2);
    }

    #[test]
    fn test_extract_union() {
        let source = r#"
union Data {
    int i;
    float f;
    char c;
};
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Data");
    }

    #[test]
    fn test_extract_function_with_complexity() {
        let source = r#"
int complex_func(int x) {
    if (x > 0) {
        for (int i = 0; i < x; i++) {
            if (i % 2 == 0) {
                continue;
            }
        }
        return 1;
    } else if (x < 0) {
        while (x < 0) {
            x++;
        }
        return -1;
    }
    return 0;
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.c"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        // Check that complexity metrics are populated
        let func = &ir.functions[0];
        assert!(func.complexity.is_some());
        let complexity = func.complexity.as_ref().unwrap();
        assert!(complexity.cyclomatic_complexity > 1);
    }
}
