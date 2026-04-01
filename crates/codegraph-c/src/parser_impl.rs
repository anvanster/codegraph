// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the CodeParser trait for C

use codegraph::CodeGraph;
use codegraph_parser_api::{
    CodeIR, CodeParser, FileInfo, ParserConfig, ParserError, ParserMetrics, ProjectInfo,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::extractor;
use crate::mapper;

/// C language parser implementing the CodeParser trait
pub struct CParser {
    config: ParserConfig,
    metrics: Mutex<ParserMetrics>,
}

impl CParser {
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            metrics: Mutex::new(ParserMetrics::default()),
        }
    }

    /// Parse C source with additional type definitions from resolved headers.
    /// The server calls this instead of `parse_source` when header context is available.
    pub fn parse_source_with_headers(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
        header_types: Vec<(String, String)>,
    ) -> Result<FileInfo, ParserError> {
        let start = Instant::now();

        let needs_preprocess = !header_types.is_empty()
            || extractor::source_needs_type_preamble(source);

        let options = extractor::ExtractionOptions {
            extract_calls: true,
            preprocess: needs_preprocess,
            header_types,
            ..Default::default()
        };

        let ir = match extractor::extract_with_options(source, file_path, &options) {
            Ok(result) if result.is_partial => {
                // Retry with tolerant mode, preserving header types
                let tolerant = extractor::ExtractionOptions {
                    tolerant_mode: true,
                    preprocess: true,
                    extract_calls: true,
                    header_types: options.header_types,
                };
                extractor::extract_with_options(source, file_path, &tolerant)?.ir
            }
            Ok(result) => result.ir,
            Err(e) => return Err(e),
        };

        let mut file_info = self.ir_to_graph(&ir, graph, file_path)?;
        file_info.parse_time = start.elapsed();
        file_info.line_count = source.lines().count();
        file_info.byte_count = source.len();

        Ok(file_info)
    }

    /// Scan a C source file for `#include "..."` directives and resolve them
    /// relative to the file's directory. Returns extracted types from found headers.
    pub fn resolve_local_includes(source: &str, file_path: &Path) -> Vec<(String, String)> {
        let parent = match file_path.parent() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut all_types = Vec::new();
        let mut seen_headers = std::collections::HashSet::new();

        for line in source.lines() {
            let trimmed = line.trim();
            // Match #include "local_header.h" (not <system_header.h>)
            if let Some(rest) = trimmed.strip_prefix("#include") {
                let rest = rest.trim();
                if rest.starts_with('"') {
                    if let Some(end) = rest[1..].find('"') {
                        let header_name = &rest[1..1 + end];
                        let header_path = parent.join(header_name);
                        if header_path.exists() && seen_headers.insert(header_path.clone()) {
                            if let Ok(header_source) = std::fs::read_to_string(&header_path) {
                                let types =
                                    crate::preprocessor::CPreprocessor::extract_header_types(
                                        &header_source,
                                    );
                                all_types.extend(types);
                            }
                        }
                    }
                }
            }
        }

        all_types
    }

    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            metrics: Mutex::new(ParserMetrics::default()),
        }
    }

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

    fn ir_to_graph(
        &self,
        ir: &CodeIR,
        graph: &mut CodeGraph,
        file_path: &Path,
    ) -> Result<FileInfo, ParserError> {
        mapper::ir_to_graph(ir, graph, file_path)
    }
}

impl Default for CParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for CParser {
    fn language(&self) -> &str {
        "c"
    }

    fn file_extensions(&self) -> &[&str] {
        &[".c", ".h"]
    }

    fn parse_file(&self, path: &Path, graph: &mut CodeGraph) -> Result<FileInfo, ParserError> {
        let start = Instant::now();
        let metadata =
            fs::metadata(path).map_err(|e| ParserError::IoError(path.to_path_buf(), e))?;

        if metadata.len() as usize > self.config.max_file_size {
            return Err(ParserError::FileTooLarge(
                path.to_path_buf(),
                metadata.len() as usize,
            ));
        }

        let source =
            fs::read_to_string(path).map_err(|e| ParserError::IoError(path.to_path_buf(), e))?;
        let result = self.parse_source(&source, path, graph);

        let duration = start.elapsed();
        if let Ok(ref info) = result {
            self.update_metrics(true, duration, info.entity_count(), 0);
        } else {
            self.update_metrics(false, duration, 0, 0);
        }

        result
    }

    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        // Auto-resolve local #include "..." headers for type context
        let header_types = Self::resolve_local_includes(source, file_path);

        if header_types.is_empty() {
            // No local headers — use the original strict→tolerant flow
            let start = Instant::now();
            let ir = match extractor::extract(source, file_path, &self.config) {
                Ok(ir) => ir,
                Err(ParserError::SyntaxError(..)) => {
                    let options = extractor::ExtractionOptions::for_kernel_code();
                    let result =
                        extractor::extract_with_options(source, file_path, &options)?;
                    result.ir
                }
                Err(e) => return Err(e),
            };

            let mut file_info = self.ir_to_graph(&ir, graph, file_path)?;
            file_info.parse_time = start.elapsed();
            file_info.line_count = source.lines().count();
            file_info.byte_count = source.len();
            Ok(file_info)
        } else {
            // Headers found — use the header-aware path
            self.parse_source_with_headers(source, file_path, graph, header_types)
        }
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

    fn parse_files(
        &self,
        paths: &[PathBuf],
        graph: &mut CodeGraph,
    ) -> Result<ProjectInfo, ParserError> {
        if self.config.parallel {
            self.parse_files_parallel(paths, graph)
        } else {
            self.parse_files_sequential(paths, graph)
        }
    }
}

impl CParser {
    /// Parse files sequentially
    fn parse_files_sequential(
        &self,
        paths: &[PathBuf],
        graph: &mut CodeGraph,
    ) -> Result<ProjectInfo, ParserError> {
        let mut files = Vec::new();
        let mut failed_files = Vec::new();
        let mut total_functions = 0;
        let mut total_classes = 0;
        let mut total_parse_time = Duration::ZERO;

        for path in paths {
            match self.parse_file(path, graph) {
                Ok(info) => {
                    total_functions += info.functions.len();
                    total_classes += info.classes.len();
                    total_parse_time += info.parse_time;
                    files.push(info);
                }
                Err(e) => {
                    failed_files.push((path.clone(), e.to_string()));
                }
            }
        }

        Ok(ProjectInfo {
            files,
            failed_files,
            total_functions,
            total_classes,
            total_parse_time,
        })
    }

    /// Parse files in parallel using rayon
    fn parse_files_parallel(
        &self,
        paths: &[PathBuf],
        graph: &mut CodeGraph,
    ) -> Result<ProjectInfo, ParserError> {
        use rayon::prelude::*;

        let graph_mutex = Mutex::new(graph);

        // Configure thread pool if parallel_workers is specified
        let pool = if let Some(num_threads) = self.config.parallel_workers {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .map_err(|e| {
                    ParserError::GraphError(format!("Failed to create thread pool: {e}"))
                })?
        } else {
            rayon::ThreadPoolBuilder::new().build().map_err(|e| {
                ParserError::GraphError(format!("Failed to create thread pool: {e}"))
            })?
        };

        let results: Vec<_> = pool.install(|| {
            paths
                .par_iter()
                .map(|path| {
                    let mut graph = graph_mutex.lock().unwrap();
                    match self.parse_file(path, &mut graph) {
                        Ok(info) => Ok(info),
                        Err(e) => Err((path.clone(), e.to_string())),
                    }
                })
                .collect()
        });

        let mut files = Vec::new();
        let mut failed_files = Vec::new();
        let mut total_functions = 0;
        let mut total_classes = 0;
        let mut total_parse_time = Duration::ZERO;

        for result in results {
            match result {
                Ok(info) => {
                    total_functions += info.functions.len();
                    total_classes += info.classes.len();
                    total_parse_time += info.parse_time;
                    files.push(info);
                }
                Err((path, error)) => {
                    failed_files.push((path, error));
                }
            }
        }

        Ok(ProjectInfo {
            files,
            failed_files,
            total_functions,
            total_classes,
            total_parse_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language() {
        let parser = CParser::new();
        assert_eq!(parser.language(), "c");
    }

    #[test]
    fn test_file_extensions() {
        let parser = CParser::new();
        assert_eq!(parser.file_extensions(), &[".c", ".h"]);
    }

    #[test]
    fn test_can_parse() {
        let parser = CParser::new();
        assert!(parser.can_parse(Path::new("main.c")));
        assert!(parser.can_parse(Path::new("header.h")));
        assert!(!parser.can_parse(Path::new("main.rs")));
        assert!(!parser.can_parse(Path::new("main.cpp")));
    }
}
