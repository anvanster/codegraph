//! Implementation of the CodeParser trait for Python
//!
//! This module provides the PythonParser struct that implements the
//! codegraph-parser-api::CodeParser trait, making the Python parser compatible
//! with the unified parser API.

use codegraph::{CodeGraph, NodeId};
use codegraph_parser_api::{
    CodeParser, FileInfo, ParserConfig, ParserError, ParserMetrics, ProjectInfo,
};
use std::path::{Path, PathBuf};
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
    fn update_metrics(&self, success: bool, duration: Duration, entities: usize, relationships: usize) {
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
        use codegraph::types::{Node, Edge, EdgeType, NodeType};
        use std::collections::HashMap;

        let mut node_map: HashMap<String, NodeId> = HashMap::new();
        let mut function_ids = Vec::new();
        let mut class_ids = Vec::new();
        let mut trait_ids = Vec::new();
        let mut import_ids = Vec::new();

        // Create module/file node
        let file_id = if let Some(ref module) = ir.module {
            let mut node = Node::new(
                module.name.clone(),
                NodeType::File,
            );
            node.add_property("path", module.path.clone());
            node.add_property("language", module.language.clone());
            node.add_property("line_count", module.line_count().to_string());

            if let Some(ref doc) = module.doc {
                node.add_property("doc", doc.clone());
            }

            let id = graph.add_node(node)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
            node_map.insert(module.name.clone(), id);
            id
        } else {
            // Create a default file node
            let file_name = file_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let mut node = Node::new(file_name.clone(), NodeType::File);
            node.add_property("path", file_path.display().to_string());
            node.add_property("language", "python");

            let id = graph.add_node(node)
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

            if let Some(ref doc) = func.doc {
                node.add_property("doc", doc.clone());
            }
            if let Some(ref return_type) = func.return_type {
                node.add_property("return_type", return_type.clone());
            }

            let func_id = graph.add_node(node)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            node_map.insert(func.name.clone(), func_id);
            function_ids.push(func_id);

            // Link function to file
            let edge = Edge::new(file_id, func_id, EdgeType::Contains);
            graph.add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }

        // Add classes
        for class in &ir.classes {
            let mut node = Node::new(class.name.clone(), NodeType::Class);
            node.add_property("visibility", class.visibility.clone());
            node.add_property("line_start", class.line_start.to_string());
            node.add_property("line_end", class.line_end.to_string());
            node.add_property("is_abstract", class.is_abstract.to_string());

            if let Some(ref doc) = class.doc {
                node.add_property("doc", doc.clone());
            }

            let class_id = graph.add_node(node)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            node_map.insert(class.name.clone(), class_id);
            class_ids.push(class_id);

            // Link class to file
            let edge = Edge::new(file_id, class_id, EdgeType::Contains);
            graph.add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            // Add methods
            for method in &class.methods {
                let method_name = format!("{}.{}", class.name, method.name);
                let mut node = Node::new(method_name.clone(), NodeType::Function);
                node.add_property("signature", method.signature.clone());
                node.add_property("visibility", method.visibility.clone());
                node.add_property("line_start", method.line_start.to_string());
                node.add_property("line_end", method.line_end.to_string());
                node.add_property("is_method", "true");
                node.add_property("parent_class", class.name.clone());

                if let Some(ref doc) = method.doc {
                    node.add_property("doc", doc.clone());
                }

                let method_id = graph.add_node(node)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;

                node_map.insert(method_name, method_id);
                function_ids.push(method_id);

                // Link method to class
                let edge = Edge::new(class_id, method_id, EdgeType::Contains);
                graph.add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        }

        // Add traits (protocols in Python)
        for trait_entity in &ir.traits {
            let mut node = Node::new(trait_entity.name.clone(), NodeType::Trait);
            node.add_property("visibility", trait_entity.visibility.clone());
            node.add_property("line_start", trait_entity.line_start.to_string());
            node.add_property("line_end", trait_entity.line_end.to_string());

            if let Some(ref doc) = trait_entity.doc {
                node.add_property("doc", doc.clone());
            }

            let trait_id = graph.add_node(node)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;

            node_map.insert(trait_entity.name.clone(), trait_id);
            trait_ids.push(trait_id);

            // Link trait to file
            let edge = Edge::new(file_id, trait_id, EdgeType::Contains);
            graph.add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }

        // Add import nodes and relationships
        for import in &ir.imports {
            let imported_module = &import.imported;

            // Create or get import node
            let import_id = if let Some(&existing_id) = node_map.get(imported_module) {
                existing_id
            } else {
                let mut node = Node::new(imported_module.clone(), NodeType::Module);
                node.add_property("is_external", "true");

                let id = graph.add_node(node)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
                node_map.insert(imported_module.clone(), id);
                id
            };

            import_ids.push(import_id);

            // Create import edge from file to imported module
            let mut edge = Edge::new(file_id, import_id, EdgeType::Imports);
            if let Some(ref alias) = import.alias {
                edge.add_property("alias", alias.clone());
            }
            if import.is_wildcard {
                edge.add_property("is_wildcard", "true");
            }
            if !import.symbols.is_empty() {
                edge.add_property("symbols", import.symbols.join(","));
            }
            graph.add_edge(edge)
                .map_err(|e| ParserError::GraphError(e.to_string()))?;
        }

        // Add call relationships
        for call in &ir.calls {
            if let (Some(&caller_id), Some(&callee_id)) =
                (node_map.get(&call.caller), node_map.get(&call.callee)) {
                let mut edge = Edge::new(caller_id, callee_id, EdgeType::Calls);
                edge.add_property("call_site_line", call.call_site_line.to_string());
                edge.add_property("is_direct", call.is_direct.to_string());

                graph.add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        }

        // Add inheritance relationships
        for inheritance in &ir.inheritance {
            if let (Some(&child_id), Some(&parent_id)) =
                (node_map.get(&inheritance.child), node_map.get(&inheritance.parent)) {
                let mut edge = Edge::new(child_id, parent_id, EdgeType::Inherits);
                edge.add_property("order", inheritance.order.to_string());

                graph.add_edge(edge)
                    .map_err(|e| ParserError::GraphError(e.to_string()))?;
            }
        }

        // Count source lines
        let line_count = if let Some(ref module) = ir.module {
            module.line_count()
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

    fn parse_file(
        &self,
        path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
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

        let ir = crate::extractor::extract(source, file_path, &old_config)
            .map_err(|e| {
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
}
