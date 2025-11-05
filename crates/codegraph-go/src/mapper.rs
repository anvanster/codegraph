//! Mapper for converting CodeIR to CodeGraph nodes and edges

use codegraph::{types::{Edge, EdgeType, Node, NodeType}, CodeGraph, NodeId};
use codegraph_parser_api::{CodeIR, FileInfo, ParserError};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub fn ir_to_graph(ir: &CodeIR, graph: &mut CodeGraph, file_path: &Path) -> Result<FileInfo, ParserError> {
    let mut node_map: HashMap<String, NodeId> = HashMap::new();
    let mut function_ids = Vec::new();
    let mut class_ids = Vec::new();
    let mut trait_ids = Vec::new();
    let mut import_ids = Vec::new();

    let file_id = if let Some(ref module) = ir.module {
        let mut node = Node::new(module.name.clone(), NodeType::File);
        node.add_property("path", module.path.clone());
        node.add_property("language", "go");
        let id = graph.add_node(node).map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(module.name.clone(), id);
        id
    } else {
        let name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        let mut node = Node::new(name.clone(), NodeType::File);
        node.add_property("path", file_path.display().to_string());
        let id = graph.add_node(node).map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(name, id);
        id
    };

    for func in &ir.functions {
        let mut node = Node::new(func.name.clone(), NodeType::Function);
        let func_id = graph.add_node(node).map_err(|e| ParserError::GraphError(e.to_string()))?;
        function_ids.push(func_id);
        let edge = Edge::new(file_id, func_id, EdgeType::Contains);
        graph.add_edge(edge).map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    for class in &ir.classes {
        let mut node = Node::new(class.name.clone(), NodeType::Class);
        let class_id = graph.add_node(node).map_err(|e| ParserError::GraphError(e.to_string()))?;
        class_ids.push(class_id);
        let edge = Edge::new(file_id, class_id, EdgeType::Contains);
        graph.add_edge(edge).map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    for interface in &ir.traits {
        let mut node = Node::new(interface.name.clone(), NodeType::Trait);
        let trait_id = graph.add_node(node).map_err(|e| ParserError::GraphError(e.to_string()))?;
        trait_ids.push(trait_id);
        let edge = Edge::new(file_id, trait_id, EdgeType::Contains);
        graph.add_edge(edge).map_err(|e| ParserError::GraphError(e.to_string()))?;
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
