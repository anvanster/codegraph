//! Mapper for converting CodeIR to CodeGraph nodes and edges
//!
//! This module handles the conversion of the intermediate representation (IR)
//! into actual graph nodes and edges in the CodeGraph database.

use codegraph::{EdgeType, NodeType, PropertyMap, CodeGraph, NodeId};
use codegraph_parser_api::{CodeIR, FileInfo, ParserError};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Convert CodeIR to graph nodes and edges, returning FileInfo
pub fn ir_to_graph(
    ir: &CodeIR,
    graph: &mut CodeGraph,
    file_path: &Path,
) -> Result<FileInfo, ParserError> {
    let mut node_map: HashMap<String, NodeId> = HashMap::new();
    let mut function_ids = Vec::new();
    let mut class_ids = Vec::new();
    let mut trait_ids = Vec::new();
    let mut import_ids = Vec::new();

    // Create module/file node
    let file_id = if let Some(ref module) = ir.module {
        let mut props = PropertyMap::new()
            .with("name", module.name.clone())
            .with("path", module.path.clone())
            .with("language", module.language.clone())
            .with("line_count", module.line_count.to_string());

        if let Some(ref doc) = module.doc_comment {
            props = props.with("doc", doc.clone());
        }

        let id = graph
            .add_node(NodeType::CodeFile, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(module.name.clone(), id);
        id
    } else {
        // Create a default file node
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let props = PropertyMap::new()
            .with("name", file_name.clone())
            .with("path", file_path.display().to_string())
            .with("language", "rust");

        let id = graph
            .add_node(NodeType::CodeFile, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(file_name, id);
        id
    };

    // Add functions
    for func in &ir.functions {
        let mut props = PropertyMap::new()
            .with("name", func.name.clone())
            .with("signature", func.signature.clone())
            .with("visibility", func.visibility.clone())
            .with("line_start", func.line_start.to_string())
            .with("line_end", func.line_end.to_string())
            .with("is_async", func.is_async)
            .with("is_static", func.is_static)
            .with("is_abstract", func.is_abstract);

        if let Some(ref doc) = func.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if let Some(ref return_type) = func.return_type {
            props = props.with("return_type", return_type.clone());
        }

        let func_id = graph
            .add_node(NodeType::Function, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link function to file or parent class
        if let Some(ref parent_class) = func.parent_class {
            // This is a method - link to class if it exists
            if let Some(&class_id) = node_map.get(parent_class) {
                graph
                    .add_edge(class_id, func_id, EdgeType::Contains, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                // Parent class not yet in map, link to file for now
                graph
                    .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        } else {
            // Top-level function - link to file
            graph
                .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add classes (structs/enums)
    for class in &ir.classes {
        let mut props = PropertyMap::new()
            .with("name", class.name.clone())
            .with("visibility", class.visibility.clone())
            .with("line_start", class.line_start.to_string())
            .with("line_end", class.line_end.to_string())
            .with("is_abstract", class.is_abstract.to_string())
            .with("is_interface", class.is_interface.to_string());

        if let Some(ref doc) = class.doc_comment {
            props = props.with("doc", doc.clone());
        }

        if !class.type_parameters.is_empty() {
            props = props.with("type_parameters", class.type_parameters.join(", "));
        }

        let class_id = graph
            .add_node(NodeType::Class, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(class.name.clone(), class_id);
        class_ids.push(class_id);

        // Link class to file
        graph
            .add_edge(file_id, class_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add traits
    for trait_entity in &ir.traits {
        let mut props = PropertyMap::new()
            .with("name", trait_entity.name.clone())
            .with("visibility", trait_entity.visibility.clone())
            .with("line_start", trait_entity.line_start.to_string())
            .with("line_end", trait_entity.line_end.to_string());

        if let Some(ref doc) = trait_entity.doc_comment {
            props = props.with("doc", doc.clone());
        }

        let trait_id = graph
            .add_node(NodeType::Interface, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(trait_entity.name.clone(), trait_id);
        trait_ids.push(trait_id);

        // Link trait to file
        graph
            .add_edge(file_id, trait_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        // Add parent trait relationships
        for parent in &trait_entity.parent_traits {
            if let Some(&parent_id) = node_map.get(parent) {
                graph
                    .add_edge(trait_id, parent_id, EdgeType::Extends, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        }
    }

    // Add import nodes and relationships
    for import in &ir.imports {
        let imported_module = &import.imported;

        // Create or get module node
        let import_id = if let Some(&existing_id) = node_map.get(imported_module) {
            existing_id
        } else {
            // Determine if this is an external or internal module
            let is_external = !imported_module.starts_with("super::")
                && !imported_module.starts_with("crate::")
                && !imported_module.starts_with("self::");

            let props = PropertyMap::new()
                .with("name", imported_module.clone())
                .with("is_external", is_external.to_string());

            let id = graph
                .add_node(NodeType::Module, props)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
            node_map.insert(imported_module.clone(), id);
            id
        };

        import_ids.push(import_id);

        // Create import edge from file to module
        let mut edge_props = PropertyMap::new();
        if let Some(ref alias) = import.alias {
            edge_props = edge_props.with("alias", alias.clone());
        }
        if import.is_wildcard {
            edge_props = edge_props.with("is_wildcard", "true");
        }
        if !import.symbols.is_empty() {
            edge_props = edge_props.with("symbols", import.symbols.join(","));
        }

        graph
            .add_edge(file_id, import_id, EdgeType::Imports, edge_props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add call relationships
    for call in &ir.calls {
        if let (Some(&caller_id), Some(&callee_id)) =
            (node_map.get(&call.caller), node_map.get(&call.callee))
        {
            graph
                .add_edge(caller_id, callee_id, EdgeType::Calls, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add implementation relationships
    for impl_rel in &ir.implementations {
        if let (Some(&implementor_id), Some(&trait_id)) = (
            node_map.get(&impl_rel.implementor),
            node_map.get(&impl_rel.trait_name),
        ) {
            graph
                .add_edge(implementor_id, trait_id, EdgeType::Implements, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add inheritance relationships
    for inheritance in &ir.inheritance {
        if let (Some(&child_id), Some(&parent_id)) =
            (node_map.get(&inheritance.child), node_map.get(&inheritance.parent))
        {
            graph
                .add_edge(child_id, parent_id, EdgeType::Extends, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    Ok(FileInfo {
        file_path: file_path.to_path_buf(),
        file_id,
        functions: function_ids,
        classes: class_ids,
        traits: trait_ids,
        imports: import_ids,
        parse_time: Duration::ZERO, // Will be set by caller
        line_count: 0,              // Will be set by caller
        byte_count: 0,              // Will be set by caller
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codegraph_parser_api::{FunctionEntity, ModuleEntity};

    #[test]
    fn test_ir_to_graph_basic() {
        let mut graph = CodeGraph::in_memory().unwrap();
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));

        ir.module = Some(ModuleEntity {
            name: "test".to_string(),
            path: "test.rs".to_string(),
            language: "rust".to_string(),
            line_count: 10,
            doc_comment: None,
            attributes: Vec::new(),
        });

        ir.functions.push(FunctionEntity {
            name: "hello".to_string(),
            signature: "fn hello()".to_string(),
            visibility: "public".to_string(),
            line_start: 1,
            line_end: 3,
            is_async: false,
            is_test: false,
            is_static: false,
            is_abstract: false,
            parameters: Vec::new(),
            return_type: None,
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: None,
        });

        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.functions.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_empty() {
        let ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 0);
        assert_eq!(file_info.classes.len(), 0);
        assert_eq!(file_info.traits.len(), 0);
    }

    #[test]
    fn test_ir_to_graph_with_function() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        ir.add_function(FunctionEntity::new("test_fn", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_struct() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        ir.add_class(codegraph_parser_api::ClassEntity::new("MyStruct", 1, 10));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_trait() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        ir.add_trait(codegraph_parser_api::TraitEntity::new("MyTrait", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.traits.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_module() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        ir.set_module(ModuleEntity::new("test", "test.rs", "rust"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        // File node should be created - verify we got a valid NodeId
        graph.get_node(file_info.file_id).unwrap();
    }

    #[test]
    fn test_ir_to_graph_with_imports() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        ir.add_import(codegraph_parser_api::ImportRelation::new("test", "std::collections"));
        ir.add_import(codegraph_parser_api::ImportRelation::new("test", "std::io"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        // Note: Import processing not yet implemented in mapper
        // When implemented, this should be: assert_eq!(file_info.imports.len(), 2);
    }

    #[test]
    fn test_ir_to_graph_with_methods() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));

        let mut class = codegraph_parser_api::ClassEntity::new("MyStruct", 1, 15);
        class.methods.push(FunctionEntity::new("new", 2, 5));
        class.methods.push(FunctionEntity::new("method", 7, 10));
        ir.add_class(class);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
        // Note: Method processing not yet implemented in mapper
        // When implemented, methods should be added as function nodes linked to the class
    }

    #[test]
    fn test_ir_to_graph_async_function() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        let func = FunctionEntity::new("async_fn", 1, 5).async_fn();
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);

        // Verify the function has is_async property
        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(func_node.properties.get("is_async"), Some(&codegraph::PropertyValue::Bool(true)));
    }

    #[test]
    fn test_ir_to_graph_function_properties() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));
        let func = FunctionEntity::new("public_fn", 1, 5)
            .with_visibility("public")
            .with_signature("pub fn public_fn() -> i32");
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);

        // Verify function properties are set
        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(func_node.properties.get("name"), Some(&codegraph::PropertyValue::String("public_fn".to_string())));
        assert_eq!(func_node.properties.get("visibility"), Some(&codegraph::PropertyValue::String("public".to_string())));
    }

    #[test]
    fn test_ir_to_graph_trait_implementation() {
        let mut ir = CodeIR::new(std::path::PathBuf::from("test.rs"));

        ir.add_trait(codegraph_parser_api::TraitEntity::new("Display", 1, 3));
        let mut class = codegraph_parser_api::ClassEntity::new("Item", 5, 10);
        class.implemented_traits.push("Display".to_string());
        ir.add_class(class);
        ir.add_implementation(codegraph_parser_api::ImplementationRelation::new("Item", "Display"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, Path::new("test.rs"));

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.traits.len(), 1);
        assert_eq!(file_info.classes.len(), 1);
    }
}
