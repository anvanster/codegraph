use crate::config::ParserConfig;
use crate::entities::{ClassEntity, FunctionEntity, ModuleEntity, TraitEntity};
use crate::relationships::{
    CallRelation, ImplementationRelation, ImportEntity, InheritanceRelation,
};
use rustpython_ast::Stmt;
use rustpython_parser::{ast, Parse};
use std::path::Path;

/// Intermediate representation of extracted code
///
/// This struct holds all entities and relationships extracted from Python source
/// before they are added to the graph database. This separation allows for:
/// - Easier testing of extraction logic without graph database
/// - Batch operations for better performance
/// - Clean separation of concerns (extraction vs graph building)
#[derive(Debug, Default)]
pub struct CodeIR {
    /// Module/file entity
    pub module: Option<ModuleEntity>,

    /// Functions extracted from the source
    pub functions: Vec<FunctionEntity>,

    /// Classes extracted from the source
    pub classes: Vec<ClassEntity>,

    /// Traits/Protocols extracted from the source
    pub traits: Vec<TraitEntity>,

    /// Function call relationships
    pub calls: Vec<CallRelation>,

    /// Import relationships
    pub imports: Vec<ImportEntity>,

    /// Inheritance relationships
    pub inheritance: Vec<InheritanceRelation>,

    /// Protocol/ABC implementation relationships
    #[allow(dead_code)]
    pub implementations: Vec<ImplementationRelation>,
}

impl CodeIR {
    /// Create a new empty CodeIR
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function entity
    pub fn add_function(&mut self, func: FunctionEntity) {
        self.functions.push(func);
    }

    /// Add a class entity
    pub fn add_class(&mut self, class: ClassEntity) {
        self.classes.push(class);
    }

    /// Add a trait entity
    #[allow(dead_code)]
    pub fn add_trait(&mut self, trait_entity: TraitEntity) {
        self.traits.push(trait_entity);
    }

    /// Add a call relationship
    pub fn add_call(&mut self, call: CallRelation) {
        self.calls.push(call);
    }

    /// Add an import relationship
    pub fn add_import(&mut self, import: ImportEntity) {
        self.imports.push(import);
    }

    /// Add an inheritance relationship
    pub fn add_inheritance(&mut self, inheritance: InheritanceRelation) {
        self.inheritance.push(inheritance);
    }

    /// Add an implementation relationship
    #[allow(dead_code)]
    pub fn add_implementation(&mut self, implementation: ImplementationRelation) {
        self.implementations.push(implementation);
    }

    /// Get total number of entities
    #[allow(dead_code)]
    pub fn entity_count(&self) -> usize {
        self.functions.len()
            + self.classes.len()
            + self.traits.len()
            + if self.module.is_some() { 1 } else { 0 }
    }

    /// Get total number of relationships
    #[allow(dead_code)]
    pub fn relationship_count(&self) -> usize {
        self.calls.len() + self.imports.len() + self.inheritance.len() + self.implementations.len()
    }
}

/// Extract all entities and relationships from a Python AST
///
/// This is a simplified implementation that will be expanded in subsequent tasks.
/// For now, it provides basic extraction of functions and classes.
pub fn extract(source: &str, file_path: &Path, _config: &ParserConfig) -> Result<CodeIR, String> {
    // Parse the source code
    let ast = ast::Suite::parse(source, &file_path.display().to_string())
        .map_err(|e| format!("Parse error: {e:?}"))?;

    let mut ir = CodeIR::new();

    // Extract module entity
    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

    // Count lines
    let line_count = source.lines().count();
    let module = ModuleEntity::new(module_name, line_count);
    ir.module = Some(module);

    // Walk through statements and extract entities
    for (idx, stmt) in ast.iter().enumerate() {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                // Create basic function entity (top-level functions only)
                let line_start = idx + 1; // Simplified line numbering
                let line_end = line_start + func_def.body.len();
                let func = FunctionEntity::new(func_def.name.as_str(), line_start, line_end);
                ir.add_function(func);

                // Extract calls from function body
                let calls =
                    extract_calls_from_body(&func_def.body, func_def.name.as_str(), line_start);
                for call in calls {
                    ir.add_call(call);
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                let line_start = idx + 1;
                let line_end = line_start + func_def.body.len();
                let func = FunctionEntity::new(func_def.name.as_str(), line_start, line_end)
                    .set_async(true);
                ir.add_function(func);

                // Extract calls from async function body
                let calls =
                    extract_calls_from_body(&func_def.body, func_def.name.as_str(), line_start);
                for call in calls {
                    ir.add_call(call);
                }
            }
            Stmt::ClassDef(class_def) => {
                let line_start = idx + 1;
                let line_end = line_start + class_def.body.len();

                // Extract methods from class body
                let mut methods = Vec::new();
                for (method_idx, method_stmt) in class_def.body.iter().enumerate() {
                    match method_stmt {
                        Stmt::FunctionDef(method_def) => {
                            let method_line_start = line_start + method_idx + 1;
                            let method_line_end = method_line_start + method_def.body.len();
                            let method = FunctionEntity::new(
                                method_def.name.as_str(),
                                method_line_start,
                                method_line_end,
                            );
                            methods.push(method);

                            // Extract calls from method body
                            let method_qualified_name =
                                format!("{}.{}", class_def.name.as_str(), method_def.name.as_str());
                            let calls = extract_calls_from_body(
                                &method_def.body,
                                &method_qualified_name,
                                method_line_start,
                            );
                            for call in calls {
                                ir.add_call(call);
                            }
                        }
                        Stmt::AsyncFunctionDef(method_def) => {
                            let method_line_start = line_start + method_idx + 1;
                            let method_line_end = method_line_start + method_def.body.len();
                            let method = FunctionEntity::new(
                                method_def.name.as_str(),
                                method_line_start,
                                method_line_end,
                            )
                            .set_async(true);
                            methods.push(method);

                            // Extract calls from async method body
                            let method_qualified_name =
                                format!("{}.{}", class_def.name.as_str(), method_def.name.as_str());
                            let calls = extract_calls_from_body(
                                &method_def.body,
                                &method_qualified_name,
                                method_line_start,
                            );
                            for call in calls {
                                ir.add_call(call);
                            }
                        }
                        _ => {
                            // Other statement types in class body (assignments, etc.)
                        }
                    }
                }

                let mut class = ClassEntity::new(class_def.name.as_str(), line_start, line_end);
                class.methods = methods;
                ir.add_class(class);

                // Extract inheritance relationships
                for base in &class_def.bases {
                    if let Some(parent_name) = extract_base_class_name(base) {
                        let inheritance = InheritanceRelation::new(
                            class_def.name.as_str(),
                            parent_name,
                            line_start,
                        );
                        ir.add_inheritance(inheritance);
                    }
                }
            }
            Stmt::Import(import) => {
                // Handle: import module1, module2, module3
                let line = idx + 1;
                for alias in &import.names {
                    let module_name = alias.name.to_string();
                    let import_entity =
                        ImportEntity::new(module_name.clone(), vec![module_name], line);
                    ir.add_import(import_entity);
                }
            }
            Stmt::ImportFrom(import_from) => {
                // Handle: from module import name1, name2, *
                let line = idx + 1;
                let from_module = import_from
                    .module
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| ".".to_string());

                // Check for wildcard import
                let is_wildcard = import_from
                    .names
                    .iter()
                    .any(|alias| alias.name.as_str() == "*");

                if is_wildcard {
                    let import_entity = ImportEntity::wildcard(from_module, line);
                    ir.add_import(import_entity);
                } else {
                    let imported_items: Vec<String> = import_from
                        .names
                        .iter()
                        .map(|alias| alias.name.to_string())
                        .collect();
                    let import_entity = ImportEntity::new(from_module, imported_items, line);
                    ir.add_import(import_entity);
                }
            }
            _ => {
                // Other statement types will be handled in future phases
            }
        }
    }

    Ok(ir)
}

/// Extract call relationships from a function/method body
fn extract_calls_from_body(
    body: &[Stmt],
    caller_name: &str,
    line_offset: usize,
) -> Vec<CallRelation> {
    let mut calls = Vec::new();

    for (idx, stmt) in body.iter().enumerate() {
        extract_calls_from_stmt(stmt, caller_name, line_offset + idx, &mut calls);
    }

    calls
}

/// Recursively extract calls from a statement
fn extract_calls_from_stmt(
    stmt: &Stmt,
    caller_name: &str,
    line: usize,
    calls: &mut Vec<CallRelation>,
) {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            extract_calls_from_expr(&expr_stmt.value, caller_name, line, calls);
        }
        Stmt::Assign(assign) => {
            extract_calls_from_expr(&assign.value, caller_name, line, calls);
        }
        Stmt::AnnAssign(ann_assign) => {
            if let Some(ref value) = ann_assign.value {
                extract_calls_from_expr(value, caller_name, line, calls);
            }
        }
        Stmt::AugAssign(aug_assign) => {
            extract_calls_from_expr(&aug_assign.value, caller_name, line, calls);
        }
        Stmt::Return(return_stmt) => {
            if let Some(ref value) = return_stmt.value {
                extract_calls_from_expr(value, caller_name, line, calls);
            }
        }
        Stmt::If(if_stmt) => {
            for stmt in &if_stmt.body {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
            for stmt in &if_stmt.orelse {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
        }
        Stmt::While(while_stmt) => {
            for stmt in &while_stmt.body {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
        }
        Stmt::For(for_stmt) => {
            for stmt in &for_stmt.body {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
        }
        Stmt::With(with_stmt) => {
            for stmt in &with_stmt.body {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
        }
        Stmt::Try(try_stmt) => {
            for stmt in &try_stmt.body {
                extract_calls_from_stmt(stmt, caller_name, line, calls);
            }
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                for stmt in &h.body {
                    extract_calls_from_stmt(stmt, caller_name, line, calls);
                }
            }
        }
        _ => {
            // Other statement types
        }
    }
}

/// Extract calls from an expression
fn extract_calls_from_expr(
    expr: &ast::Expr,
    caller_name: &str,
    line: usize,
    calls: &mut Vec<CallRelation>,
) {
    use ast::Expr;

    match expr {
        Expr::Call(call_expr) => {
            // Extract the callee name with full qualification
            let callee_name = extract_callee_name(call_expr.func.as_ref());

            if let Some(callee) = callee_name {
                let is_method = matches!(call_expr.func.as_ref(), Expr::Attribute(_));
                calls.push(CallRelation {
                    caller: caller_name.to_string(),
                    callee,
                    line,
                    is_method_call: is_method,
                });
            }

            // Also check arguments for nested calls
            for arg in &call_expr.args {
                extract_calls_from_expr(arg, caller_name, line, calls);
            }
        }
        Expr::BinOp(binop) => {
            extract_calls_from_expr(&binop.left, caller_name, line, calls);
            extract_calls_from_expr(&binop.right, caller_name, line, calls);
        }
        Expr::UnaryOp(unary) => {
            extract_calls_from_expr(&unary.operand, caller_name, line, calls);
        }
        Expr::IfExp(if_exp) => {
            extract_calls_from_expr(&if_exp.body, caller_name, line, calls);
            extract_calls_from_expr(&if_exp.orelse, caller_name, line, calls);
        }
        Expr::ListComp(list_comp) => {
            extract_calls_from_expr(&list_comp.elt, caller_name, line, calls);
        }
        _ => {
            // Other expression types
        }
    }
}

/// Extract callee name from a call expression's func attribute
/// Handles: foo(), obj.method(), self.method(), Class.method(), a.b.c()
fn extract_callee_name(expr: &ast::Expr) -> Option<String> {
    use ast::Expr;

    match expr {
        Expr::Name(name) => {
            // Simple function call: func()
            Some(name.id.to_string())
        }
        Expr::Attribute(attr) => {
            // Method/attribute call: obj.method() or self.method()
            // Build qualified name by recursively getting the value part
            let base = extract_attribute_chain(&attr.value);
            Some(format!("{}.{}", base, attr.attr))
        }
        _ => None,
    }
}

/// Extract the base object/chain from an attribute expression
/// Examples: self -> "self", obj -> "obj", a.b -> "a.b"
fn extract_attribute_chain(expr: &ast::Expr) -> String {
    use ast::Expr;

    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Attribute(attr) => {
            let base = extract_attribute_chain(&attr.value);
            format!("{}.{}", base, attr.attr)
        }
        _ => "unknown".to_string(),
    }
}

/// Extract base class name from an inheritance expression
/// Handles: class Child(Parent), class Child(module.Parent)
fn extract_base_class_name(expr: &ast::Expr) -> Option<String> {
    use ast::Expr;

    match expr {
        Expr::Name(name) => {
            // Simple base: class Child(Parent)
            Some(name.id.to_string())
        }
        Expr::Attribute(attr) => {
            // Qualified base: class Child(module.Parent)
            let base = extract_attribute_chain(&attr.value);
            Some(format!("{}.{}", base, attr.attr))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_ir_new() {
        let ir = CodeIR::new();
        assert_eq!(ir.entity_count(), 0);
        assert_eq!(ir.relationship_count(), 0);
    }

    #[test]
    fn test_code_ir_counts() {
        let mut ir = CodeIR::new();

        ir.add_function(FunctionEntity::new("test_func", 1, 5));
        assert_eq!(ir.entity_count(), 1);

        ir.add_class(ClassEntity::new("TestClass", 10, 20));
        assert_eq!(ir.entity_count(), 2);
    }

    #[test]
    fn test_call_extraction_from_function() {
        let source = r#"
def greet(name):
    print(f"Hello, {name}")
    return name.upper()

def main():
    greet("World")
    result = greet("Alice")
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Should extract 2 functions
        assert_eq!(ir.functions.len(), 2, "Should find 2 functions");

        // Should extract calls from main to greet
        assert!(
            ir.calls.len() >= 2,
            "Should find at least 2 calls (main->greet twice)"
        );

        // Verify calls
        let greet_calls: Vec<_> = ir
            .calls
            .iter()
            .filter(|c| c.caller == "main" && c.callee == "greet")
            .collect();
        assert_eq!(
            greet_calls.len(),
            2,
            "Should find 2 calls from main to greet"
        );
    }

    #[test]
    fn test_call_extraction_from_method() {
        let source = r#"
class Calculator:
    def add(self, a, b):
        return a + b
    
    def multiply(self, a, b):
        result = self.add(a, 0)
        return result + a * b
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Should extract 1 class with 2 methods
        assert_eq!(ir.classes.len(), 1, "Should find 1 class");
        assert_eq!(ir.classes[0].methods.len(), 2, "Should find 2 methods");

        // Should extract call from multiply to add
        assert!(!ir.calls.is_empty(), "Should find at least 1 call");

        // Verify method call with proper qualification (self.add)
        let method_calls: Vec<_> = ir
            .calls
            .iter()
            .filter(|c| c.caller == "Calculator.multiply" && c.callee == "self.add")
            .collect();
        assert_eq!(
            method_calls.len(),
            1,
            "Should find call from Calculator.multiply to self.add"
        );

        // Verify it's marked as a method call
        assert!(
            method_calls[0].is_method_call,
            "Should be marked as method call"
        );
    }

    #[test]
    fn test_different_call_types() {
        let source = r#"
def standalone():
    pass

class MyClass:
    def method(self):
        standalone()           # Function call
        self.helper()          # Self method call
        other.method()         # Object method call
        MyClass.static_method() # Qualified call
    
    def helper(self):
        pass
    
    @staticmethod
    def static_method():
        pass

other = MyClass()
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Find calls from method
        let method_calls: Vec<_> = ir
            .calls
            .iter()
            .filter(|c| c.caller == "MyClass.method")
            .collect();

        // Should find 4 calls
        assert_eq!(method_calls.len(), 4, "Should find 4 calls from method");

        // Check each call type
        let call_names: Vec<_> = method_calls.iter().map(|c| c.callee.as_str()).collect();
        assert!(
            call_names.contains(&"standalone"),
            "Should find standalone() call"
        );
        assert!(
            call_names.contains(&"self.helper"),
            "Should find self.helper() call"
        );
        assert!(
            call_names.contains(&"other.method"),
            "Should find other.method() call"
        );
        assert!(
            call_names.contains(&"MyClass.static_method"),
            "Should find MyClass.static_method() call"
        );
    }

    #[test]
    fn test_import_extraction() {
        let source = r#"
import os
import sys
from pathlib import Path
from typing import List, Dict
from collections import *

def main():
    pass
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Should extract 6 import statements (os, sys, pathlib, typing, collections)
        assert_eq!(ir.imports.len(), 5, "Should find 5 import statements");

        // Check regular imports
        let os_import = ir.imports.iter().find(|i| i.from_module == "os");
        assert!(os_import.is_some(), "Should find os import");

        let sys_import = ir.imports.iter().find(|i| i.from_module == "sys");
        assert!(sys_import.is_some(), "Should find sys import");

        // Check from imports
        let pathlib_import = ir.imports.iter().find(|i| i.from_module == "pathlib");
        assert!(pathlib_import.is_some(), "Should find pathlib import");
        assert_eq!(
            pathlib_import.unwrap().imported_items,
            vec!["Path"],
            "Should import Path from pathlib"
        );

        let typing_import = ir.imports.iter().find(|i| i.from_module == "typing");
        assert!(typing_import.is_some(), "Should find typing import");
        assert_eq!(
            typing_import.unwrap().imported_items.len(),
            2,
            "Should import 2 items from typing"
        );

        // Check wildcard import
        let wildcard_import = ir.imports.iter().find(|i| i.from_module == "collections");
        assert!(
            wildcard_import.is_some(),
            "Should find collections wildcard import"
        );
        assert!(
            wildcard_import.unwrap().is_wildcard,
            "Should be marked as wildcard"
        );
    }

    #[test]
    fn test_inheritance_extraction() {
        let source = r#"
class Animal:
    def move(self):
        pass

class Dog(Animal):
    def bark(self):
        pass

class Cat(Animal):
    def meow(self):
        pass

class GermanShepherd(Dog):
    def guard(self):
        pass
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Should extract 4 classes
        assert_eq!(ir.classes.len(), 4, "Should find 4 classes");

        // Should extract 3 inheritance relationships
        assert_eq!(
            ir.inheritance.len(),
            3,
            "Should find 3 inheritance relationships"
        );

        // Check Dog inherits from Animal
        let dog_inheritance = ir
            .inheritance
            .iter()
            .find(|i| i.child == "Dog" && i.parent == "Animal");
        assert!(dog_inheritance.is_some(), "Dog should inherit from Animal");

        // Check Cat inherits from Animal
        let cat_inheritance = ir
            .inheritance
            .iter()
            .find(|i| i.child == "Cat" && i.parent == "Animal");
        assert!(cat_inheritance.is_some(), "Cat should inherit from Animal");

        // Check GermanShepherd inherits from Dog
        let gs_inheritance = ir
            .inheritance
            .iter()
            .find(|i| i.child == "GermanShepherd" && i.parent == "Dog");
        assert!(
            gs_inheritance.is_some(),
            "GermanShepherd should inherit from Dog"
        );
    }

    #[test]
    fn test_multiple_inheritance() {
        let source = r#"
class A:
    pass

class B:
    pass

class C(A, B):
    pass
"#;
        let path = Path::new("test.py");
        let config = ParserConfig::default();
        let ir = extract(source, path, &config).unwrap();

        // Should extract 3 classes
        assert_eq!(ir.classes.len(), 3, "Should find 3 classes");

        // Should extract 2 inheritance relationships (C inherits from both A and B)
        assert_eq!(
            ir.inheritance.len(),
            2,
            "Should find 2 inheritance relationships"
        );

        let c_inherits_a = ir
            .inheritance
            .iter()
            .find(|i| i.child == "C" && i.parent == "A");
        assert!(c_inherits_a.is_some(), "C should inherit from A");

        let c_inherits_b = ir
            .inheritance
            .iter()
            .find(|i| i.child == "C" && i.parent == "B");
        assert!(c_inherits_b.is_some(), "C should inherit from B");
    }
}
