//! Integration tests for codegraph-rust parser

use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_rust::RustParser;
use std::path::Path;

#[test]
fn test_parse_simple_function() {
    let source = r#"
fn hello() {
    println!("Hello, world!");
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
    assert_eq!(info.line_count, 4);
}

#[test]
fn test_parse_function_with_parameters() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
}

#[test]
fn test_parse_async_function() {
    let source = r#"
async fn fetch_data() -> Result<String, Error> {
    Ok("data".to_string())
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
}

#[test]
fn test_parse_struct() {
    let source = r#"
pub struct Person {
    pub name: String,
    age: u32,
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);
}

#[test]
fn test_parse_enum() {
    let source = r#"
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);
}

#[test]
fn test_parse_trait() {
    let source = r#"
pub trait Drawable {
    fn draw(&self);
    fn clear(&mut self);
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.traits.len(), 1);
}

#[test]
fn test_parse_impl_block() {
    let source = r#"
struct Calculator;

impl Calculator {
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    fn subtract(a: i32, b: i32) -> i32 {
        a - b
    }
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);
    assert_eq!(info.functions.len(), 2); // Two methods
}

#[test]
fn test_parse_trait_implementation() {
    let source = r#"
trait Speak {
    fn speak(&self);
}

struct Dog;

impl Speak for Dog {
    fn speak(&self) {
        println!("Woof!");
    }
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.traits.len(), 1);
    assert_eq!(info.classes.len(), 1);
    assert_eq!(info.functions.len(), 1);
}

#[test]
fn test_parse_generic_struct() {
    let source = r#"
pub struct Container<T> {
    value: T,
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.classes.len(), 1);
}

#[test]
fn test_parse_use_statements() {
    let source = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use super::module;
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.imports.len(), 3);
}

#[test]
fn test_parse_complex_file() {
    let source = r#"
//! This is a module doc comment

use std::fmt;

/// A person struct
pub struct Person {
    pub name: String,
    age: u32,
}

impl Person {
    /// Create a new person
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    /// Get the person's age
    pub fn age(&self) -> u32 {
        self.age
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.age)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person() {
        let person = Person::new("Alice".to_string(), 30);
        assert_eq!(person.age(), 30);
    }
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("person.rs"), &mut graph);
    assert!(result.is_ok());

    let info = result.unwrap();
    // Should have Person struct, impl blocks with methods, test module
    assert!(info.classes.len() >= 1);
    assert!(info.functions.len() >= 2);
}

#[test]
fn test_syntax_error() {
    let source = "fn broken( { }"; // Invalid syntax

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = RustParser::new();

    let result = parser.parse_source(source, Path::new("test.rs"), &mut graph);
    assert!(result.is_err());
}

#[test]
fn test_parser_metrics() {
    let source = r#"
fn func1() {}
fn func2() {}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let mut parser = RustParser::new();

    let _ = parser.parse_source(source, Path::new("test.rs"), &mut graph);

    let metrics = parser.metrics();
    assert_eq!(metrics.files_attempted, 1);
    assert_eq!(metrics.files_succeeded, 1);

    parser.reset_metrics();
    let metrics = parser.metrics();
    assert_eq!(metrics.files_attempted, 0);
}
