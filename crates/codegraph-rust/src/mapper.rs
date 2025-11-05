//! Mapper for converting CodeIR to CodeGraph nodes and edges
//!
//! This module handles the conversion of the intermediate representation (IR)
//! into actual graph nodes and edges in the CodeGraph database.

use codegraph::{types::{Edge, EdgeType, Node, NodeType}, CodeGraph, NodeId};
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
        let mut node = Node::new(module.name.clone(), NodeType::File);
        node.add_property("path", module.path.clone());
        node.add_property("language", module.language.clone());
        node.add_property("line_count", module.line_count.to_string());

        if let Some(ref doc) = module.doc_comment {
            node.add_property("doc", doc.clone());
        }

        let id = graph
            .add_node(node)
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
        let mut node = Node::new(file_name.clone(), NodeType::File);
        node.add_property("path", file_path.display().to_string());
        node.add_property("language", "rust");

        let id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(file_name, id);
        id
    };

    // Add functions
    for func in &ir.functions {
        let mut node = Node::new(func.name.clone(), NodeType::Function);
        node.add_property("signature", func.signature.clone());
        node.add_property("visibility", func.visibility.clone());
        node.add_property("line_start", func.line_start.to_string());
        node.add_property("line_end", func.line_end.to_string());
        node.add_property("is_async", func.is_async.to_string());
        node.add_property("is_static", func.is_static.to_string());
        node.add_property("is_abstract", func.is_abstract.to_string());

        if let Some(ref doc) = func.doc_comment {
            node.add_property("doc", doc.clone());
        }
        if let Some(ref return_type) = func.return_type {
            node.add_property("return_type", return_type.clone());
        }

        let func_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link function to file or parent class
        if let Some(ref parent_class) = func.parent_class {
            // This is a method - link to class if it exists
            if let Some(&class_id) = node_map.get(parent_class) {
                let edge = Edge::new(class_id, func_id, EdgeType::Contains);
                graph
                    .add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                // Parent class not yet in map, link to file for now
                let edge = Edge::new(file_id, func_id, EdgeType::Contains);
                graph
                    .add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        } else {
            // Top-level function - link to file
            let edge = Edge::new(file_id, func_id, EdgeType::Contains);
            graph
                .add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add classes (structs/enums)
    for class in &ir.classes {
        let mut node = Node::new(class.name.clone(), NodeType::Class);
        node.add_property("visibility", class.visibility.clone());
        node.add_property("line_start", class.line_start.to_string());
        node.add_property("line_end", class.line_end.to_string());
        node.add_property("is_abstract", class.is_abstract.to_string());
        node.add_property("is_interface", class.is_interface.to_string());

        if let Some(ref doc) = class.doc_comment {
            node.add_property("doc", doc.clone());
        }

        if !class.type_parameters.is_empty() {
            node.add_property("type_parameters", class.type_parameters.join(", "));
        }

        let class_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(class.name.clone(), class_id);
        class_ids.push(class_id);

        // Link class to file
        let edge = Edge::new(file_id, class_id, EdgeType::Contains);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add traits
    for trait_entity in &ir.traits {
        let mut node = Node::new(trait_entity.name.clone(), NodeType::Trait);
        node.add_property("visibility", trait_entity.visibility.clone());
        node.add_property("line_start", trait_entity.line_start.to_string());
        node.add_property("line_end", trait_entity.line_end.to_string());

        if let Some(ref doc) = trait_entity.doc_comment {
            node.add_property("doc", doc.clone());
        }

        let trait_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(trait_entity.name.clone(), trait_id);
        trait_ids.push(trait_id);

        // Link trait to file
        let edge = Edge::new(file_id, trait_id, EdgeType::Contains);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        // Add parent trait relationships
        for parent in &trait_entity.parent_traits {
            if let Some(&parent_id) = node_map.get(parent) {
                let edge = Edge::new(trait_id, parent_id, EdgeType::Inherits);
                graph
                    .add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        }
    }

    // Add import relationships
    for import in &ir.imports {
        let mut node = Node::new(import.imported.clone(), NodeType::Import);
        node.add_property("importer", import.importer.clone());
        node.add_property("is_wildcard", import.is_wildcard.to_string());

        if let Some(ref alias) = import.alias {
            node.add_property("alias", alias.clone());
        }

        let import_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        import_ids.push(import_id);

        // Link import to file
        let edge = Edge::new(file_id, import_id, EdgeType::Imports);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add call relationships
    for call in &ir.calls {
        if let (Some(&caller_id), Some(&callee_id)) =
            (node_map.get(&call.caller), node_map.get(&call.callee))
        {
            let edge = Edge::new(caller_id, callee_id, EdgeType::Calls);
            graph
                .add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add implementation relationships
    for impl_rel in &ir.implementations {
        if let (Some(&implementor_id), Some(&trait_id)) = (
            node_map.get(&impl_rel.implementor),
            node_map.get(&impl_rel.trait_name),
        ) {
            let edge = Edge::new(implementor_id, trait_id, EdgeType::Implements);
            graph
                .add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add inheritance relationships
    for inheritance in &ir.inheritance {
        if let (Some(&child_id), Some(&parent_id)) =
            (node_map.get(&inheritance.child), node_map.get(&inheritance.parent))
        {
            let edge = Edge::new(child_id, parent_id, EdgeType::Inherits);
            graph
                .add_edge(edge)
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
}
