//! Integration tests for codegraph-go parser

use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_go::GoParser;
use std::path::Path;

#[test]
fn test_parse_simple_function() {
    let source = r#"
package main

func hello() {
    println("Hello, world!")
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = GoParser::new();

    let result = parser.parse_source(source, Path::new("main.go"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
}

#[test]
fn test_parser_info() {
    let parser = GoParser::new();
    assert_eq!(parser.language(), "go");
    assert!(parser.can_parse(Path::new("main.go")));
}
