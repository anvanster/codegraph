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
        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let props = PropertyMap::new()
            .with("name", name.clone())
            .with("path", file_path.display().to_string())
            .with("language", "csharp");

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
            .with("line_start", func.line_start as i64)
            .with("line_end", func.line_end as i64)
            .with("is_async", func.is_async.to_string())
            .with("is_static", func.is_static.to_string())
            .with("is_abstract", func.is_abstract.to_string())
            .with("is_test", func.is_test.to_string());

        if let Some(ref doc) = func.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if let Some(ref return_type) = func.return_type {
            props = props.with("return_type", return_type.clone());
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

        let func_id = graph
            .add_node(NodeType::Function, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(func.name.clone(), func_id);
        function_ids.push(func_id);

        // Link function to file or parent class
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

    // Add classes
    for class in &ir.classes {
        let mut props = PropertyMap::new()
            .with("name", class.name.clone())
            .with("path", file_path.display().to_string())
            .with("visibility", class.visibility.clone())
            .with("line_start", class.line_start as i64)
            .with("line_end", class.line_end as i64)
            .with("is_abstract", class.is_abstract.to_string());

        if let Some(ref doc) = class.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if !class.attributes.is_empty() {
            props = props.with("attributes", class.attributes.join(","));
        }
        if !class.type_parameters.is_empty() {
            props = props.with("type_parameters", class.type_parameters.join(","));
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

        // Add methods
        for method in &class.methods {
            let method_name = format!("{}.{}", class.name, method.name);
            let mut method_props = PropertyMap::new()
                .with("name", method_name.clone())
                .with("path", file_path.display().to_string())
                .with("signature", method.signature.clone())
                .with("visibility", method.visibility.clone())
                .with("line_start", method.line_start as i64)
                .with("line_end", method.line_end as i64)
                .with("is_method", "true")
                .with("parent_class", class.name.clone());

            if let Some(ref doc) = method.doc_comment {
                method_props = method_props.with("doc", doc.clone());
            }

            let method_id = graph
                .add_node(NodeType::Function, method_props)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            node_map.insert(method_name, method_id);
            function_ids.push(method_id);

            // Link method to class
            graph
                .add_edge(class_id, method_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }
    }

    // Add interfaces
    for trait_entity in &ir.traits {
        let mut props = PropertyMap::new()
            .with("name", trait_entity.name.clone())
            .with("path", file_path.display().to_string())
            .with("visibility", trait_entity.visibility.clone())
            .with("line_start", trait_entity.line_start as i64)
            .with("line_end", trait_entity.line_end as i64);

        if let Some(ref doc) = trait_entity.doc_comment {
            props = props.with("doc", doc.clone());
        }
        if !trait_entity.required_methods.is_empty() {
            let method_names: Vec<String> = trait_entity
                .required_methods
                .iter()
                .map(|m| m.name.clone())
                .collect();
            props = props.with("required_methods", method_names.join(","));
        }

        let trait_id = graph
            .add_node(NodeType::Interface, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;

        node_map.insert(trait_entity.name.clone(), trait_id);
        trait_ids.push(trait_id);

        // Link interface to file
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

    // Add implementation relationships (class implements interface)
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
    use codegraph_parser_api::{ClassEntity, FunctionEntity, ImportRelation, TraitEntity};
    use std::path::PathBuf;

    #[test]
    fn test_ir_to_graph_empty() {
        let ir = CodeIR::new(PathBuf::from("Test.cs"));
        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 0);
        assert_eq!(file_info.classes.len(), 0);
        assert_eq!(file_info.traits.len(), 0);
    }

    #[test]
    fn test_ir_to_graph_with_function() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_function(FunctionEntity::new("TestFunc", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_class() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_class(ClassEntity::new("Person", 1, 10));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_interface() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_trait(TraitEntity::new("IReadable", 1, 5));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.traits.len(), 1);
    }

    #[test]
    fn test_ir_to_graph_with_module() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.set_module(codegraph_parser_api::ModuleEntity::new(
            "Program", "Test.cs", "csharp",
        ));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        graph.get_node(file_info.file_id).unwrap();
    }

    #[test]
    fn test_ir_to_graph_with_imports() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_import(ImportRelation::new("global", "System"));
        ir.add_import(ImportRelation::new("global", "System.Collections.Generic"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.imports.len(), 2);
    }

    #[test]
    fn test_ir_to_graph_with_methods() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));

        let mut class = ClassEntity::new("Calculator", 1, 10);
        class.methods.push(FunctionEntity::new("Add", 2, 4));
        class.methods.push(FunctionEntity::new("Subtract", 5, 7));
        ir.add_class(class);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
        assert_eq!(file_info.functions.len(), 2);
    }

    #[test]
    fn test_ir_to_graph_function_properties() {
        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        let func = FunctionEntity::new("PublicFunc", 1, 5)
            .with_visibility("public")
            .with_signature("public string PublicFunc()");
        ir.add_function(func);

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.functions.len(), 1);

        let func_node = graph.get_node(file_info.functions[0]).unwrap();
        assert_eq!(
            func_node.properties.get("name"),
            Some(&codegraph::PropertyValue::String("PublicFunc".to_string()))
        );
        assert_eq!(
            func_node.properties.get("visibility"),
            Some(&codegraph::PropertyValue::String("public".to_string()))
        );
    }

    #[test]
    fn test_ir_to_graph_with_implementation() {
        use codegraph::EdgeType;
        use codegraph_parser_api::ImplementationRelation;

        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_class(ClassEntity::new("Circle", 1, 20));
        ir.add_trait(TraitEntity::new("IShape", 22, 30));
        ir.add_implementation(ImplementationRelation::new("Circle", "IShape"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 1);
        assert_eq!(file_info.traits.len(), 1);

        let class_id = file_info.classes[0];
        let interface_id = file_info.traits[0];

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

    #[test]
    fn test_ir_to_graph_with_inheritance() {
        use codegraph::EdgeType;
        use codegraph_parser_api::InheritanceRelation;

        let mut ir = CodeIR::new(PathBuf::from("Test.cs"));
        ir.add_class(ClassEntity::new("Animal", 1, 10));
        ir.add_class(ClassEntity::new("Dog", 12, 25));
        ir.add_inheritance(InheritanceRelation::new("Dog", "Animal"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let result = ir_to_graph(&ir, &mut graph, PathBuf::from("Test.cs").as_path());

        assert!(result.is_ok());
        let file_info = result.unwrap();
        assert_eq!(file_info.classes.len(), 2);

        // Find Dog and Animal node IDs
        let dog_id = file_info.classes[1];
        let animal_id = file_info.classes[0];

        let edges = graph.get_edges_between(dog_id, animal_id).unwrap();
        assert!(
            !edges.is_empty(),
            "Should have extends edge between Dog and Animal"
        );

        let edge = graph.get_edge(edges[0]).unwrap();
        assert_eq!(
            edge.edge_type,
            EdgeType::Extends,
            "Edge should be of type Extends"
        );
    }
}
