//! Mapper for converting CodeIR to CodeGraph nodes and edges

use codegraph::{CodeGraph, EdgeType, NodeId, NodeType, PropertyMap};
use codegraph_parser_api::{CodeIR, FileInfo, ParserError};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub fn ir_to_graph(
    ir: &CodeIR,
    graph: &mut CodeGraph,
    file_path: &Path,
) -> Result<FileInfo, ParserError> {
    let mut node_map: HashMap<String, NodeId> = HashMap::new();
    let mut function_ids = Vec::new();
    let mut class_ids = Vec::new();
    let trait_ids = Vec::new();
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
        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let props = PropertyMap::new()
            .with("name", name.clone())
            .with("path", file_path.display().to_string())
            .with("language", "c");

        let id = graph
            .add_node(NodeType::CodeFile, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(name, id);
        id
    };

    // Add functions
    for func in &ir.functions {
        let mut props = PropertyMap::new()
            .with("name", func.name.clone())
            .with("signature", func.signature.clone())
            .with("visibility", func.visibility.clone())
            .with("line_start", func.line_start as i64)
            .with("line_end", func.line_end as i64)
            .with("is_async", func.is_async)
            .with("is_static", func.is_static);

        if let Some(ref doc) = func.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if let Some(ref return_type) = func.return_type {
            props = props.with("return_type", return_type.clone());
        }

        // Add complexity metrics if available
        if let Some(ref complexity) = func.complexity {
            props = props
                .with(
                    "cyclomatic_complexity",
                    complexity.cyclomatic_complexity as i64,
                )
                .with("complexity_grade", complexity.grade().to_string())
                .with("branches", complexity.branches as i64)
                .with("loops", complexity.loops as i64)
                .with("logical_operators", complexity.logical_operators as i64)
                .with("max_nesting_depth", complexity.max_nesting_depth as i64);
        }

        let func_id = graph
            .add_node(NodeType::Function, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link function to file
        graph
            .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add classes/structs/unions/enums
    for class in &ir.classes {
        let mut props = PropertyMap::new()
            .with("name", class.name.clone())
            .with("visibility", class.visibility.clone())
            .with("line_start", class.line_start as i64)
            .with("line_end", class.line_end as i64)
            .with("is_abstract", class.is_abstract);

        if let Some(ref doc) = class.doc_comment {
            props = props.with("doc", doc.clone());
        }

        // Add type attribute (struct, union, enum)
        if !class.attributes.is_empty() {
            props = props.with("type_kind", class.attributes[0].clone());
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

        // Add fields as properties or child nodes (simplified: store as metadata)
        for (idx, field) in class.fields.iter().enumerate() {
            let field_props = PropertyMap::new()
                .with("name", field.name.clone())
                .with(
                    "type_annotation",
                    field.type_annotation.clone().unwrap_or_default(),
                )
                .with("visibility", field.visibility.clone())
                .with("is_static", field.is_static)
                .with("is_constant", field.is_constant)
                .with("field_index", idx as i64);

            let field_id = graph
                .add_node(NodeType::Variable, field_props)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            // Link field to class
            graph
                .add_edge(class_id, field_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add import nodes and relationships
    for import in &ir.imports {
        let imported_module = &import.imported;

        // Create or get import node
        let import_id = if let Some(&existing_id) = node_map.get(imported_module) {
            existing_id
        } else {
            let mut props = PropertyMap::new()
                .with("name", imported_module.clone())
                .with("is_external", "true");

            // Mark system includes
            if import.alias.as_deref() == Some("system") {
                props = props.with("is_system", "true");
            }

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

        graph
            .add_edge(file_id, import_id, EdgeType::Imports, edge_props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add call relationships
    // Track unresolved calls per caller for cross-file resolution
    let mut unresolved_calls: HashMap<String, Vec<String>> = HashMap::new();

    for call in &ir.calls {
        if let Some(&caller_id) = node_map.get(&call.caller) {
            if let Some(&callee_id) = node_map.get(&call.callee) {
                // Both caller and callee are in this file - create direct edge
                let edge_props = PropertyMap::new()
                    .with("call_site_line", call.call_site_line as i64)
                    .with("is_direct", call.is_direct);

                graph
                    .add_edge(caller_id, callee_id, EdgeType::Calls, edge_props)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                // Callee not found in this file - store for cross-file resolution
                unresolved_calls
                    .entry(call.caller.clone())
                    .or_default()
                    .push(call.callee.clone());
            }
        }
    }

    // Store unresolved calls on caller nodes for post-processing
    for (caller_name, callees) in unresolved_calls {
        if let Some(&caller_id) = node_map.get(&caller_name) {
            if let Ok(node) = graph.get_node(caller_id) {
                let existing = node.properties.get_string("unresolved_calls").unwrap_or("");
                let mut all_callees: Vec<&str> = if existing.is_empty() {
                    Vec::new()
                } else {
                    existing.split(',').collect()
                };
                for callee in &callees {
                    if !all_callees.contains(&callee.as_str()) {
                        all_callees.push(callee);
                    }
                }
                let new_props = node
                    .properties
                    .clone()
                    .with("unresolved_calls", all_callees.join(","));
                let _ = graph.update_node_properties(caller_id, new_props);
            }
        }
    }

    // Count source lines
    let line_count = if let Some(ref module) = ir.module {
        module.line_count
    } else {
        0
    };

    Ok(FileInfo {
        file_path: file_path.to_path_buf(),
        file_id,
        functions: function_ids,
        classes: class_ids,
        traits: trait_ids,
        imports: import_ids,
        parse_time: Duration::ZERO,
        line_count,
        byte_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codegraph_parser_api::{ClassEntity, FunctionEntity, ImportRelation};
    use std::path::PathBuf;

    #[test]
    fn test_ir_to_graph_empty() {
        let ir = CodeIR::new(PathBuf::from("test.c"));
        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 0);
        assert_eq!(file_info.classes.len(), 0);
        assert_eq!(file_info.traits.len(), 0);
    }

    #[test]
    fn test_ir_to_graph_with_function() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));
        ir.add_function(FunctionEntity::new("main", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_struct() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));
        ir.add_class(ClassEntity::new("Point", 1, 10));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_module() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));
        ir.set_module(codegraph_parser_api::ModuleEntity::new(
            "main", "test.c", "c",
        ));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        // File node should be created
        graph.get_node(file_info.file_id).unwrap();
    }

    #[test]
    fn test_ir_to_graph_with_imports() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));
        ir.add_import(ImportRelation::new("main", "stdio.h"));
        ir.add_import(ImportRelation::new("main", "stdlib.h"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.imports.len(), 2);
    }

    #[test]
    fn test_ir_to_graph_with_fields() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));

        let mut class = ClassEntity::new("Point", 1, 10);
        class.fields.push(codegraph_parser_api::Field {
            name: "x".to_string(),
            type_annotation: Some("int".to_string()),
            visibility: "public".to_string(),
            is_static: false,
            is_constant: false,
            default_value: None,
        });
        class.fields.push(codegraph_parser_api::Field {
            name: "y".to_string(),
            type_annotation: Some("int".to_string()),
            visibility: "public".to_string(),
            is_static: false,
            is_constant: false,
            default_value: None,
        });
        ir.add_class(class);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_function_properties() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));
        let func = FunctionEntity::new("helper", 1, 5)
            .with_visibility("private")
            .with_signature("static int helper(void)");
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);

        // Verify function properties
        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(
            func_node.properties.get("name"),
            Some(&codegraph::PropertyValue::String("helper".to_string()))
        );
        assert_eq!(
            func_node.properties.get("visibility"),
            Some(&codegraph::PropertyValue::String("private".to_string()))
        );
    }

    #[test]
    fn test_ir_to_graph_with_complexity() {
        let mut ir = CodeIR::new(PathBuf::from("test.c"));

        let mut func = FunctionEntity::new("complex", 1, 20);
        func.complexity = Some(codegraph_parser_api::ComplexityMetrics {
            cyclomatic_complexity: 10,
            branches: 5,
            loops: 3,
            logical_operators: 2,
            max_nesting_depth: 4,
            exception_handlers: 0,
            early_returns: 1,
        });
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("test.c").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();

        // Verify complexity properties
        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(
            func_node.properties.get("cyclomatic_complexity"),
            Some(&codegraph::PropertyValue::Int(10))
        );
        assert_eq!(
            func_node.properties.get("complexity_grade"),
            Some(&codegraph::PropertyValue::String("B".to_string()))
        );
    }
}
