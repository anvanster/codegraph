//! AST extraction for Rust source code
//!
//! This module parses Rust source code and extracts entities and relationships
//! into a CodeIR representation.

use codegraph_parser_api::{
    CallRelation, ClassEntity, CodeIR, FunctionEntity, ImplementationRelation, ImportRelation,
    InheritanceRelation, ModuleEntity, Parameter, ParserConfig, ParserError, TraitEntity,
};
use std::path::Path;
use syn::{visit::Visit, File, Item};

use crate::visitor::RustVisitor;

/// Extract code entities and relationships from Rust source code
///
/// # Arguments
/// * `source` - Rust source code as a string
/// * `file_path` - Path to the source file
/// * `config` - Parser configuration
///
/// # Returns
/// CodeIR containing all extracted entities and relationships
pub fn extract(
    source: &str,
    file_path: &Path,
    config: &ParserConfig,
) -> Result<CodeIR, ParserError> {
    // Parse the source code into a syn AST
    let syntax_tree = syn::parse_file(source).map_err(|e| {
        ParserError::SyntaxError(file_path.to_path_buf(), 0, 0, e.to_string())
    })?;

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
        language: "rust".to_string(),
        line_count: source.lines().count(),
        doc_comment: extract_file_doc(&syntax_tree),
        attributes: Vec::new(),
    };

    ir.module = Some(module);

    // Create visitor and walk the AST
    let mut visitor = RustVisitor::new(config.clone());
    visitor.visit_file(&syntax_tree);

    // Transfer extracted entities to IR
    ir.functions = visitor.functions;
    ir.classes = visitor.classes;
    ir.traits = visitor.traits;
    ir.imports = visitor.imports;
    ir.calls = visitor.calls;
    ir.implementations = visitor.implementations;
    ir.inheritance = visitor.inheritance;

    Ok(ir)
}

/// Extract documentation from the file level (if any)
fn extract_file_doc(file: &File) -> Option<String> {
    // Look for inner doc comments at the file level
    for attr in &file.attrs {
        if attr.path().is_ident("doc") {
            if let Ok(meta) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function() {
        let source = r#"
fn hello() {
    println!("Hello, world!");
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "hello");
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
pub struct MyStruct {
    field1: String,
    field2: i32,
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "MyStruct");
    }

    #[test]
    fn test_extract_trait() {
        let source = r#"
pub trait MyTrait {
    fn method(&self);
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.traits.len(), 1);
        assert_eq!(ir.traits[0].name, "MyTrait");
    }

    #[test]
    fn test_syntax_error() {
        let source = "fn hello( { "; // Invalid syntax
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParserError::SyntaxError(_, _, _, _)));
    }

    #[test]
    fn test_extract_enum() {
        let source = r#"
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Color");
    }

    #[test]
    fn test_extract_impl_block() {
        let source = r#"
struct Calculator;

impl Calculator {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        // Method should be extracted
        assert!(ir.functions.len() > 0 || ir.classes[0].methods.len() > 0);
    }

    #[test]
    fn test_extract_async_function() {
        let source = r#"
async fn fetch_data() -> String {
    "data".to_string()
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "fetch_data");
        assert!(ir.functions[0].is_async);
    }

    #[test]
    fn test_extract_use_statement() {
        let source = r#"
use std::collections::HashMap;
use std::fs;

fn test() {}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.imports.len(), 2);
    }

    #[test]
    fn test_extract_multiple_entities() {
        let source = r#"
use std::fmt;

pub trait Display {
    fn display(&self) -> String;
}

pub struct Person {
    name: String,
    age: u32,
}

impl Display for Person {
    fn display(&self) -> String {
        format!("{}: {}", self.name, self.age)
    }
}

pub fn greet(p: &Person) {
    println!("{}", p.display());
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.traits.len(), 1);
        assert_eq!(ir.traits[0].name, "Display");
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Person");
        assert!(ir.functions.len() >= 1); // greet function
        assert!(ir.imports.len() >= 1); // use std::fmt
    }

    #[test]
    fn test_extract_generic_struct() {
        let source = r#"
pub struct Container<T> {
    value: T,
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.classes.len(), 1);
        assert_eq!(ir.classes[0].name, "Container");
    }

    #[test]
    fn test_extract_module_info() {
        let source = r#"
fn test() {
    println!("test");
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("module.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.module.is_some());
        let module = ir.module.unwrap();
        assert_eq!(module.name, "module");
        assert_eq!(module.language, "rust");
        assert!(module.line_count > 0);
    }

    #[test]
    fn test_extract_visibility_modifiers() {
        let source = r#"
pub fn public_fn() {}
fn private_fn() {}
pub(crate) fn crate_fn() {}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 3);
    }

    #[test]
    fn test_extract_trait_implementation() {
        let source = r#"
pub trait Speak {
    fn speak(&self);
}

pub struct Dog;

impl Speak for Dog {
    fn speak(&self) {
        println!("Woof!");
    }
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.traits.len(), 1);
        assert_eq!(ir.classes.len(), 1);
        assert!(ir.implementations.len() > 0 || ir.classes[0].implemented_traits.len() > 0);
    }

    #[test]
    fn test_extract_test_function() {
        let source = r#"
#[test]
fn test_something() {
    assert_eq!(2 + 2, 4);
}
"#;
        let config = ParserConfig::default();
        let result = extract(source, Path::new("test.rs"), &config);

        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert!(ir.functions[0].is_test);
    }
}
