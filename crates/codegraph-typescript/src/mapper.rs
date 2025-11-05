//! Mapper for converting CodeIR to CodeGraph nodes and edges

use codegraph::{
    types::{Edge, EdgeType, Node, NodeType},
    CodeGraph, NodeId,
};
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
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let mut node = Node::new(file_name.clone(), NodeType::File);
        node.add_property("path", file_path.display().to_string());
        node.add_property("language", "typescript");

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
        node.add_property("line_start", func.line_start.to_string());
        node.add_property("line_end", func.line_end.to_string());
        node.add_property("is_async", func.is_async.to_string());

        let func_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link to file or parent class
        if let Some(ref parent_class) = func.parent_class {
            if let Some(&class_id) = node_map.get(parent_class) {
                let edge = Edge::new(class_id, func_id, EdgeType::Contains);
                graph
                    .add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                let edge = Edge::new(file_id, func_id, EdgeType::Contains);
                graph
                    .add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        } else {
            let edge = Edge::new(file_id, func_id, EdgeType::Contains);
            graph
                .add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add classes
    for class in &ir.classes {
        let mut node = Node::new(class.name.clone(), NodeType::Class);
        node.add_property("line_start", class.line_start.to_string());
        node.add_property("line_end", class.line_end.to_string());

        let class_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(class.name.clone(), class_id);
        class_ids.push(class_id);

        let edge = Edge::new(file_id, class_id, EdgeType::Contains);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add interfaces (as traits)
    for interface in &ir.traits {
        let mut node = Node::new(interface.name.clone(), NodeType::Trait);
        node.add_property("line_start", interface.line_start.to_string());
        node.add_property("line_end", interface.line_end.to_string());

        let trait_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(interface.name.clone(), trait_id);
        trait_ids.push(trait_id);

        let edge = Edge::new(file_id, trait_id, EdgeType::Contains);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add imports
    for import in &ir.imports {
        let mut node = Node::new(import.imported.clone(), NodeType::Import);
        node.add_property("importer", import.importer.clone());

        let import_id = graph
            .add_node(node)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        import_ids.push(import_id);

        let edge = Edge::new(file_id, import_id, EdgeType::Imports);
        graph
            .add_edge(edge)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    Ok(FileInfo {
        file_path: file_path.to_path_buf(),
        file_id,
        functions: function_ids,
        classes: class_ids,
        traits: trait_ids,
        imports: import_ids,
        parse_time: Duration::ZERO,
        line_count: 0,
        byte_count: 0,
    })
}
