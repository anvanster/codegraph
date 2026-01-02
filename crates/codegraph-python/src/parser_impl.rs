//! Implementation of the CodeParser trait for Python
//!
//! This module provides the PythonParser struct that implements the
//! codegraph-parser-api::CodeParser trait, making the Python parser compatible
//! with the unified parser API.

use codegraph::{CodeGraph, NodeId};
use codegraph_parser_api::{CodeParser, FileInfo, ParserConfig, ParserError, ParserMetrics};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Python language parser implementing the CodeParser trait
pub struct PythonParser {
    config: ParserConfig,
    metrics: Mutex<ParserMetrics>,
}

impl PythonParser {
    /// Create a new Python parser with default configuration
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            metrics: Mutex::new(ParserMetrics::default()),
        }
    }

    /// Create a new Python parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            metrics: Mutex::new(ParserMetrics::default()),
        }
    }

    /// Update metrics after parsing a file
    fn update_metrics(
        &self,
        success: bool,
        duration: Duration,
        entities: usize,
        relationships: usize,
    ) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.files_attempted += 1;
        if success {
            metrics.files_succeeded += 1;
        } else {
            metrics.files_failed += 1;
        }
        metrics.total_parse_time += duration;
        metrics.total_entities += entities;
        metrics.total_relationships += relationships;
    }

    /// Convert CodeIR to graph nodes and return FileInfo
    fn ir_to_graph(
        &self,
        ir: &codegraph_parser_api::CodeIR,
        graph: &mut CodeGraph,
        file_path: &Path,
    ) -> Result<FileInfo, ParserError> {
        use codegraph::{EdgeType, NodeType, PropertyMap};
        use std::collections::HashMap;

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
                .with("language", "python");

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
                .with("is_async", func.is_async.to_string())
                .with("is_static", func.is_static.to_string());

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

            // Link function to file
            graph
                .add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }

        // Add classes
        for class in &ir.classes {
            let mut props = PropertyMap::new()
                .with("name", class.name.clone())
                .with("visibility", class.visibility.clone())
                .with("line_start", class.line_start.to_string())
                .with("line_end", class.line_end.to_string())
                .with("is_abstract", class.is_abstract.to_string());

            if let Some(ref doc) = class.doc_comment {
                props = props.with("doc", doc.clone());
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
                    .with("signature", method.signature.clone())
                    .with("visibility", method.visibility.clone())
                    .with("line_start", method.line_start.to_string())
                    .with("line_end", method.line_end.to_string())
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

        // Add traits (protocols in Python)
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
        // Track unresolved calls per caller for cross-file resolution
        let mut unresolved_calls: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for call in &ir.calls {
            if let Some(&caller_id) = node_map.get(&call.caller) {
                if let Some(&callee_id) = node_map.get(&call.callee) {
                    // Both caller and callee are in this file - create direct edge
                    let edge_props = PropertyMap::new()
                        .with("call_site_line", call.call_site_line.to_string())
                        .with("is_direct", call.is_direct.to_string());

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

        // Add implementation relationships (class implements protocol/interface)
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
            parse_time: Duration::ZERO, // Will be set by caller
            line_count,
            byte_count: 0, // Will be set by caller
        })
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for PythonParser {
    fn language(&self) -> &str {
        "python"
    }

    fn file_extensions(&self) -> &[&str] {
        &[".py", ".pyw"]
    }

    fn parse_file(&self, path: &Path, graph: &mut CodeGraph) -> Result<FileInfo, ParserError> {
        let start = Instant::now();

        // Check file extension
        if !self.can_parse(path) {
            return Err(ParserError::ParseError(
                path.to_path_buf(),
                "Invalid file extension for Python parser".to_string(),
            ));
        }

        // Read file
        let source = std::fs::read_to_string(path)
            .map_err(|e| ParserError::IoError(path.to_path_buf(), e))?;

        // Check file size
        let byte_count = source.len();
        if byte_count > self.config.max_file_size {
            self.update_metrics(false, start.elapsed(), 0, 0);
            return Err(ParserError::FileTooLarge(path.to_path_buf(), byte_count));
        }

        // Parse source
        let mut file_info = self.parse_source(&source, path, graph)?;
        file_info.byte_count = byte_count;

        Ok(file_info)
    }

    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        let start = Instant::now();

        // Check size limit
        if source.len() > self.config.max_file_size {
            self.update_metrics(false, start.elapsed(), 0, 0);
            return Err(ParserError::FileTooLarge(
                file_path.to_path_buf(),
                source.len(),
            ));
        }

        // Extract entities using existing extractor
        // Convert ParserConfig to old config format
        let old_config = crate::config::ParserConfig {
            include_private: !self.config.skip_private,
            include_tests: !self.config.skip_tests,
            max_file_size: self.config.max_file_size,
            parallel: self.config.parallel,
            num_threads: self.config.parallel_workers,
            ..Default::default()
        };

        let ir = crate::extractor::extract(source, file_path, &old_config).map_err(|e| {
            self.update_metrics(false, start.elapsed(), 0, 0);
            ParserError::ParseError(file_path.to_path_buf(), e)
        })?;

        // Count entities and relationships
        let entity_count = ir.entity_count();
        let relationship_count = ir.relationship_count();

        // Convert IR to graph
        let mut file_info = self.ir_to_graph(&ir, graph, file_path)?;

        // Set timing and update metrics
        let duration = start.elapsed();
        file_info.parse_time = duration;
        file_info.byte_count = source.len();

        self.update_metrics(true, duration, entity_count, relationship_count);

        Ok(file_info)
    }

    fn config(&self) -> &ParserConfig {
        &self.config
    }

    fn metrics(&self) -> ParserMetrics {
        self.metrics.lock().unwrap().clone()
    }

    fn reset_metrics(&mut self) {
        *self.metrics.lock().unwrap() = ParserMetrics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_parser_new() {
        let parser = PythonParser::new();
        assert_eq!(parser.language(), "python");
    }

    #[test]
    fn test_python_parser_file_extensions() {
        let parser = PythonParser::new();
        let exts = parser.file_extensions();
        assert_eq!(exts.len(), 2);
        assert!(exts.contains(&".py"));
        assert!(exts.contains(&".pyw"));
    }

    #[test]
    fn test_python_parser_can_parse() {
        let parser = PythonParser::new();
        assert!(parser.can_parse(Path::new("test.py")));
        assert!(parser.can_parse(Path::new("test.pyw")));
        assert!(!parser.can_parse(Path::new("test.rs")));
        assert!(!parser.can_parse(Path::new("test.txt")));
    }

    #[test]
    fn test_metrics_initial_state() {
        let parser = PythonParser::new();
        let metrics = parser.metrics();
        assert_eq!(metrics.files_attempted, 0);
        assert_eq!(metrics.files_succeeded, 0);
        assert_eq!(metrics.files_failed, 0);
    }

    #[test]
    fn test_implements_edge_creation() {
        use codegraph::{CodeGraph, EdgeType};
        use codegraph_parser_api::{
            ClassEntity, CodeIR, ImplementationRelation, ModuleEntity, TraitEntity,
        };
        use std::path::PathBuf;

        let parser = PythonParser::new();

        // Create IR with a class implementing a protocol (Python's equivalent of interface)
        let mut ir = CodeIR::new(PathBuf::from("test.py"));
        ir.set_module(ModuleEntity::new("test", "test.py", "python"));
        ir.add_class(ClassEntity::new("MyClass", 1, 20));
        ir.add_trait(TraitEntity::new("MyProtocol", 22, 30));
        ir.add_implementation(ImplementationRelation::new("MyClass", "MyProtocol"));

        let mut graph = CodeGraph::in_memory().unwrap();
        let file_info = parser
            .ir_to_graph(&ir, &mut graph, Path::new("test.py"))
            .unwrap();

        assert_eq!(file_info.classes.len(), 1);
        assert_eq!(file_info.traits.len(), 1);

        // Find class and protocol node IDs
        let class_id = file_info.classes[0];
        let protocol_id = file_info.traits[0];

        // Verify implements edge was created
        let edges = graph.get_edges_between(class_id, protocol_id).unwrap();
        assert!(
            !edges.is_empty(),
            "Should have implements edge between class and protocol"
        );

        let edge = graph.get_edge(edges[0]).unwrap();
        assert_eq!(
            edge.edge_type,
            EdgeType::Implements,
            "Edge should be of type Implements"
        );
    }
}
