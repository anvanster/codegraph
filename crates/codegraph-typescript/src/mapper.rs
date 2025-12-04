//! Mapper for converting CodeIR to CodeGraph nodes and edges

use codegraph::{CodeGraph, EdgeType, NodeId, NodeType, PropertyMap};
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
            .with("line_count", module.line_count as i64);

        if let Some(ref doc) = module.doc_comment {
            props = props.with("doc", doc.clone());
        }

        let id = graph
            .add_node(NodeType::CodeFile, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(module.name.clone(), id);
        id
    } else {
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let props = PropertyMap::new()
            .with("name", file_name.clone())
            .with("path", file_path.display().to_string())
            .with("language", "typescript");

        let id = graph
            .add_node(NodeType::CodeFile, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(file_name, id);
        id
    };

    // Add functions
    for func in &ir.functions {
        let props = PropertyMap::new()
            .with("name", func.name.clone())
            .with("signature", func.signature.clone())
            .with("line_start", func.line_start as i64)
            .with("line_end", func.line_end as i64)
            .with("is_async", func.is_async);

        let func_id = graph
            .add_node(NodeType::Function, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link to file or parent class
        if let Some(ref parent_class) = func.parent_class {
            if let Some(&class_id) = node_map.get(parent_class) {
                graph
                    .add_edge(class_id, func_id, EdgeType::Contains, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                graph
                    .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        } else {
            graph
                .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add classes
    for class in &ir.classes {
        let props = PropertyMap::new()
            .with("name", class.name.clone())
            .with("line_start", class.line_start as i64)
            .with("line_end", class.line_end as i64);

        let class_id = graph
            .add_node(NodeType::Class, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(class.name.clone(), class_id);
        class_ids.push(class_id);

        graph
            .add_edge(file_id, class_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add interfaces
    for interface in &ir.traits {
        let props = PropertyMap::new()
            .with("name", interface.name.clone())
            .with("line_start", interface.line_start as i64)
            .with("line_end", interface.line_end as i64);

        let trait_id = graph
            .add_node(NodeType::Interface, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(interface.name.clone(), trait_id);
        trait_ids.push(trait_id);

        graph
            .add_edge(file_id, trait_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add import nodes and relationships
    for import in &ir.imports {
        let imported_module = &import.imported;

        // Create or get import node
        let import_id = if let Some(&existing_id) = node_map.get(imported_module) {
            existing_id
        } else {
            let props = PropertyMap::new()
                .with("name", imported_module.clone())
                .with("is_external", "true");

            let id = graph
                .add_node(NodeType::Module, props)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
            node_map.insert(imported_module.clone(), id);
            id
        };

        import_ids.push(import_id);

        // Create import edge from file to imported module
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
            let edge_props = PropertyMap::new()
                .with("call_site_line", call.call_site_line.to_string())
                .with("is_direct", call.is_direct.to_string());

            graph
                .add_edge(caller_id, callee_id, EdgeType::Calls, edge_props)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add inheritance relationships
    for inheritance in &ir.inheritance {
        if let (Some(&child_id), Some(&parent_id)) = (
            node_map.get(&inheritance.child),
            node_map.get(&inheritance.parent),
        ) {
            let edge_props = PropertyMap::new().with("order", inheritance.order.to_string());

            graph
                .add_edge(child_id, parent_id, EdgeType::Extends, edge_props)
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
                .add_edge(
                    implementor_id,
                    trait_id,
                    EdgeType::Implements,
                    PropertyMap::new(),
                )
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
        parse_time: Duration::ZERO,
        line_count: 0,
        byte_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codegraph_parser_api::{ClassEntity, FunctionEntity, TraitEntity};
    use std::path::PathBuf;

    #[test]
    fn test_ir_to_graph_empty() {
        let ir = CodeIR::new(PathBuf::from("test.ts"));
        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 0);
        assert_eq!(file_info.classes.len(), 0);
        assert_eq!(file_info.traits.len(), 0);
    }

    #[test]
    fn test_ir_to_graph_with_function() {
        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_function(FunctionEntity::new("testFunc", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_class() {
        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_class(ClassEntity::new("TestClass", 1, 10));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_interface() {
        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_trait(TraitEntity::new("ITest", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.traits.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_module() {
        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.set_module(codegraph_parser_api::ModuleEntity::new(
            "test",
            "test.ts",
            "typescript",
        ));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        // File node should be created - just verify we got a valid NodeId
        graph.get_node(file_info.file_id).unwrap();
    }

    #[test]
    fn test_ir_to_graph_async_function() {
        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        let func = FunctionEntity::new("asyncFunc", 1, 5).async_fn();
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);

        // Verify the function has is_async property
        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(
            func_node.properties.get("is_async"),
            Some(&codegraph::PropertyValue::Bool(true))
        );
    }

    #[test]
    fn test_ir_to_graph_with_imports() {
        use codegraph::{Direction, EdgeType};
        use codegraph_parser_api::ImportRelation;

        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_import(ImportRelation::new("test.ts", "react"));
        ir.add_import(
            ImportRelation::new("test.ts", "lodash")
                .with_symbols(vec!["map".to_string(), "filter".to_string()]),
        );

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();

        // Verify import nodes were created and returned
        assert_eq!(file_info.imports.len(), 2, "Should have 2 import nodes");

        // Verify import edges were created (file -> module with EdgeType::Imports)
        let neighbors = graph
            .get_neighbors(file_info.file_id, Direction::Outgoing)
            .unwrap();

        // Check that import modules are in the neighbors
        let mut import_edges_count = 0;
        for neighbor_id in &neighbors {
            let edges = graph
                .get_edges_between(file_info.file_id, *neighbor_id)
                .unwrap();
            for edge_id in edges {
                let edge = graph.get_edge(edge_id).unwrap();
                if edge.edge_type == EdgeType::Imports {
                    import_edges_count += 1;
                }
            }
        }
        assert_eq!(import_edges_count, 2, "Should have 2 import edges");
    }

    #[test]
    fn test_ir_to_graph_with_calls() {
        use codegraph::EdgeType;
        use codegraph_parser_api::CallRelation;

        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_function(FunctionEntity::new("caller", 1, 10));
        ir.add_function(FunctionEntity::new("callee", 12, 20));
        ir.add_call(CallRelation::new("caller", "callee", 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 2);

        // Find caller and callee node IDs
        let caller_id = file_info.functions[0];
        let callee_id = file_info.functions[1];

        // Verify call edge was created
        let edges = graph.get_edges_between(caller_id, callee_id).unwrap();
        assert!(
            !edges.is_empty(),
            "Should have call edge between caller and callee"
        );

        let edge = graph.get_edge(edges[0]).unwrap();
        assert_eq!(
            edge.edge_type,
            EdgeType::Calls,
            "Edge should be of type Calls"
        );
    }

    #[test]
    fn test_ir_to_graph_with_inheritance() {
        use codegraph::EdgeType;
        use codegraph_parser_api::InheritanceRelation;

        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_class(ClassEntity::new("ChildClass", 1, 20));
        ir.add_class(ClassEntity::new("ParentClass", 22, 40));
        ir.add_inheritance(InheritanceRelation::new("ChildClass", "ParentClass"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 2);

        // Find child and parent node IDs
        let child_id = file_info.classes[0];
        let parent_id = file_info.classes[1];

        // Verify extends edge was created
        let edges = graph.get_edges_between(child_id, parent_id).unwrap();
        assert!(
            !edges.is_empty(),
            "Should have extends edge between child and parent"
        );

        let edge = graph.get_edge(edges[0]).unwrap();
        assert_eq!(
            edge.edge_type,
            EdgeType::Extends,
            "Edge should be of type Extends"
        );
    }

    #[test]
    fn test_ir_to_graph_with_implementation() {
        use codegraph::EdgeType;
        use codegraph_parser_api::ImplementationRelation;

        let mut ir = CodeIR::new(PathBuf::from("test.ts"));
        ir.add_class(ClassEntity::new("MyClass", 1, 20));
        ir.add_trait(TraitEntity::new("IMyInterface", 22, 30));
        ir.add_implementation(ImplementationRelation::new("MyClass", "IMyInterface"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.ts").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
        assert_eq!(file_info.traits.len(), 1);

        // Find class and interface node IDs
        let class_id = file_info.classes[0];
        let interface_id = file_info.traits[0];

        // Verify implements edge was created
        let edges = graph.get_edges_between(class_id, interface_id).unwrap();
        assert!(
            !edges.is_empty(),
            "Should have implements edge between class and interface"
        );

        let edge = graph.get_edge(edges[0]).unwrap();
        assert_eq!(
            edge.edge_type,
            EdgeType::Implements,
            "Edge should be of type Implements"
        );
    }
}
