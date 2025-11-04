# CodeGraph Parser API Specification v0.1.0

## Overview

This document provides the complete specification for `codegraph-parser-api`, the shared contract that all CodeGraph language parsers must implement.

## Crate Structure

```
codegraph-parser-api/
├── src/
│   ├── lib.rs              # Main exports
│   ├── traits.rs           # CodeParser trait
│   ├── config.rs           # Configuration types
│   ├── metrics.rs          # Metrics and telemetry
│   ├── errors.rs           # Error types
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── function.rs     # FunctionEntity
│   │   ├── class.rs        # ClassEntity
│   │   ├── module.rs       # ModuleEntity
│   │   └── trait_.rs       # TraitEntity
│   ├── relationships/
│   │   ├── mod.rs
│   │   ├── calls.rs        # CallRelation
│   │   ├── imports.rs      # ImportRelation
│   │   ├── inheritance.rs  # InheritanceRelation
│   │   └── implementations.rs  # ImplementationRelation
│   └── ir.rs               # Intermediate Representation
├── Cargo.toml
└── README.md
```

## Dependencies

```toml
[package]
name = "codegraph-parser-api"
version = "0.1.0"
edition = "2021"
authors = ["anvanster"]
license = "Apache-2.0"
description = "Shared API and types for CodeGraph language parsers"
repository = "https://github.com/anvanster/codegraph"
keywords = ["parser", "code-analysis", "ast", "graph"]
categories = ["parser-implementations", "data-structures"]

[dependencies]
codegraph = "0.1.1"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

[dev-dependencies]
serde_json = "1.0"
```

---

## Core Trait: CodeParser

### Definition

```rust
// src/traits.rs

use crate::{
    config::ParserConfig,
    errors::ParserError,
    ir::CodeIR,
    metrics::ParserMetrics,
};
use codegraph::{CodeGraph, NodeId};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Core trait that all language parsers must implement
///
/// This trait defines the contract for extracting code entities and relationships
/// from source code and inserting them into a CodeGraph database.
///
/// # Thread Safety
/// Implementations must be `Send + Sync` to support parallel parsing.
///
/// # Example
/// ```rust,ignore
/// use codegraph_parser_api::{CodeParser, ParserConfig};
/// use codegraph::CodeGraph;
///
/// struct MyParser {
///     config: ParserConfig,
/// }
///
/// impl CodeParser for MyParser {
///     fn language(&self) -> &str {
///         "mylang"
///     }
///     
///     fn file_extensions(&self) -> &[&str] {
///         &[".my"]
///     }
///     
///     // ... implement other required methods
/// }
/// ```
pub trait CodeParser: Send + Sync {
    /// Returns the language identifier (lowercase, e.g., "python", "rust")
    fn language(&self) -> &str;

    /// Returns supported file extensions (e.g., [".py", ".pyw"])
    fn file_extensions(&self) -> &[&str];

    /// Parse a single file and insert entities/relationships into the graph
    ///
    /// # Arguments
    /// * `path` - Path to the source file
    /// * `graph` - Mutable reference to the CodeGraph database
    ///
    /// # Returns
    /// `FileInfo` containing metadata about parsed entities
    ///
    /// # Errors
    /// Returns `ParserError` if:
    /// - File cannot be read
    /// - Source code has syntax errors
    /// - Graph insertion fails
    fn parse_file(
        &self,
        path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError>;

    /// Parse source code string and insert into graph
    ///
    /// Useful for parsing code snippets or in-memory source.
    ///
    /// # Arguments
    /// * `source` - Source code string
    /// * `file_path` - Logical path for this source (used for graph nodes)
    /// * `graph` - Mutable reference to the CodeGraph database
    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError>;

    /// Parse multiple files (can be overridden for parallel parsing)
    ///
    /// Default implementation parses files sequentially. Override this
    /// for parallel parsing implementation.
    ///
    /// # Arguments
    /// * `paths` - List of file paths to parse
    /// * `graph` - Mutable reference to the CodeGraph database
    ///
    /// # Returns
    /// `ProjectInfo` containing aggregate statistics
    fn parse_files(
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
            total_functions,
            total_classes,
            total_parse_time,
            failed_files,
        })
    }

    /// Parse a directory recursively
    ///
    /// # Arguments
    /// * `dir` - Directory path to parse
    /// * `graph` - Mutable reference to the CodeGraph database
    fn parse_directory(
        &self,
        dir: &Path,
        graph: &mut CodeGraph,
    ) -> Result<ProjectInfo, ParserError> {
        let paths = self.discover_files(dir)?;
        self.parse_files(&paths, graph)
    }

    /// Discover parseable files in a directory
    ///
    /// Default implementation walks the directory and filters by extension.
    /// Can be overridden for custom discovery logic.
    fn discover_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ParserError> {
        use std::fs;

        let mut files = Vec::new();
        let extensions = self.file_extensions();

        fn walk_dir(
            dir: &Path,
            extensions: &[&str],
            files: &mut Vec<PathBuf>,
        ) -> Result<(), ParserError> {
            if !dir.is_dir() {
                return Ok(());
            }

            for entry in fs::read_dir(dir)
                .map_err(|e| ParserError::IoError(dir.to_path_buf(), e))?
            {
                let entry = entry
                    .map_err(|e| ParserError::IoError(dir.to_path_buf(), e))?;
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(&path, extensions, files)?;
                } else if let Some(ext) = path.extension() {
                    let ext_str = format!(".{}", ext.to_string_lossy());
                    if extensions.contains(&ext_str.as_str()) {
                        files.push(path);
                    }
                }
            }

            Ok(())
        }

        walk_dir(dir, extensions, &mut files)?;
        Ok(files)
    }

    /// Check if this parser can handle the given file
    ///
    /// Default implementation checks file extension.
    fn can_parse(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = format!(".{}", ext.to_string_lossy());
            self.file_extensions().contains(&ext_str.as_str())
        } else {
            false
        }
    }

    /// Get parser configuration
    fn config(&self) -> &ParserConfig;

    /// Get accumulated metrics
    ///
    /// Returns current parsing metrics (files processed, time taken, etc.)
    fn metrics(&self) -> ParserMetrics;

    /// Reset metrics
    ///
    /// Clears accumulated metrics. Useful for benchmarking.
    fn reset_metrics(&mut self);
}
```

---

## Return Types

### FileInfo

```rust
// src/traits.rs

use serde::{Deserialize, Serialize};

/// Information about a successfully parsed file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    /// Path to the source file
    pub file_path: PathBuf,

    /// Node ID of the file/module in the graph
    pub file_id: NodeId,

    /// Node IDs of all functions extracted
    pub functions: Vec<NodeId>,

    /// Node IDs of all classes extracted
    pub classes: Vec<NodeId>,

    /// Node IDs of all traits/interfaces extracted
    pub traits: Vec<NodeId>,

    /// Node IDs of all imports extracted
    pub imports: Vec<NodeId>,

    /// Time taken to parse this file
    pub parse_time: Duration,

    /// Number of lines in the file
    pub line_count: usize,

    /// File size in bytes
    pub byte_count: usize,
}

impl FileInfo {
    /// Total number of entities extracted
    pub fn entity_count(&self) -> usize {
        self.functions.len() + self.classes.len() + self.traits.len()
    }
}
```

### ProjectInfo

```rust
// src/traits.rs

/// Aggregate information about a parsed project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Information about each successfully parsed file
    pub files: Vec<FileInfo>,

    /// Total number of functions across all files
    pub total_functions: usize,

    /// Total number of classes across all files
    pub total_classes: usize,

    /// Total parse time for all files
    pub total_parse_time: Duration,

    /// Files that failed to parse (path, error message)
    pub failed_files: Vec<(PathBuf, String)>,
}

impl ProjectInfo {
    /// Total number of files processed (success + failure)
    pub fn total_files(&self) -> usize {
        self.files.len() + self.failed_files.len()
    }

    /// Success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_files() == 0 {
            0.0
        } else {
            self.files.len() as f64 / self.total_files() as f64
        }
    }

    /// Average parse time per file
    pub fn avg_parse_time(&self) -> Duration {
        if self.files.is_empty() {
            Duration::ZERO
        } else {
            self.total_parse_time / self.files.len() as u32
        }
    }
}
```

---

## Configuration

```rust
// src/config.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for parser behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Skip private/internal entities (language-specific)
    pub skip_private: bool,

    /// Skip test files and test functions
    pub skip_tests: bool,

    /// Maximum file size to parse (in bytes)
    /// Files larger than this will be skipped
    pub max_file_size: usize,

    /// Timeout per file (None = no timeout)
    pub timeout_per_file: Option<Duration>,

    /// Enable parallel parsing (for `parse_files`)
    pub parallel: bool,

    /// Number of parallel workers (None = use num_cpus)
    pub parallel_workers: Option<usize>,

    /// Include documentation/comments in entities
    pub include_docs: bool,

    /// Extract type information (when available)
    pub extract_types: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            skip_private: false,
            skip_tests: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            timeout_per_file: Some(Duration::from_secs(30)),
            parallel: false,
            parallel_workers: None,
            include_docs: true,
            extract_types: true,
        }
    }
}

impl ParserConfig {
    /// Create config for fast parsing (skips tests, docs, types)
    pub fn fast() -> Self {
        Self {
            skip_tests: true,
            include_docs: false,
            extract_types: false,
            ..Default::default()
        }
    }

    /// Create config for comprehensive parsing
    pub fn comprehensive() -> Self {
        Self {
            skip_private: false,
            skip_tests: false,
            include_docs: true,
            extract_types: true,
            ..Default::default()
        }
    }

    /// Enable parallel parsing
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set maximum file size
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }
}
```

---

## Metrics

```rust
// src/metrics.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Metrics collected during parsing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParserMetrics {
    /// Total files attempted to parse
    pub files_attempted: usize,

    /// Files successfully parsed
    pub files_succeeded: usize,

    /// Files that failed parsing
    pub files_failed: usize,

    /// Total time spent parsing
    pub total_parse_time: Duration,

    /// Total entities extracted
    pub total_entities: usize,

    /// Total relationships extracted
    pub total_relationships: usize,

    /// Peak memory usage (if available)
    pub peak_memory_bytes: Option<usize>,
}

impl Default for ParserMetrics {
    fn default() -> Self {
        Self {
            files_attempted: 0,
            files_succeeded: 0,
            files_failed: 0,
            total_parse_time: Duration::ZERO,
            total_entities: 0,
            total_relationships: 0,
            peak_memory_bytes: None,
        }
    }
}

impl ParserMetrics {
    /// Success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.files_attempted == 0 {
            0.0
        } else {
            self.files_succeeded as f64 / self.files_attempted as f64
        }
    }

    /// Average parse time per file
    pub fn avg_parse_time(&self) -> Duration {
        if self.files_succeeded == 0 {
            Duration::ZERO
        } else {
            self.total_parse_time / self.files_succeeded as u32
        }
    }

    /// Average entities per file
    pub fn avg_entities_per_file(&self) -> f64 {
        if self.files_succeeded == 0 {
            0.0
        } else {
            self.total_entities as f64 / self.files_succeeded as f64
        }
    }

    /// Merge another metrics object into this one
    pub fn merge(&mut self, other: &ParserMetrics) {
        self.files_attempted += other.files_attempted;
        self.files_succeeded += other.files_succeeded;
        self.files_failed += other.files_failed;
        self.total_parse_time += other.total_parse_time;
        self.total_entities += other.total_entities;
        self.total_relationships += other.total_relationships;

        // Take max memory
        self.peak_memory_bytes = match (self.peak_memory_bytes, other.peak_memory_bytes) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
    }
}
```

---

## Error Handling

```rust
// src/errors.rs

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Error, Debug)]
pub enum ParserError {
    /// Failed to read file
    #[error("IO error reading {0}: {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    /// Syntax error in source code
    #[error("Syntax error in {0}:{1}:{2}: {3}")]
    SyntaxError(PathBuf, usize, usize, String),

    /// File too large
    #[error("File {0} exceeds maximum size ({1} bytes)")]
    FileTooLarge(PathBuf, usize),

    /// Parsing timeout
    #[error("Parsing {0} exceeded timeout")]
    Timeout(PathBuf),

    /// Graph insertion error
    #[error("Failed to insert into graph: {0}")]
    GraphError(String),

    /// Unsupported language feature
    #[error("Unsupported language feature in {0}: {1}")]
    UnsupportedFeature(PathBuf, String),

    /// Generic parsing error
    #[error("Parse error in {0}: {1}")]
    ParseError(PathBuf, String),
}

/// Result type for parser operations
pub type ParserResult<T> = Result<T, ParserError>;
```

---

## Entities

All entities are in the `entities/` module. Here's the complete specification:

### FunctionEntity

```rust
// src/entities/function.rs

use serde::{Deserialize, Serialize};

/// Represents a function parameter
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Type annotation (if available)
    pub type_annotation: Option<String>,

    /// Default value (if any)
    pub default_value: Option<String>,

    /// Is this a variadic parameter? (e.g., *args, **kwargs)
    pub is_variadic: bool,
}

impl Parameter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: None,
            default_value: None,
            is_variadic: false,
        }
    }

    pub fn with_type(mut self, type_ann: impl Into<String>) -> Self {
        self.type_annotation = Some(type_ann.into());
        self
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    pub fn variadic(mut self) -> Self {
        self.is_variadic = true;
        self
    }
}

/// Represents a function/method in any language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionEntity {
    /// Function name
    pub name: String,

    /// Full signature (including parameters and return type)
    pub signature: String,

    /// Visibility: "public", "private", "protected", "internal"
    pub visibility: String,

    /// Starting line number (1-indexed)
    pub line_start: usize,

    /// Ending line number (1-indexed)
    pub line_end: usize,

    /// Is this an async/coroutine function?
    pub is_async: bool,

    /// Is this a test function?
    pub is_test: bool,

    /// Is this a static method?
    pub is_static: bool,

    /// Is this an abstract method?
    pub is_abstract: bool,

    /// Function parameters
    pub parameters: Vec<Parameter>,

    /// Return type annotation (if available)
    pub return_type: Option<String>,

    /// Documentation/docstring
    pub doc_comment: Option<String>,

    /// Decorators/attributes (e.g., [@property], [@deprecated])
    pub attributes: Vec<String>,

    /// Parent class (if this is a method)
    pub parent_class: Option<String>,
}

impl FunctionEntity {
    pub fn new(name: impl Into<String>, line_start: usize, line_end: usize) -> Self {
        Self {
            name: name.into(),
            signature: String::new(),
            visibility: "public".to_string(),
            line_start,
            line_end,
            is_async: false,
            is_test: false,
            is_static: false,
            is_abstract: false,
            parameters: Vec::new(),
            return_type: None,
            doc_comment: None,
            attributes: Vec::new(),
            parent_class: None,
        }
    }

    // Builder methods
    pub fn with_signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = sig.into();
        self
    }

    pub fn with_visibility(mut self, vis: impl Into<String>) -> Self {
        self.visibility = vis.into();
        self
    }

    pub fn async_fn(mut self) -> Self {
        self.is_async = true;
        self
    }

    pub fn test_fn(mut self) -> Self {
        self.is_test = true;
        self
    }

    pub fn with_parameters(mut self, params: Vec<Parameter>) -> Self {
        self.parameters = params;
        self
    }

    pub fn with_return_type(mut self, ret: impl Into<String>) -> Self {
        self.return_type = Some(ret.into());
        self
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc_comment = Some(doc.into());
        self
    }

    pub fn with_attributes(mut self, attrs: Vec<String>) -> Self {
        self.attributes = attrs;
        self
    }
}
```

### ClassEntity

```rust
// src/entities/class.rs

use super::function::FunctionEntity;
use serde::{Deserialize, Serialize};

/// Represents a class field/attribute
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Field {
    /// Field name
    pub name: String,

    /// Type annotation (if available)
    pub type_annotation: Option<String>,

    /// Visibility: "public", "private", "protected"
    pub visibility: String,

    /// Is this a static/class field?
    pub is_static: bool,

    /// Is this a constant?
    pub is_constant: bool,

    /// Default value
    pub default_value: Option<String>,
}

impl Field {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: None,
            visibility: "public".to_string(),
            is_static: false,
            is_constant: false,
            default_value: None,
        }
    }

    pub fn with_type(mut self, type_ann: impl Into<String>) -> Self {
        self.type_annotation = Some(type_ann.into());
        self
    }

    pub fn static_field(mut self) -> Self {
        self.is_static = true;
        self
    }

    pub fn constant(mut self) -> Self {
        self.is_constant = true;
        self
    }
}

/// Represents a class/struct in any language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassEntity {
    /// Class name
    pub name: String,

    /// Visibility: "public", "private", "internal"
    pub visibility: String,

    /// Starting line number (1-indexed)
    pub line_start: usize,

    /// Ending line number (1-indexed)
    pub line_end: usize,

    /// Is this an abstract class?
    pub is_abstract: bool,

    /// Is this an interface/trait definition?
    pub is_interface: bool,

    /// Base classes (inheritance)
    pub base_classes: Vec<String>,

    /// Interfaces/traits implemented
    pub implemented_traits: Vec<String>,

    /// Methods in this class
    pub methods: Vec<FunctionEntity>,

    /// Fields/attributes
    pub fields: Vec<Field>,

    /// Documentation/docstring
    pub doc_comment: Option<String>,

    /// Decorators/attributes
    pub attributes: Vec<String>,

    /// Generic type parameters (if any)
    pub type_parameters: Vec<String>,
}

impl ClassEntity {
    pub fn new(name: impl Into<String>, line_start: usize, line_end: usize) -> Self {
        Self {
            name: name.into(),
            visibility: "public".to_string(),
            line_start,
            line_end,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment: None,
            attributes: Vec::new(),
            type_parameters: Vec::new(),
        }
    }

    pub fn abstract_class(mut self) -> Self {
        self.is_abstract = true;
        self
    }

    pub fn interface(mut self) -> Self {
        self.is_interface = true;
        self
    }

    pub fn with_bases(mut self, bases: Vec<String>) -> Self {
        self.base_classes = bases;
        self
    }

    pub fn with_methods(mut self, methods: Vec<FunctionEntity>) -> Self {
        self.methods = methods;
        self
    }

    pub fn with_fields(mut self, fields: Vec<Field>) -> Self {
        self.fields = fields;
        self
    }
}
```

### ModuleEntity & TraitEntity

```rust
// src/entities/module.rs

use serde::{Deserialize, Serialize};

/// Represents a file/module
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEntity {
    /// Module name (usually filename without extension)
    pub name: String,

    /// Full path to the file
    pub path: String,

    /// Language identifier
    pub language: String,

    /// Number of lines
    pub line_count: usize,

    /// Documentation/module docstring
    pub doc_comment: Option<String>,

    /// Module-level attributes/pragmas
    pub attributes: Vec<String>,
}

// src/entities/trait_.rs

use super::function::FunctionEntity;
use serde::{Deserialize, Serialize};

/// Represents a trait/protocol/interface definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitEntity {
    /// Trait name
    pub name: String,

    /// Visibility
    pub visibility: String,

    /// Starting line number
    pub line_start: usize,

    /// Ending line number
    pub line_end: usize,

    /// Required methods
    pub required_methods: Vec<FunctionEntity>,

    /// Parent traits (trait inheritance)
    pub parent_traits: Vec<String>,

    /// Documentation
    pub doc_comment: Option<String>,
}
```

---

## Relationships

```rust
// src/relationships/calls.rs

use codegraph::NodeId;
use serde::{Deserialize, Serialize};

/// Represents a function call relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallRelation {
    /// Caller function name
    pub caller: String,

    /// Callee function name
    pub callee: String,

    /// Line number where the call occurs
    pub call_site_line: usize,

    /// Is this a direct call or indirect (e.g., through function pointer)?
    pub is_direct: bool,
}

// src/relationships/imports.rs

/// Represents an import/dependency relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportRelation {
    /// Importing module
    pub importer: String,

    /// Imported module
    pub imported: String,

    /// Specific symbols imported (empty = whole module)
    pub symbols: Vec<String>,

    /// Is this a wildcard import?
    pub is_wildcard: bool,

    /// Import alias (if any)
    pub alias: Option<String>,
}

// src/relationships/inheritance.rs

/// Represents class inheritance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InheritanceRelation {
    /// Child class
    pub child: String,

    /// Parent class
    pub parent: String,

    /// Inheritance order (for multiple inheritance)
    pub order: usize,
}

// src/relationships/implementations.rs

/// Represents trait/interface implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImplementationRelation {
    /// Implementing class
    pub implementor: String,

    /// Trait/interface being implemented
    pub trait_name: String,
}
```

---

## Intermediate Representation (IR)

```rust
// src/ir.rs

use crate::{
    entities::{ClassEntity, FunctionEntity, ModuleEntity, TraitEntity},
    relationships::{
        CallRelation, ImplementationRelation, ImportRelation, InheritanceRelation,
    },
};
use std::path::PathBuf;

/// Intermediate representation of extracted code
///
/// This is the bridge between language-specific AST and the CodeGraph database.
/// Parsers extract entities and relationships into this IR, then the IR is
/// inserted into the graph in a batch operation.
#[derive(Debug, Default, Clone)]
pub struct CodeIR {
    /// Source file path
    pub file_path: PathBuf,

    /// Module/file entity
    pub module: Option<ModuleEntity>,

    /// Extracted functions
    pub functions: Vec<FunctionEntity>,

    /// Extracted classes
    pub classes: Vec<ClassEntity>,

    /// Extracted traits/interfaces
    pub traits: Vec<TraitEntity>,

    /// Function call relationships
    pub calls: Vec<CallRelation>,

    /// Import relationships
    pub imports: Vec<ImportRelation>,

    /// Inheritance relationships
    pub inheritance: Vec<InheritanceRelation>,

    /// Implementation relationships
    pub implementations: Vec<ImplementationRelation>,
}

impl CodeIR {
    /// Create a new empty IR
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            ..Default::default()
        }
    }

    /// Total number of entities
    pub fn entity_count(&self) -> usize {
        self.functions.len()
            + self.classes.len()
            + self.traits.len()
            + if self.module.is_some() { 1 } else { 0 }
    }

    /// Total number of relationships
    pub fn relationship_count(&self) -> usize {
        self.calls.len()
            + self.imports.len()
            + self.inheritance.len()
            + self.implementations.len()
    }

    /// Add a function
    pub fn add_function(&mut self, func: FunctionEntity) {
        self.functions.push(func);
    }

    /// Add a class
    pub fn add_class(&mut self, class: ClassEntity) {
        self.classes.push(class);
    }

    /// Add a trait
    pub fn add_trait(&mut self, trait_entity: TraitEntity) {
        self.traits.push(trait_entity);
    }

    /// Add a call relationship
    pub fn add_call(&mut self, call: CallRelation) {
        self.calls.push(call);
    }

    /// Add an import relationship
    pub fn add_import(&mut self, import: ImportRelation) {
        self.imports.push(import);
    }

    /// Add an inheritance relationship
    pub fn add_inheritance(&mut self, inheritance: InheritanceRelation) {
        self.inheritance.push(inheritance);
    }

    /// Add an implementation relationship
    pub fn add_implementation(&mut self, implementation: ImplementationRelation) {
        self.implementations.push(implementation);
    }
}
```

---

## Summary

This specification provides:

1. **CodeParser trait** - Core interface all parsers implement
2. **Configuration** - ParserConfig for customizing behavior
3. **Metrics** - ParserMetrics for monitoring performance
4. **Entities** - Language-agnostic types for code elements
5. **Relationships** - Types for code dependencies
6. **IR** - Intermediate representation for batch operations
7. **Errors** - Comprehensive error handling

All types are:
- **Serializable** (Serde support)
- **Thread-safe** (Send + Sync where needed)
- **Documented** (comprehensive doc comments)
- **Testable** (derived traits for assertions)

Next: See MIGRATION_GUIDE.md for step-by-step implementation instructions.
