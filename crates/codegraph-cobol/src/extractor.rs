//! AST extraction for COBOL source code

use codegraph_parser_api::{CodeIR, ModuleEntity, ParserConfig, ParserError};
use std::path::Path;
use tree_sitter::Parser;

use crate::visitor::CobolVisitor;

/// Extract code entities and relationships from COBOL source code.
pub fn extract(
    source: &str,
    file_path: &Path,
    _config: &ParserConfig,
) -> Result<CodeIR, ParserError> {
    let mut parser = Parser::new();
    let language = crate::ts_cobol::language();
    parser
        .set_language(&language)
        .map_err(|e| ParserError::ParseError(file_path.to_path_buf(), e.to_string()))?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        ParserError::ParseError(file_path.to_path_buf(), "Failed to parse".to_string())
    })?;

    // Note: NOT checking root_node.has_error() — COBOL dialects and complex
    // preprocessor directives can produce partial error nodes in the grammar
    // while still containing extractable entities.
    let root_node = tree.root_node();

    let mut ir = CodeIR::new(file_path.to_path_buf());

    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    ir.module = Some(ModuleEntity {
        name: module_name,
        path: file_path.display().to_string(),
        language: "cobol".to_string(),
        line_count: source.lines().count(),
        doc_comment: None,
        attributes: Vec::new(),
    });

    let mut visitor = CobolVisitor::new(source.as_bytes());
    visitor.visit_node(root_node);

    // Map COBOL programs to classes and paragraphs to functions
    ir.classes = visitor.programs;
    ir.functions = visitor.paragraphs;
    ir.imports = visitor.imports;
    ir.calls = visitor.calls;

    Ok(ir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_COBOL: &str = concat!(
        "       identification division.\n",
        "       program-id. MINIMAL.\n",
        "       procedure division.\n",
        "       stop run.\n",
    );

    const COBOL_WITH_PARAGRAPH: &str = concat!(
        "       identification division.\n",
        "       program-id. MYPROG.\n",
        "       procedure division.\n",
        "       MAIN-PARA.\n",
        "           stop run.\n",
    );

    const COBOL_WITH_COPY: &str = concat!(
        "       identification division.\n",
        "       program-id. COPYPROG.\n",
        "       data division.\n",
        "       working-storage section.\n",
        "       copy MYBOOK.\n",
        "       procedure division.\n",
        "       stop run.\n",
    );

    #[test]
    fn test_extract_minimal_cobol() {
        let config = ParserConfig::default();
        let result = extract(MINIMAL_COBOL, Path::new("minimal.cob"), &config);

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "MINIMAL");
    }

    #[test]
    fn test_extract_program_name() {
        let config = ParserConfig::default();
        let result = extract(MINIMAL_COBOL, Path::new("test.cob"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.module.is_some());
        let module = ir.module.unwrap();
        assert_eq!(module.name, "test");
        assert_eq!(module.language, "cobol");
    }

    #[test]
    fn test_extract_paragraph() {
        let config = ParserConfig::default();
        let result = extract(COBOL_WITH_PARAGRAPH, Path::new("para.cob"), &config);

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "MAIN-PARA");
        assert_eq!(ir.functions[0].parent_class, Some("MYPROG".to_string()));
    }

    #[test]
    fn test_extract_copy_statement() {
        let config = ParserConfig::default();
        let result = extract(COBOL_WITH_COPY, Path::new("copy.cob"), &config);

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();
        assert!(!ir.imports.is_empty(), "Expected COPY import");
        assert_eq!(ir.imports[0].imported, "MYBOOK");
    }

    #[test]
    fn test_extract_module_line_count() {
        let config = ParserConfig::default();
        let result = extract(MINIMAL_COBOL, Path::new("count.cob"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        let module = ir.module.unwrap();
        assert!(module.line_count > 0);
    }

    #[test]
    fn test_extract_multiple_paragraphs() {
        let source = concat!(
            "       identification division.\n",
            "       program-id. MULTI.\n",
            "       procedure division.\n",
            "       PARA-A.\n",
            "           continue.\n",
            "       PARA-B.\n",
            "           stop run.\n",
        );
        let config = ParserConfig::default();
        let result = extract(source, Path::new("multi.cob"), &config);

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.functions.len(), 2);
        let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"PARA-A"),
            "Missing PARA-A, got: {:?}",
            names
        );
        assert!(
            names.contains(&"PARA-B"),
            "Missing PARA-B, got: {:?}",
            names
        );
    }
}
