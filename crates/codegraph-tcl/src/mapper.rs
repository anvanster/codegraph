//! Mapper for converting CodeIR + TclExtraData to CodeGraph nodes and edges

use codegraph::{CodeGraph, EdgeType, NodeId, NodeType, PropertyMap};
use codegraph_parser_api::{CodeIR, FileInfo, ParserError};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::extractor::TclExtraData;

pub fn ir_to_graph(
    ir: &CodeIR,
    extra: &TclExtraData,
    graph: &mut CodeGraph,
    file_path: &Path,
) -> Result<FileInfo, ParserError> {
    let mut node_map: HashMap<String, NodeId> = HashMap::new();
    let mut function_ids = Vec::new();
    let mut class_ids = Vec::new();
    let trait_ids = Vec::new();
    let mut import_ids = Vec::new();

    // Create module/file node with SDC/EDA properties
    let file_id = if let Some(ref module) = ir.module {
        let mut props = PropertyMap::new()
            .with("name", module.name.clone())
            .with("path", module.path.clone())
            .with("language", module.language.clone())
            .with("line_count", module.line_count.to_string());

        if let Some(ref doc) = module.doc_comment {
            props = props.with("doc", doc.clone());
        }

        // Attach SDC properties
        if !extra.sdc.clocks.is_empty() {
            props = props.with(
                "sdc_clocks",
                serde_json::to_string(&extra.sdc.clocks).unwrap_or_default(),
            );
        }
        if !extra.sdc.io_delays.is_empty() {
            props = props.with(
                "sdc_io_delays",
                serde_json::to_string(&extra.sdc.io_delays).unwrap_or_default(),
            );
        }
        if !extra.sdc.timing_exceptions.is_empty() {
            props = props.with(
                "sdc_timing_exceptions",
                serde_json::to_string(&extra.sdc.timing_exceptions).unwrap_or_default(),
            );
        }

        // Attach EDA data
        if !extra.eda.design_reads.is_empty() {
            props = props.with(
                "eda_design_reads",
                serde_json::to_string(&extra.eda.design_reads).unwrap_or_default(),
            );
        }
        if !extra.eda.design_writes.is_empty() {
            props = props.with(
                "eda_design_writes",
                serde_json::to_string(&extra.eda.design_writes).unwrap_or_default(),
            );
        }
        if !extra.eda.registered_commands.is_empty() {
            props = props.with(
                "eda_registered_commands",
                serde_json::to_string(&extra.eda.registered_commands).unwrap_or_default(),
            );
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
            .with("language", "tcl");

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
            .with("path", file_path.display().to_string())
            .with("signature", func.signature.clone())
            .with("visibility", func.visibility.clone())
            .with("line_start", func.line_start.to_string())
            .with("line_end", func.line_end.to_string())
            .with("is_async", func.is_async.to_string())
            .with("is_static", func.is_static.to_string())
            .with("is_abstract", func.is_abstract.to_string())
            .with("is_test", func.is_test.to_string());

        if let Some(ref doc) = func.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if let Some(ref parent) = func.parent_class {
            props = props.with("parent_class", parent.clone());
        }
        if !func.parameters.is_empty() {
            let param_names: Vec<String> = func.parameters.iter().map(|p| p.name.clone()).collect();
            props = props.with("parameters", param_names.join(","));
        }
        if !func.attributes.is_empty() {
            props = props.with("attributes", func.attributes.join(","));
        }
        if let Some(ref complexity) = func.complexity {
            props = props.with(
                "cyclomatic_complexity",
                complexity.cyclomatic_complexity.to_string(),
            );
        }

        let func_id = graph
            .add_node(NodeType::Function, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link function to parent namespace/class or file
        if let Some(ref parent_class) = func.parent_class {
            if let Some(&class_id) = node_map.get(parent_class) {
                graph
                    .add_edge(class_id, func_id, EdgeType::Contains, PropertyMap::new())
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        } else {
            graph
                .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add classes (namespaces in Tcl)
    for class in &ir.classes {
        let mut props = PropertyMap::new()
            .with("name", class.name.clone())
            .with("path", file_path.display().to_string())
            .with("visibility", class.visibility.clone())
            .with("line_start", class.line_start.to_string())
            .with("line_end", class.line_end.to_string())
            .with("is_abstract", class.is_abstract.to_string());

        if let Some(ref doc) = class.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if !class.attributes.is_empty() {
            props = props.with("attributes", class.attributes.join(","));
        }

        let class_id = graph
            .add_node(NodeType::Class, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(class.name.clone(), class_id);
        class_ids.push(class_id);

        // Link namespace to file
        graph
            .add_edge(file_id, class_id, EdgeType::Contains, PropertyMap::new())
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
    }

    // Add import relationships
    for import in &ir.imports {
        let imported_module = &import.imported;

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

        let mut edge_props = PropertyMap::new();
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
    let mut unresolved_calls: HashMap<String, Vec<String>> = HashMap::new();

    for call in &ir.calls {
        if let Some(&caller_id) = node_map.get(&call.caller) {
            if let Some(&callee_id) = node_map.get(&call.callee) {
                let edge_props = PropertyMap::new()
                    .with("call_site_line", call.call_site_line.to_string())
                    .with("is_direct", call.is_direct.to_string());

                graph
                    .add_edge(caller_id, callee_id, EdgeType::Calls, edge_props)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            } else {
                unresolved_calls
                    .entry(call.caller.clone())
                    .or_default()
                    .push(call.callee.clone());
            }
        }
    }

    // Store unresolved calls on caller nodes
    for (caller_name, callees) in unresolved_calls {
        if let Some(&caller_id) = node_map.get(&caller_name) {
            if let Ok(node) = graph.get_node(caller_id) {
                let existing = node
                    .properties
                    .get_string("unresolved_calls")
                    .unwrap_or("");
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
