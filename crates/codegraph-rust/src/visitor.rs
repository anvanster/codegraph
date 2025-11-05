//! AST visitor for extracting Rust entities
//!
//! This module implements a syn visitor that walks the Rust AST and extracts
//! functions, structs, enums, traits, and their relationships.

use codegraph_parser_api::{
    CallRelation, ClassEntity, Field, FunctionEntity, ImplementationRelation, ImportRelation,
    InheritanceRelation, Parameter, ParserConfig, TraitEntity,
};
use syn::visit::Visit;
use syn::{
    Attribute, FnArg, GenericParam, Ident, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod,
    ItemStruct, ItemTrait, ItemUse, Pat, PathArguments, ReturnType, Signature, TraitItem, Type,
    Visibility,
};

/// Visitor that extracts entities and relationships from Rust AST
pub struct RustVisitor {
    pub config: ParserConfig,
    pub functions: Vec<FunctionEntity>,
    pub classes: Vec<ClassEntity>,
    pub traits: Vec<TraitEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,
    pub implementations: Vec<ImplementationRelation>,
    pub inheritance: Vec<InheritanceRelation>,
    current_class: Option<String>,
}

impl RustVisitor {
    pub fn new(config: ParserConfig) -> Self {
        Self {
            config,
            functions: Vec::new(),
            classes: Vec::new(),
            traits: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            implementations: Vec::new(),
            inheritance: Vec::new(),
            current_class: None,
        }
    }

    /// Extract visibility string from syn::Visibility
    fn extract_visibility(vis: &Visibility) -> String {
        match vis {
            Visibility::Public(_) => "public".to_string(),
            Visibility::Restricted(r) => {
                if r.path.is_ident("crate") {
                    "internal".to_string()
                } else if r.path.is_ident("super") {
                    "protected".to_string()
                } else {
                    "restricted".to_string()
                }
            }
            Visibility::Inherited => "private".to_string(),
        }
    }

    /// Extract documentation from attributes
    fn extract_doc(attrs: &[Attribute]) -> Option<String> {
        let mut docs = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("doc") {
                if let Ok(meta) = attr.meta.require_name_value() {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            docs.push(lit_str.value().trim().to_string());
                        }
                    }
                }
            }
        }
        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n"))
        }
    }

    /// Extract parameters from function signature
    fn extract_parameters(sig: &Signature) -> Vec<Parameter> {
        sig.inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Receiver(_) => Some(Parameter {
                    name: "self".to_string(),
                    type_annotation: Some("Self".to_string()),
                    default_value: None,
                    is_variadic: false,
                }),
                FnArg::Typed(pat_type) => {
                    let name = if let Pat::Ident(ident) = &*pat_type.pat {
                        ident.ident.to_string()
                    } else {
                        "unknown".to_string()
                    };

                    let type_annotation = quote::quote!(#pat_type.ty).to_string();

                    Some(Parameter {
                        name,
                        type_annotation: Some(type_annotation),
                        default_value: None,
                        is_variadic: false,
                    })
                }
            })
            .collect()
    }

    /// Extract return type from signature
    fn extract_return_type(sig: &Signature) -> Option<String> {
        match &sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
        }
    }

    /// Check if function is async
    fn is_async(sig: &Signature) -> bool {
        sig.asyncness.is_some()
    }

    /// Check if function is a test
    fn is_test(attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| attr.path().is_ident("test"))
    }

    /// Extract generic type parameters
    fn extract_type_parameters(generics: &syn::Generics) -> Vec<String> {
        generics
            .params
            .iter()
            .filter_map(|param| {
                if let GenericParam::Type(type_param) = param {
                    Some(type_param.ident.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<'ast> Visit<'ast> for RustVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let vis = Self::extract_visibility(&node.vis);
        let name = node.sig.ident.to_string();
        let signature = quote::quote!(#node.sig).to_string();
        let doc = Self::extract_doc(&node.attrs);
        let parameters = Self::extract_parameters(&node.sig);
        let return_type = Self::extract_return_type(&node.sig);
        let is_async = Self::is_async(&node.sig);
        let is_test = Self::is_test(&node.attrs);

        // Skip private functions if configured
        if self.config.skip_private && vis == "private" {
            return;
        }

        // Skip test functions if configured
        if self.config.skip_tests && is_test {
            return;
        }

        let func = FunctionEntity {
            name: name.clone(),
            signature,
            visibility: vis,
            line_start: 0, // syn doesn't provide line numbers easily
            line_end: 0,
            is_async,
            is_test,
            is_static: false,
            is_abstract: false,
            parameters,
            return_type,
            doc_comment: doc,
            attributes: Vec::new(),
            parent_class: self.current_class.clone(),
        };

        self.functions.push(func);

        // Continue visiting the function body for calls
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let vis = Self::extract_visibility(&node.vis);
        let name = node.ident.to_string();
        let doc = Self::extract_doc(&node.attrs);

        // Skip private structs if configured
        if self.config.skip_private && vis == "private" {
            return;
        }

        // Extract fields
        let fields = node
            .fields
            .iter()
            .map(|f| {
                let field_name = f
                    .ident
                    .as_ref()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unnamed".to_string());
                let field_vis = Self::extract_visibility(&f.vis);
                let type_annotation = quote::quote!(#f.ty).to_string();

                Field {
                    name: field_name,
                    type_annotation: Some(type_annotation),
                    visibility: field_vis,
                    is_static: false,
                    is_constant: false,
                    default_value: None,
                }
            })
            .collect();

        let type_parameters = Self::extract_type_parameters(&node.generics);

        let class = ClassEntity {
            name: name.clone(),
            visibility: vis,
            line_start: 0,
            line_end: 0,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields,
            doc_comment: doc,
            attributes: Vec::new(),
            type_parameters,
        };

        self.classes.push(class);

        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        let vis = Self::extract_visibility(&node.vis);
        let name = node.ident.to_string();
        let doc = Self::extract_doc(&node.attrs);

        // Skip private enums if configured
        if self.config.skip_private && vis == "private" {
            return;
        }

        // Treat enums as classes in the graph
        let type_parameters = Self::extract_type_parameters(&node.generics);

        let class = ClassEntity {
            name: name.clone(),
            visibility: vis,
            line_start: 0,
            line_end: 0,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(), // Could extract variants as fields
            doc_comment: doc,
            attributes: vec!["enum".to_string()],
            type_parameters,
        };

        self.classes.push(class);

        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let vis = Self::extract_visibility(&node.vis);
        let name = node.ident.to_string();
        let doc = Self::extract_doc(&node.attrs);

        // Skip private traits if configured
        if self.config.skip_private && vis == "private" {
            return;
        }

        // Extract trait methods
        let mut required_methods = Vec::new();
        for item in &node.items {
            if let TraitItem::Fn(method) = item {
                let method_name = method.sig.ident.to_string();
                let signature = quote::quote!(#method.sig).to_string();
                let parameters = Self::extract_parameters(&method.sig);
                let return_type = Self::extract_return_type(&method.sig);

                let func = FunctionEntity {
                    name: method_name,
                    signature,
                    visibility: "public".to_string(),
                    line_start: 0,
                    line_end: 0,
                    is_async: Self::is_async(&method.sig),
                    is_test: false,
                    is_static: false,
                    is_abstract: true,
                    parameters,
                    return_type,
                    doc_comment: Self::extract_doc(&method.attrs),
                    attributes: Vec::new(),
                    parent_class: Some(name.clone()),
                };

                required_methods.push(func);
            }
        }

        // Extract parent traits (trait inheritance)
        let parent_traits = node
            .supertraits
            .iter()
            .filter_map(|bound| {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    trait_bound
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        let trait_entity = TraitEntity {
            name: name.clone(),
            visibility: vis,
            line_start: 0,
            line_end: 0,
            required_methods,
            parent_traits,
            doc_comment: doc,
            attributes: Vec::new(),
        };

        self.traits.push(trait_entity);

        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Extract the implementing type
        let implementor = if let Type::Path(type_path) = &*node.self_ty {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        // Check if this is a trait implementation
        if let Some((_, trait_path, _)) = &node.trait_ {
            let trait_name = trait_path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let impl_rel = ImplementationRelation {
                implementor: implementor.clone(),
                trait_name,
            };

            self.implementations.push(impl_rel);
        }

        // Set current class context for methods
        let previous_class = self.current_class.clone();
        self.current_class = Some(implementor.clone());

        // Extract methods from impl block
        for item in &node.items {
            if let ImplItem::Fn(method) = item {
                let vis = Self::extract_visibility(&method.vis);
                let name = method.sig.ident.to_string();
                let signature = quote::quote!(#method.sig).to_string();
                let doc = Self::extract_doc(&method.attrs);
                let parameters = Self::extract_parameters(&method.sig);
                let return_type = Self::extract_return_type(&method.sig);
                let is_async = Self::is_async(&method.sig);

                // Check if it's a static method (no self parameter)
                let is_static = !method
                    .sig
                    .inputs
                    .iter()
                    .any(|arg| matches!(arg, FnArg::Receiver(_)));

                let func = FunctionEntity {
                    name,
                    signature,
                    visibility: vis,
                    line_start: 0,
                    line_end: 0,
                    is_async,
                    is_test: false,
                    is_static,
                    is_abstract: false,
                    parameters,
                    return_type,
                    doc_comment: doc,
                    attributes: Vec::new(),
                    parent_class: Some(implementor.clone()),
                };

                self.functions.push(func);
            }
        }

        syn::visit::visit_item_impl(self, node);

        // Restore previous class context
        self.current_class = previous_class;
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        // Extract use statements as imports
        let import_path = quote::quote!(#node.tree).to_string();

        let import = ImportRelation {
            importer: "current_module".to_string(), // Would need context to know the module
            imported: import_path,
            symbols: Vec::new(),
            is_wildcard: false,
            alias: None,
        };

        self.imports.push(import);

        syn::visit::visit_item_use(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_function() {
        let source = r#"
fn hello() {
    println!("Hello");
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "hello");
    }

    #[test]
    fn test_visitor_struct() {
        let source = r#"
pub struct MyStruct {
    pub field1: String,
    field2: i32,
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "MyStruct");
        assert_eq!(visitor.classes[0].fields.len(), 2);
    }

    #[test]
    fn test_visitor_trait() {
        let source = r#"
pub trait MyTrait {
    fn method(&self);
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.traits.len(), 1);
        assert_eq!(visitor.traits[0].name, "MyTrait");
        assert_eq!(visitor.traits[0].required_methods.len(), 1);
    }

    #[test]
    fn test_visitor_enum() {
        let source = r#"
pub enum Status {
    Active,
    Inactive,
    Pending,
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "Status");
    }

    #[test]
    fn test_visitor_impl_block() {
        let source = r#"
struct MyStruct;

impl MyStruct {
    fn new() -> Self {
        MyStruct
    }

    fn method(&self) {}
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.classes.len(), 1);
        // Methods should be extracted
        let struct_with_methods = &visitor.classes[0];
        assert!(struct_with_methods.methods.len() > 0 || visitor.functions.len() > 0);
    }

    #[test]
    fn test_visitor_async_function() {
        let source = r#"
async fn fetch() -> String {
    "data".to_string()
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0].is_async);
    }

    #[test]
    fn test_visitor_use_statements() {
        let source = r#"
use std::collections::HashMap;
use std::io::{self, Read};
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert!(visitor.imports.len() >= 1);
    }

    #[test]
    fn test_visitor_generic_struct() {
        let source = r#"
pub struct Wrapper<T> {
    value: T,
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.classes.len(), 1);
        assert_eq!(visitor.classes[0].name, "Wrapper");
        assert!(visitor.classes[0].type_parameters.len() > 0);
    }

    #[test]
    fn test_visitor_trait_impl() {
        let source = r#"
pub trait Display {
    fn display(&self);
}

pub struct Item;

impl Display for Item {
    fn display(&self) {}
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.traits.len(), 1);
        assert_eq!(visitor.classes.len(), 1);
        assert!(visitor.implementations.len() > 0 || visitor.classes[0].implemented_traits.len() > 0);
    }

    #[test]
    fn test_visitor_function_with_attributes() {
        let source = r#"
#[test]
#[ignore]
fn test_something() {}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0].is_test);
        // Note: Full attribute extraction not yet implemented
        // Attributes are detected for is_test flag but not stored in attributes vector
    }

    #[test]
    fn test_visitor_visibility_modifiers() {
        let source = r#"
pub fn public_fn() {}
fn private_fn() {}
pub(crate) fn crate_fn() {}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.functions.len(), 3);
        // Check that visibility is captured
        let public_count = visitor.functions.iter()
            .filter(|f| f.visibility.contains("public"))
            .count();
        assert!(public_count >= 1);
    }

    #[test]
    fn test_visitor_multiple_items() {
        let source = r#"
use std::fmt;

pub trait Trait1 {
    fn method1(&self);
}

pub struct Struct1 {
    field: i32,
}

pub enum Enum1 {
    Variant1,
    Variant2,
}

pub fn function1() {}

impl Struct1 {
    fn new() -> Self {
        Struct1 { field: 0 }
    }
}
"#;
        let syntax_tree = syn::parse_file(source).unwrap();
        let mut visitor = RustVisitor::new(ParserConfig::default());
        visitor.visit_file(&syntax_tree);

        assert_eq!(visitor.traits.len(), 1);
        assert!(visitor.classes.len() >= 2); // Struct1 and Enum1
        assert!(visitor.functions.len() >= 1);
        assert!(visitor.imports.len() >= 1);
    }
}
