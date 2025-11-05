//! Integration tests for codegraph-typescript parser

use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_typescript::TypeScriptParser;
use std::path::Path;

#[test]
fn test_parse_simple_function() {
    let source = r#"
function hello() {
    console.log("Hello, world!");
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = TypeScriptParser::new();

    let result = parser.parse_source(source, Path::new("test.ts"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
}

#[test]
fn test_parse_class() {
    let source = r#"
class Person {
    name: string;
    age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = TypeScriptParser::new();

    let result = parser.parse_source(source, Path::new("test.ts"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);
}

#[test]
fn test_parser_info() {
    let parser = TypeScriptParser::new();
    assert_eq!(parser.language(), "typescript");
    assert!(parser.can_parse(Path::new("test.ts")));
    assert!(parser.can_parse(Path::new("test.js")));
}
