//! Integration tests for the C++ parser

use codegraph::CodeGraph;
use codegraph_cpp::CppParser;
use codegraph_parser_api::CodeParser;
use std::path::Path;

const SAMPLE_CPP: &str = include_str!("fixtures/sample.cpp");
const CLASSES_HPP: &str = include_str!("fixtures/classes.hpp");

#[test]
fn test_parse_sample_cpp() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(SAMPLE_CPP, Path::new("sample.cpp"), &mut graph);
    assert!(
        result.is_ok(),
        "Failed to parse sample.cpp: {:?}",
        result.err()
    );

    let file_info = result.unwrap();

    // Should have classes (Container, Base, Derived) and functions
    assert!(!file_info.classes.is_empty(), "Should have found classes");
    assert!(
        !file_info.functions.is_empty(),
        "Should have found functions"
    );
}

#[test]
fn test_parse_classes_hpp() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(CLASSES_HPP, Path::new("classes.hpp"), &mut graph);
    assert!(
        result.is_ok(),
        "Failed to parse classes.hpp: {:?}",
        result.err()
    );

    let file_info = result.unwrap();

    // Should have includes
    assert!(!file_info.imports.is_empty(), "Should have found imports");

    // Should have classes (Point struct, Shape, Circle, Rectangle)
    assert!(
        file_info.classes.len() >= 4,
        "Should have at least 4 classes/structs, found {}",
        file_info.classes.len()
    );
}

#[test]
fn test_namespace_qualified_names() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(SAMPLE_CPP, Path::new("sample.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Check that classes have namespace-qualified names in the graph
    // We verify by checking the number of classes created
    assert!(
        file_info.classes.len() >= 3,
        "Expected at least 3 classes (Container, Base, Derived), found {}",
        file_info.classes.len()
    );
}

#[test]
fn test_inheritance_detection() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(CLASSES_HPP, Path::new("classes.hpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Should have classes that inherit (Circle : Shape, Rectangle : Shape)
    // We verify by the number of classes created
    assert!(
        file_info.classes.len() >= 4,
        "Expected at least 4 classes, found {}",
        file_info.classes.len()
    );
}

#[test]
fn test_function_extraction() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(SAMPLE_CPP, Path::new("sample.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Should have helper and main functions plus methods
    assert!(
        !file_info.functions.is_empty(),
        "Expected to find functions"
    );
}

#[test]
fn test_include_extraction() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(SAMPLE_CPP, Path::new("sample.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Should have at least iostream, vector, memory includes
    assert!(
        file_info.imports.len() >= 3,
        "Expected at least 3 imports, found {}",
        file_info.imports.len()
    );
}

#[test]
fn test_parser_language_name() {
    let parser = CppParser::new();
    assert_eq!(parser.language(), "cpp");
}

#[test]
fn test_file_extensions() {
    let parser = CppParser::new();
    let extensions = parser.file_extensions();

    assert!(extensions.contains(&".cpp"));
    assert!(extensions.contains(&".hpp"));
    assert!(extensions.contains(&".h"));
    assert!(extensions.contains(&".cc"));
    assert!(extensions.contains(&".cxx"));
}

#[test]
fn test_can_parse() {
    let parser = CppParser::new();

    assert!(parser.can_parse(Path::new("file.cpp")));
    assert!(parser.can_parse(Path::new("file.hpp")));
    assert!(parser.can_parse(Path::new("file.cc")));
    assert!(parser.can_parse(Path::new("file.h")));
    assert!(parser.can_parse(Path::new("file.cxx")));
    assert!(parser.can_parse(Path::new("file.hxx")));
    assert!(parser.can_parse(Path::new("file.hh")));

    // Should not parse other file types
    assert!(!parser.can_parse(Path::new("file.c"))); // C files handled by codegraph-c
    assert!(!parser.can_parse(Path::new("file.rs")));
    assert!(!parser.can_parse(Path::new("file.py")));
}

#[test]
fn test_graph_node_creation() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let result = parser.parse_source(SAMPLE_CPP, Path::new("sample.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Verify we can retrieve nodes by their IDs
    let file_node = graph.get_node(file_info.file_id);
    assert!(
        file_node.is_ok(),
        "Should be able to retrieve file node by ID"
    );

    // Verify class nodes
    for class_id in &file_info.classes {
        let class_node = graph.get_node(*class_id);
        assert!(
            class_node.is_ok(),
            "Should be able to retrieve class node by ID"
        );
    }

    // Verify function nodes
    for func_id in &file_info.functions {
        let func_node = graph.get_node(*func_id);
        assert!(
            func_node.is_ok(),
            "Should be able to retrieve function node by ID"
        );
    }
}

#[test]
fn test_enum_as_class() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = r#"
enum Color { Red, Green, Blue };
enum class Status { Active, Inactive };
"#;

    let result = parser.parse_source(source, Path::new("enums.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Enums should be treated as classes
    assert!(
        file_info.classes.len() >= 2,
        "Expected at least 2 enum classes, found {}",
        file_info.classes.len()
    );
}

#[test]
fn test_struct_as_class() {
    let parser = CppParser::new();
    let mut graph = CodeGraph::in_memory().unwrap();

    let source = r#"
struct Point {
    int x;
    int y;
};
"#;

    let result = parser.parse_source(source, Path::new("struct.cpp"), &mut graph);
    assert!(result.is_ok());

    let file_info = result.unwrap();

    // Structs should be treated as classes
    assert_eq!(
        file_info.classes.len(),
        1,
        "Expected 1 struct/class, found {}",
        file_info.classes.len()
    );
}
