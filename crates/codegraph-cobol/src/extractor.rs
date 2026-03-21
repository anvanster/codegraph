//! AST extraction for COBOL source code

use codegraph_parser_api::{CodeIR, ModuleEntity, ParserConfig, ParserError};
use std::path::Path;
use tree_sitter::Parser;

use codegraph_parser_api::{CallRelation, FunctionEntity};

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

    // Source-level extraction for constructs the grammar doesn't parse:
    // GO TO and EXEC CICS XCTL/LINK
    extract_goto_and_cics(source, &ir.functions, &mut ir.calls);

    Ok(ir)
}

/// Source-level extraction for GO TO and EXEC CICS constructs.
///
/// The tree-sitter COBOL grammar doesn't parse GO TO as a distinct node type
/// and doesn't understand CICS extensions at all. We extract these from source
/// text directly.
fn extract_goto_and_cics(
    source: &str,
    paragraphs: &[FunctionEntity],
    calls: &mut Vec<CallRelation>,
) {
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let line_1indexed = line_num + 1;

        // Find which paragraph this line belongs to
        let caller = paragraphs
            .iter()
            .rfind(|p| line_1indexed >= p.line_start && line_1indexed <= p.line_end)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "file".to_string());

        // GO TO paragraph-name
        if let Some(target) = trimmed
            .strip_prefix("GO TO ")
            .or_else(|| trimmed.strip_prefix("GO  TO "))
            .or_else(|| trimmed.strip_prefix("go to "))
        {
            let target = target.trim().trim_end_matches('.');
            if !target.is_empty() && !target.contains(' ') {
                calls.push(CallRelation::new(
                    caller.clone(),
                    target.to_string(),
                    line_1indexed,
                ));
            }
        }

        // EXEC CICS XCTL PROGRAM('name') or EXEC CICS LINK PROGRAM('name')
        if (trimmed.contains("EXEC CICS XCTL") || trimmed.contains("EXEC CICS LINK"))
            && trimmed.contains("PROGRAM")
        {
            // Program name might be on this line or next
            if let Some(prog) = extract_cics_program_name(trimmed) {
                calls.push(CallRelation::new(caller.clone(), prog, line_1indexed));
            }
        }
        // PROGRAM clause might be on the next line after EXEC CICS XCTL
        if trimmed.starts_with("PROGRAM") && trimmed.contains('(') {
            if let Some(prog) = extract_cics_program_name(trimmed) {
                calls.push(CallRelation::new(caller.clone(), prog, line_1indexed));
            }
        }
    }
}

/// Extract program name from CICS PROGRAM clause.
/// Handles: PROGRAM('MYPROG'), PROGRAM(WS-PROGNAME), PROGRAM (CDEMO-TO-PROGRAM)
fn extract_cics_program_name(text: &str) -> Option<String> {
    let idx = text.find("PROGRAM")?;
    let rest = &text[idx + 7..];
    let rest = rest.trim();
    let rest = rest.strip_prefix('(')?;
    let end = rest.find(')')?;
    let name = rest[..end].trim().trim_matches('\'').trim_matches('"');
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
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

    #[test]
    fn test_extract_perform_calls() {
        let source = concat!(
            "       identification division.\n",
            "       program-id. PERFTEST.\n",
            "       procedure division.\n",
            "       MAIN-PARA.\n",
            "           perform INIT-PARA\n",
            "           perform PROCESS-DATA\n",
            "           stop run.\n",
            "       INIT-PARA.\n",
            "           display 'init'.\n",
            "       PROCESS-DATA.\n",
            "           display 'process'.\n",
        );
        let config = ParserConfig::default();
        let result = extract(source, Path::new("perf.cob"), &config);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();

        eprintln!(
            "Functions: {:?}",
            ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        eprintln!(
            "Calls: {:?}",
            ir.calls
                .iter()
                .map(|c| format!("{} -> {}", c.caller, c.callee))
                .collect::<Vec<_>>()
        );

        assert!(
            !ir.calls.is_empty(),
            "Expected PERFORM calls to be extracted"
        );

        let callees: Vec<&str> = ir.calls.iter().map(|c| c.callee.as_str()).collect();
        assert!(
            callees.contains(&"INIT-PARA"),
            "Expected PERFORM INIT-PARA. Got: {:?}",
            callees
        );
        assert!(
            callees.contains(&"PROCESS-DATA"),
            "Expected PERFORM PROCESS-DATA. Got: {:?}",
            callees
        );
    }

    #[test]
    fn test_extract_goto_and_cics() {
        let source = concat!(
            "       identification division.\n",
            "       program-id. GOTEST.\n",
            "       procedure division.\n",
            "       MAIN-PARA.\n",
            "           GO TO EXIT-PARA\n",
            "           stop run.\n",
            "       EXIT-PARA.\n",
            "           exit.\n",
        );
        let config = ParserConfig::default();
        let result = extract(source, Path::new("goto.cob"), &config);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let ir = result.unwrap();

        let callees: Vec<&str> = ir.calls.iter().map(|c| c.callee.as_str()).collect();
        eprintln!("GO TO calls: {:?}", callees);
        assert!(
            callees.contains(&"EXIT-PARA"),
            "Expected GO TO EXIT-PARA. Got: {:?}",
            callees
        );
    }
}
