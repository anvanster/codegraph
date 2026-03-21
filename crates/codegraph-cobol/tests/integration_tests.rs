//! Integration tests for the COBOL parser

use codegraph::CodeGraph;
use codegraph_cobol::CobolParser;
use codegraph_parser_api::CodeParser;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// COBOL fixed-format helper: 7-space indent for Area A keywords
const INDENT: &str = "       ";

fn cobol_program(name: &str, body: &str) -> String {
    format!(
        "{INDENT}identification division.\n\
         {INDENT}program-id. {name}.\n\
         {body}"
    )
}

fn procedure_div(stmts: &str) -> String {
    format!("{INDENT}procedure division.\n{stmts}")
}

#[test]
fn test_parse_minimal_program() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = cobol_program("MINIMAL", &procedure_div(&format!("{INDENT}stop run.\n")));
    let result = parser.parse_source(&source, Path::new("minimal.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1, "Expected 1 program");
}

#[test]
fn test_program_name_extracted() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = cobol_program("HELLO", &procedure_div(&format!("{INDENT}stop run.\n")));
    let result = parser.parse_source(&source, Path::new("hello.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();

    assert_eq!(info.classes.len(), 1);
    let prog_node = graph.get_node(info.classes[0]).unwrap();
    let prog_name = prog_node.properties.get_string("name").unwrap();
    assert_eq!(prog_name, "HELLO");
}

#[test]
fn test_paragraph_extracted() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let body = procedure_div(&format!(
        "{INDENT}MAIN-PARA.\n\
         {INDENT}    stop run.\n"
    ));
    let source = cobol_program("PARA-PROG", &body);
    let result = parser.parse_source(&source, Path::new("para.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();

    assert_eq!(info.classes.len(), 1, "Expected 1 program");
    assert_eq!(info.functions.len(), 1, "Expected 1 paragraph");

    let para_node = graph.get_node(info.functions[0]).unwrap();
    let para_name = para_node.properties.get_string("name").unwrap();
    assert_eq!(para_name, "MAIN-PARA");
}

#[test]
fn test_multiple_paragraphs() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let body = procedure_div(&format!(
        "{INDENT}PARA-A.\n\
         {INDENT}    continue.\n\
         {INDENT}PARA-B.\n\
         {INDENT}    stop run.\n"
    ));
    let source = cobol_program("MULTI", &body);
    let result = parser.parse_source(&source, Path::new("multi.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();

    assert_eq!(info.functions.len(), 2, "Expected 2 paragraphs");
    let names: Vec<String> = info
        .functions
        .iter()
        .map(|id| {
            graph
                .get_node(*id)
                .unwrap()
                .properties
                .get_string("name")
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(names.contains(&"PARA-A".to_string()), "Missing PARA-A");
    assert!(names.contains(&"PARA-B".to_string()), "Missing PARA-B");
}

#[test]
fn test_copy_statement_extracted() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = format!(
        "{INDENT}identification division.\n\
         {INDENT}program-id. COPYPROG.\n\
         {INDENT}data division.\n\
         {INDENT}working-storage section.\n\
         {INDENT}copy MYBOOK.\n\
         {INDENT}procedure division.\n\
         {INDENT}stop run.\n"
    );
    let result = parser.parse_source(&source, Path::new("copy.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();

    assert!(
        !info.imports.is_empty(),
        "Expected COPY statement as import"
    );
    let import_node = graph.get_node(info.imports[0]).unwrap();
    let imported = import_node.properties.get_string("name").unwrap();
    assert_eq!(imported, "MYBOOK");
}

#[test]
fn test_can_parse_extensions() {
    let parser = CobolParser::new();
    assert!(parser.can_parse(Path::new("program.cob")));
    assert!(parser.can_parse(Path::new("program.cbl")));
    assert!(parser.can_parse(Path::new("program.cobol")));
    assert!(parser.can_parse(Path::new("copybook.cpy")));
    assert!(!parser.can_parse(Path::new("script.py")));
    assert!(!parser.can_parse(Path::new("main.rs")));
}

#[test]
fn test_file_info_line_count() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = cobol_program("LINES", &procedure_div(&format!("{INDENT}stop run.\n")));
    let result = parser.parse_source(&source, Path::new("lines.cob"), &mut graph);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.line_count > 0);
}

#[test]
fn test_parse_file_from_disk() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = cobol_program("FILEPROG", &procedure_div(&format!("{INDENT}stop run.\n")));
    let mut tmp = NamedTempFile::with_suffix(".cob").unwrap();
    tmp.write_all(source.as_bytes()).unwrap();

    let result = parser.parse_file(tmp.path(), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);

    // Verify metrics updated
    let metrics = parser.metrics();
    assert_eq!(metrics.files_attempted, 1);
    assert_eq!(metrics.files_succeeded, 1);
}

#[test]
fn test_language_identifier() {
    let parser = CobolParser::new();
    assert_eq!(parser.language(), "cobol");
}

#[test]
fn test_paragraph_linked_to_program() {
    let parser = CobolParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let body = procedure_div(&format!("{INDENT}MAIN-PARA.\n{INDENT}    stop run.\n"));
    let source = cobol_program("LINKED", &body);
    let result = parser.parse_source(&source, Path::new("linked.cob"), &mut graph);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let info = result.unwrap();

    assert_eq!(info.classes.len(), 1);
    assert_eq!(info.functions.len(), 1);

    // Verify the paragraph has parent_class set to program name
    let para_node = graph.get_node(info.functions[0]).unwrap();
    let parent = para_node.properties.get_string("parent_class");
    assert_eq!(
        parent,
        Some("LINKED"),
        "Paragraph should be linked to LINKED program"
    );
}
