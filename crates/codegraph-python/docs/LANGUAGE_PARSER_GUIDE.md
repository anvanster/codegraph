# Language Parser Module Development Guide
## Constitutional Document for codegraph-* Parsers

**Version**: 1.0  
**Status**: CANONICAL  
**Applies to**: All language-specific parser modules (codegraph-rust, codegraph-python, codegraph-js, etc.)

---

## Table of Contents

1. [Mission & Principles](#mission--principles)
2. [Module Naming & Structure](#module-naming--structure)
3. [Core API Contract](#core-api-contract)
4. [Common Dependencies](#common-dependencies)
5. [Entity Extraction Standards](#entity-extraction-standards)
6. [Relationship Tracking](#relationship-tracking)
7. [Testing Requirements](#testing-requirements)
8. [Documentation Standards](#documentation-standards)
9. [Error Handling](#error-handling)
10. [Performance Standards](#performance-standards)
11. [Quality Checklist](#quality-checklist)

---

## Mission & Principles

### Mission Statement
**"Transform language-specific ASTs into standardized codegraph representations with zero ambiguity."**

### Core Principles

#### 🔌 Parser Agnostic (within reason)
- Use the **best** parser for each language (syn for Rust, ast for Python, acorn for JS)
- Favor maturity and community support over novelty
- Document why the parser was chosen

#### 🎯 Consistent API Surface
- ALL language modules MUST implement the same core API
- Users should be able to swap parsers with minimal code changes
- Configuration patterns should be identical across languages

#### 🧪 Test-Driven Development
- Tests MUST be written before implementation
- Minimum 90% code coverage
- Every exported function MUST have tests

#### 📊 Observable & Measurable
- Every parser reports parsing statistics
- Performance metrics are tracked
- Progress reporting for large codebases

#### 🛡️ Defensive & Safe
- Handle malformed/incomplete code gracefully
- Never panic in library code (only in examples)
- Always return `Result<T, E>` for fallible operations

#### 🚀 Performance First
- Target: 1000 files/second on average hardware
- Memory efficient (streaming when possible)
- Parallel processing where appropriate

---

## Module Naming & Structure

### Naming Convention

```
codegraph-{language}
```

**Examples:**
- `codegraph-rust`
- `codegraph-python`
- `codegraph-javascript` (NOT `codegraph-js`)
- `codegraph-typescript`
- `codegraph-go`
- `codegraph-java`
- `codegraph-csharp`

### Directory Structure (MANDATORY)

```
codegraph-{language}/
├── Cargo.toml                 # Package manifest
├── README.md                  # Quick start + examples
├── CHANGELOG.md              # Version history
├── LICENSE-MIT               # Dual license
├── LICENSE-APACHE            # Dual license
├── .gitignore
├── src/
│   ├── lib.rs                # Public API exports
│   ├── parser.rs             # Main parser implementation
│   ├── config.rs             # ParserConfig struct
│   ├── error.rs              # Error types
│   ├── extractor.rs          # AST → IR extraction
│   ├── builder.rs            # IR → codegraph building
│   ├── entities/             # Entity extraction modules
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   ├── function.rs
│   │   ├── class.rs
│   │   └── module.rs
│   ├── relationships/        # Relationship extraction
│   │   ├── mod.rs
│   │   ├── calls.rs
│   │   ├── imports.rs
│   │   └── inheritance.rs
│   └── visitor.rs            # AST visitor (if applicable)
├── tests/
│   ├── integration/
│   │   ├── basic_parsing.rs
│   │   ├── relationships.rs
│   │   └── error_handling.rs
│   └── fixtures/             # Sample source files
│       ├── simple.{ext}
│       ├── complex.{ext}
│       └── malformed.{ext}
├── examples/
│   ├── basic_parse.rs
│   ├── call_graph.rs
│   ├── dependency_analysis.rs
│   └── project_stats.rs
└── benches/                  # Benchmarks
    └── parsing.rs
```

---

## Core API Contract

### MANDATORY: All parsers MUST implement this exact API

```rust
// src/lib.rs

use codegraph::{CodeGraph, NodeId};
use std::path::{Path, PathBuf};

/// Main parser for {language} source code
pub struct Parser {
    config: ParserConfig,
}

impl Parser {
    /// Create a new parser with default configuration
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }
    
    /// Create a parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }
    
    /// Parse a single source file and add entities to the graph
    ///
    /// # Arguments
    /// * `graph` - Mutable reference to the CodeGraph
    /// * `file_path` - Path to the source file
    ///
    /// # Returns
    /// `FileInfo` containing node IDs for all extracted entities
    ///
    /// # Errors
    /// Returns `ParseError` if the file cannot be read or parsed
    pub fn parse_file(
        &self,
        graph: &mut CodeGraph,
        file_path: impl AsRef<Path>,
    ) -> Result<FileInfo, ParseError> {
        todo!()
    }
    
    /// Parse an entire project (recursively walk directory tree)
    ///
    /// # Arguments
    /// * `graph` - Mutable reference to the CodeGraph
    /// * `project_root` - Root directory of the project
    ///
    /// # Returns
    /// `ProjectInfo` containing statistics and all file information
    ///
    /// # Errors
    /// Returns `ParseError` for I/O errors or parse failures
    pub fn parse_project(
        &self,
        graph: &mut CodeGraph,
        project_root: impl AsRef<Path>,
    ) -> Result<ProjectInfo, ParseError> {
        todo!()
    }
    
    /// Parse source code from a string (useful for testing)
    ///
    /// # Arguments
    /// * `graph` - Mutable reference to the CodeGraph
    /// * `source` - Source code as a string
    /// * `file_name` - Virtual file name for the source
    ///
    /// # Returns
    /// `FileInfo` containing node IDs for extracted entities
    pub fn parse_source(
        &self,
        graph: &mut CodeGraph,
        source: &str,
        file_name: &str,
    ) -> Result<FileInfo, ParseError> {
        todo!()
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
```

### MANDATORY: Configuration Structure

```rust
// src/config.rs

use serde::{Deserialize, Serialize};

/// Configuration for parser behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Include private/internal items (default: true)
    pub include_private: bool,
    
    /// Include test code (default: true)
    pub include_tests: bool,
    
    /// Parse documentation comments (default: true)
    pub parse_docs: bool,
    
    /// Maximum file size in bytes (default: 10MB)
    pub max_file_size: usize,
    
    /// Follow module/import declarations (default: true)
    pub follow_modules: bool,
    
    /// File extensions to parse (language-specific)
    pub file_extensions: Vec<String>,
    
    /// Directories to exclude (default: ["target", "node_modules", ".git"])
    pub exclude_dirs: Vec<String>,
    
    /// Enable parallel processing (default: true)
    pub parallel: bool,
    
    /// Number of threads for parallel processing (default: num_cpus)
    pub num_threads: Option<usize>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            include_private: true,
            include_tests: true,
            parse_docs: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            follow_modules: true,
            file_extensions: Self::default_extensions(),
            exclude_dirs: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            parallel: true,
            num_threads: None, // Use num_cpus
        }
    }
}

impl ParserConfig {
    /// Returns default file extensions for this language
    fn default_extensions() -> Vec<String> {
        // LANGUAGE-SPECIFIC: Override in each implementation
        // Rust: vec!["rs"]
        // Python: vec!["py", "pyi"]
        // JavaScript: vec!["js", "jsx", "mjs"]
        todo!()
    }
}
```

### MANDATORY: Return Types

```rust
// src/lib.rs (continued)

use codegraph::NodeId;
use std::path::PathBuf;
use std::time::Duration;

/// Information about a parsed file
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Node ID of the file in the graph
    pub file_id: NodeId,
    
    /// Path to the file
    pub file_path: PathBuf,
    
    /// Node IDs of all functions
    pub functions: Vec<NodeId>,
    
    /// Node IDs of all classes/structs
    pub classes: Vec<NodeId>,
    
    /// Node IDs of all traits/interfaces
    pub traits: Vec<NodeId>,
    
    /// Node IDs of all modules/namespaces
    pub modules: Vec<NodeId>,
    
    /// Number of lines parsed
    pub line_count: usize,
    
    /// Parsing duration
    pub parse_time: Duration,
}

/// Information about a parsed project
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// Root directory of the project
    pub project_root: PathBuf,
    
    /// Information for each parsed file
    pub files: Vec<FileInfo>,
    
    /// Total number of functions across all files
    pub total_functions: usize,
    
    /// Total number of classes/structs
    pub total_classes: usize,
    
    /// Total number of traits/interfaces
    pub total_traits: usize,
    
    /// Total number of modules
    pub total_modules: usize,
    
    /// Total lines of code parsed
    pub total_lines: usize,
    
    /// Total parsing time
    pub total_time: Duration,
    
    /// Files that failed to parse (with error messages)
    pub failed_files: Vec<(PathBuf, String)>,
}

impl ProjectInfo {
    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.files.len() + self.failed_files.len();
        if total == 0 {
            return 100.0;
        }
        (self.files.len() as f64 / total as f64) * 100.0
    }
    
    /// Get average parse time per file
    pub fn avg_parse_time(&self) -> Duration {
        if self.files.is_empty() {
            return Duration::from_secs(0);
        }
        self.total_time / self.files.len() as u32
    }
}
```

### MANDATORY: Error Types

```rust
// src/error.rs

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    /// I/O error reading file
    #[error("Failed to read file {path}: {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    
    /// File is too large
    #[error("File {path} exceeds maximum size of {max_size} bytes")]
    FileTooLarge {
        path: PathBuf,
        max_size: usize,
    },
    
    /// Parse error from underlying parser
    #[error("Failed to parse {file}: {message}")]
    SyntaxError {
        file: String,
        line: Option<usize>,
        column: Option<usize>,
        message: String,
    },
    
    /// Error building graph
    #[error("Failed to build graph: {0}")]
    GraphError(#[from] codegraph::GraphError),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    
    /// Unsupported language feature
    #[error("Unsupported language feature in {file}: {feature}")]
    UnsupportedFeature {
        file: String,
        feature: String,
    },
}

pub type Result<T> = std::result::Result<T, ParseError>;
```

---

## Common Dependencies

### MANDATORY Dependencies (all parsers)

```toml
[dependencies]
# Core graph database
codegraph = "0.1.1"

# Error handling
thiserror = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# File system traversal
walkdir = "2.0"

# Parallel processing
rayon = "1.10"

# Logging
tracing = "0.1"

[dev-dependencies]
# Temporary directories for tests
tempfile = "3.0"

# Benchmarking
criterion = "0.5"

# Test utilities
pretty_assertions = "1.4"
```

### Language-Specific Parser Dependencies

**Choose ONE primary parser per language:**

```toml
# Rust
[dependencies]
syn = { version = "2.0", features = ["full", "visit", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"

# Python
[dependencies]
rustpython-parser = "0.3"
# OR
tree-sitter = "0.20"
tree-sitter-python = "0.20"

# JavaScript/TypeScript
[dependencies]
swc_ecma_parser = "0.140"
swc_ecma_ast = "0.110"
swc_common = "0.33"
# OR
tree-sitter = "0.20"
tree-sitter-javascript = "0.20"

# Go
[dependencies]
tree-sitter = "0.20"
tree-sitter-go = "0.20"

# Java
[dependencies]
tree-sitter = "0.20"
tree-sitter-java = "0.20"
```

---

## Entity Extraction Standards

### Intermediate Representation (IR)

Every parser MUST use an IR layer between AST and codegraph:

```rust
// src/extractor.rs

/// Intermediate representation of parsed code
pub struct CodeIR {
    /// Path to the source file
    pub file_path: PathBuf,
    
    /// Extracted functions
    pub functions: Vec<FunctionEntity>,
    
    /// Extracted classes/structs
    pub classes: Vec<ClassEntity>,
    
    /// Extracted traits/interfaces
    pub traits: Vec<TraitEntity>,
    
    /// Extracted modules/namespaces
    pub modules: Vec<ModuleEntity>,
    
    /// Extracted imports
    pub imports: Vec<ImportEntity>,
    
    /// Function call relationships
    pub calls: Vec<CallRelation>,
    
    /// Inheritance relationships
    pub inheritance: Vec<InheritanceRelation>,
    
    /// Implementation relationships (trait/interface)
    pub implementations: Vec<ImplementationRelation>,
}
```

### MANDATORY: Function Entity Structure

```rust
// src/entities/function.rs

/// Represents a function/method in any language
#[derive(Debug, Clone)]
pub struct FunctionEntity {
    /// Function name
    pub name: String,
    
    /// Full signature (including parameters and return type)
    pub signature: String,
    
    /// Visibility (public, private, protected, internal, etc.)
    pub visibility: String,
    
    /// Starting line number (1-indexed)
    pub line_start: usize,
    
    /// Ending line number (1-indexed)
    pub line_end: usize,
    
    /// Is this an async/coroutine function?
    pub is_async: bool,
    
    /// Is this a test function?
    pub is_test: bool,
    
    /// Parameters
    pub parameters: Vec<Parameter>,
    
    /// Return type (if statically known)
    pub return_type: Option<String>,
    
    /// Documentation comment
    pub doc_comment: Option<String>,
    
    /// Parent class/struct (for methods)
    pub parent: Option<String>,
    
    /// Language-specific attributes/decorators
    pub attributes: Vec<String>,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    
    /// Parameter type (if statically known)
    pub type_annotation: Option<String>,
    
    /// Default value (if any)
    pub default_value: Option<String>,
}
```

### MANDATORY: Class Entity Structure

```rust
// src/entities/class.rs

/// Represents a class/struct in any language
#[derive(Debug, Clone)]
pub struct ClassEntity {
    /// Class name
    pub name: String,
    
    /// Visibility
    pub visibility: String,
    
    /// Starting line number
    pub line_start: usize,
    
    /// Ending line number
    pub line_end: usize,
    
    /// Is this an abstract class?
    pub is_abstract: bool,
    
    /// Base classes (inheritance)
    pub base_classes: Vec<String>,
    
    /// Implemented traits/interfaces
    pub implemented_traits: Vec<String>,
    
    /// Methods (function names)
    pub methods: Vec<String>,
    
    /// Fields/properties
    pub fields: Vec<Field>,
    
    /// Documentation comment
    pub doc_comment: Option<String>,
    
    /// Language-specific attributes/decorators
    pub attributes: Vec<String>,
}

/// Class field/property
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: String,
    pub is_static: bool,
}
```

---

## Relationship Tracking

### MANDATORY: Relationship Types

All parsers MUST track these relationships:

```rust
// src/relationships/mod.rs

/// Function call relationship
#[derive(Debug, Clone)]
pub struct CallRelation {
    /// Name of the calling function
    pub caller: String,
    
    /// Name of the called function
    pub callee: String,
    
    /// Line number where the call occurs
    pub line: usize,
    
    /// Is this a method call?
    pub is_method_call: bool,
}

/// Import/use relationship
#[derive(Debug, Clone)]
pub struct ImportEntity {
    /// What is being imported
    pub imported_items: Vec<String>,
    
    /// Source module/package
    pub from_module: String,
    
    /// Line number of import
    pub line: usize,
    
    /// Is this a wildcard import? (import *)
    pub is_wildcard: bool,
}

/// Inheritance relationship
#[derive(Debug, Clone)]
pub struct InheritanceRelation {
    /// Child class
    pub child: String,
    
    /// Parent class
    pub parent: String,
    
    /// Line number where inheritance is declared
    pub line: usize,
}

/// Trait/Interface implementation
#[derive(Debug, Clone)]
pub struct ImplementationRelation {
    /// Implementing class
    pub implementor: String,
    
    /// Trait/interface being implemented
    pub trait_name: String,
    
    /// Line number
    pub line: usize,
}
```

---

## Testing Requirements

### Test Coverage Requirements

- **Minimum 90% code coverage**
- Every public function MUST have at least one test
- Edge cases MUST be tested (empty files, malformed code, etc.)

### MANDATORY: Test Structure

```rust
// tests/integration/basic_parsing.rs

use codegraph_xxx::{Parser, ParserConfig};
use codegraph::CodeGraph;

#[test]
fn test_parse_simple_function() {
    let source = r#"
        // LANGUAGE-SPECIFIC SOURCE CODE HERE
    "#;
    
    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = Parser::new();
    
    let info = parser.parse_source(&mut graph, source, "test.ext").unwrap();
    
    assert_eq!(info.functions.len(), 1);
    
    let func = graph.get_node(info.functions[0]).unwrap();
    assert_eq!(func.properties.get_string("name"), Some("function_name"));
}

#[test]
fn test_parse_with_syntax_error() {
    let source = "invalid syntax here!!!";
    
    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = Parser::new();
    
    let result = parser.parse_source(&mut graph, source, "test.ext");
    assert!(result.is_err());
}

#[test]
fn test_exclude_tests_when_configured() {
    let source = r#"
        // Regular function
        // Test function
    "#;
    
    let mut graph = CodeGraph::in_memory().unwrap();
    let config = ParserConfig {
        include_tests: false,
        ..Default::default()
    };
    let parser = Parser::with_config(config);
    
    let info = parser.parse_source(&mut graph, source, "test.ext").unwrap();
    
    // Should only include the regular function
    assert_eq!(info.functions.len(), 1);
}
```

### MANDATORY: Test Fixtures

Provide at least these fixtures:

```
tests/fixtures/
├── simple.{ext}              # 1 function, 1 class
├── complex.{ext}             # Multiple entities, relationships
├── malformed.{ext}           # Syntax errors
├── empty.{ext}               # Empty file
├── only_comments.{ext}       # File with only comments
└── large.{ext}               # Large file (>1000 lines)
```

---

## Documentation Standards

### MANDATORY: README.md Structure

```markdown
# codegraph-{language}

{One-line description}

## Features

- ✅ Parses {language} source files into codegraph
- ✅ Extracts functions, classes, modules
- ✅ Tracks relationships (calls, imports, inheritance)
- ✅ Configurable parsing behavior
- ✅ Parallel processing support

## Installation

\`\`\`toml
[dependencies]
codegraph-{language} = "0.1"
codegraph = "0.1.1"
\`\`\`

## Quick Start

\`\`\`rust
use codegraph_{language}::Parser;
use codegraph::CodeGraph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = CodeGraph::open("./project.graph")?;
    let parser = Parser::new();
    
    let info = parser.parse_project(&mut graph, "./src")?;
    
    println!("Parsed {} files", info.files.len());
    Ok(())
}
\`\`\`

## Configuration

{Configuration examples}

## Examples

{Link to examples directory}

## License

Dual-licensed under MIT or Apache-2.0
```

### MANDATORY: Inline Documentation

Every public function MUST have:
- Summary line
- `# Arguments` section
- `# Returns` section
- `# Errors` section
- `# Examples` section (when applicable)

```rust
/// Parse a single source file and add entities to the graph.
///
/// This function reads the file at `file_path`, parses it using the
/// configured parser, extracts all entities (functions, classes, etc.),
/// and adds them to the provided `graph`.
///
/// # Arguments
///
/// * `graph` - Mutable reference to the CodeGraph where entities will be stored
/// * `file_path` - Path to the source file to parse
///
/// # Returns
///
/// Returns `FileInfo` containing node IDs for all extracted entities and
/// parsing statistics.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The file cannot be read (I/O error)
/// - The file exceeds the maximum size
/// - The file contains syntax errors
/// - Graph operations fail
///
/// # Examples
///
/// ```
/// use codegraph_rust::Parser;
/// use codegraph::CodeGraph;
///
/// let mut graph = CodeGraph::in_memory()?;
/// let parser = Parser::new();
/// let info = parser.parse_file(&mut graph, "src/main.rs")?;
/// println!("Found {} functions", info.functions.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_file(
    &self,
    graph: &mut CodeGraph,
    file_path: impl AsRef<Path>,
) -> Result<FileInfo> {
    // Implementation
}
```

---

## Error Handling

### Error Handling Principles

1. **Never panic in library code** - Always return `Result`
2. **Provide context** - Include file paths, line numbers in errors
3. **Fail gracefully** - Invalid syntax in one file shouldn't stop the entire parse
4. **Log warnings** - Use `tracing::warn!` for recoverable issues

### Example: Graceful Failure

```rust
pub fn parse_project(
    &self,
    graph: &mut CodeGraph,
    project_root: impl AsRef<Path>,
) -> Result<ProjectInfo> {
    let mut project_info = ProjectInfo::default();
    
    for entry in WalkDir::new(project_root) {
        let entry = entry?;
        let path = entry.path();
        
        if !self.should_parse_file(path) {
            continue;
        }
        
        // Try to parse, but don't fail entire project if one file fails
        match self.parse_file(graph, path) {
            Ok(file_info) => {
                project_info.files.push(file_info);
            }
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", path.display(), e);
                project_info.failed_files.push((path.to_path_buf(), e.to_string()));
            }
        }
    }
    
    Ok(project_info)
}
```

---

## Performance Standards

### Target Metrics

| Operation | Target Performance |
|-----------|-------------------|
| Parse single file (<1000 lines) | <10ms |
| Parse single file (1000-10000 lines) | <100ms |
| Parse project (100 files) | <1 second |
| Parse project (1000 files) | <10 seconds |
| Memory usage | <500MB for 1000 files |

### Performance Best Practices

1. **Use streaming parsers** when possible
2. **Enable parallel processing** for multi-file parsing
3. **Reuse allocations** - use object pools for frequently created objects
4. **Minimize graph operations** - batch inserts when possible
5. **Profile regularly** - use `cargo bench` to catch regressions

### MANDATORY: Benchmarks

```rust
// benches/parsing.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use codegraph_xxx::Parser;
use codegraph::CodeGraph;

fn bench_parse_small_file(c: &mut Criterion) {
    let source = include_str!("../tests/fixtures/simple.ext");
    
    c.bench_function("parse_small_file", |b| {
        b.iter(|| {
            let mut graph = CodeGraph::in_memory().unwrap();
            let parser = Parser::new();
            black_box(parser.parse_source(&mut graph, source, "test.ext").unwrap());
        });
    });
}

criterion_group!(benches, bench_parse_small_file);
criterion_main!(benches);
```

---

## Quality Checklist

Before releasing a new language parser, verify ALL of these:

### Code Quality
- [ ] No `unsafe` code (without justification)
- [ ] Zero clippy warnings with `clippy::pedantic`
- [ ] All functions are documented
- [ ] No unwrap() in library code (only examples/tests)
- [ ] Error messages are helpful and actionable

### Testing
- [ ] 90%+ code coverage
- [ ] All public APIs have tests
- [ ] Integration tests with real code samples
- [ ] Benchmark suite exists
- [ ] Tests pass on all platforms (Linux, macOS, Windows)

### Documentation
- [ ] README with quick start
- [ ] CHANGELOG with version history
- [ ] All public APIs documented
- [ ] Examples directory with 3+ working examples
- [ ] API reference published to docs.rs

### API Contract
- [ ] Implements `Parser` struct with standard API
- [ ] Implements `ParserConfig` with standard fields
- [ ] Returns `FileInfo` and `ProjectInfo` types
- [ ] Uses `ParseError` enum
- [ ] Supports `parse_file()`, `parse_project()`, `parse_source()`

### Performance
- [ ] Meets target parse times
- [ ] Memory usage is reasonable
- [ ] Parallel processing works correctly
- [ ] No memory leaks (verify with valgrind/instruments)

### Repository
- [ ] Dual licensed (MIT/Apache-2.0)
- [ ] CI/CD pipeline configured (GitHub Actions)
- [ ] Published to crates.io
- [ ] Tagged release with semantic versioning

---

## Appendix: Example Implementations

### Minimal Parser Implementation

```rust
// src/lib.rs - Absolute minimum to satisfy the API contract

pub mod config;
pub mod error;
mod extractor;
mod builder;

pub use config::ParserConfig;
pub use error::{ParseError, Result};

use codegraph::{CodeGraph, NodeId};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Parser {
    config: ParserConfig,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }
    
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }
    
    pub fn parse_file(
        &self,
        graph: &mut CodeGraph,
        file_path: impl AsRef<Path>,
    ) -> Result<FileInfo> {
        let start = Instant::now();
        let path = file_path.as_ref();
        
        // Read file
        let source = std::fs::read_to_string(path)
            .map_err(|e| ParseError::IoError {
                path: path.to_path_buf(),
                source: e,
            })?;
        
        // Check file size
        if source.len() > self.config.max_file_size {
            return Err(ParseError::FileTooLarge {
                path: path.to_path_buf(),
                max_size: self.config.max_file_size,
            });
        }
        
        // Parse and extract
        let ir = extractor::extract(&source, path)?;
        
        // Build graph
        let file_info = builder::build_graph(graph, &ir, &self.config)?;
        
        let parse_time = start.elapsed();
        Ok(FileInfo {
            parse_time,
            ..file_info
        })
    }
    
    pub fn parse_project(
        &self,
        graph: &mut CodeGraph,
        project_root: impl AsRef<Path>,
    ) -> Result<ProjectInfo> {
        let start = Instant::now();
        let root = project_root.as_ref();
        
        let mut project_info = ProjectInfo {
            project_root: root.to_path_buf(),
            files: Vec::new(),
            total_functions: 0,
            total_classes: 0,
            total_traits: 0,
            total_modules: 0,
            total_lines: 0,
            total_time: Duration::from_secs(0),
            failed_files: Vec::new(),
        };
        
        // Walk directory tree
        for entry in walkdir::WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path()))
        {
            let entry = entry.map_err(|e| ParseError::IoError {
                path: root.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let path = entry.path();
            if !self.should_parse_file(path) {
                continue;
            }
            
            match self.parse_file(graph, path) {
                Ok(file_info) => {
                    project_info.total_functions += file_info.functions.len();
                    project_info.total_classes += file_info.classes.len();
                    project_info.total_traits += file_info.traits.len();
                    project_info.total_modules += file_info.modules.len();
                    project_info.total_lines += file_info.line_count;
                    project_info.files.push(file_info);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", path.display(), e);
                    project_info.failed_files.push((path.to_path_buf(), e.to_string()));
                }
            }
        }
        
        project_info.total_time = start.elapsed();
        Ok(project_info)
    }
    
    pub fn parse_source(
        &self,
        graph: &mut CodeGraph,
        source: &str,
        file_name: &str,
    ) -> Result<FileInfo> {
        let start = Instant::now();
        
        let ir = extractor::extract(source, Path::new(file_name))?;
        let file_info = builder::build_graph(graph, &ir, &self.config)?;
        
        let parse_time = start.elapsed();
        Ok(FileInfo {
            parse_time,
            ..file_info
        })
    }
    
    fn should_parse_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            self.config.file_extensions.iter().any(|e| e == &*ext)
        } else {
            false
        }
    }
    
    fn should_exclude(&self, path: &Path) -> bool {
        path.components().any(|c| {
            if let std::path::Component::Normal(name) = c {
                let name = name.to_string_lossy();
                self.config.exclude_dirs.iter().any(|d| d == &*name)
            } else {
                false
            }
        })
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_id: NodeId,
    pub file_path: PathBuf,
    pub functions: Vec<NodeId>,
    pub classes: Vec<NodeId>,
    pub traits: Vec<NodeId>,
    pub modules: Vec<NodeId>,
    pub line_count: usize,
    pub parse_time: Duration,
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub project_root: PathBuf,
    pub files: Vec<FileInfo>,
    pub total_functions: usize,
    pub total_classes: usize,
    pub total_traits: usize,
    pub total_modules: usize,
    pub total_lines: usize,
    pub total_time: Duration,
    pub failed_files: Vec<(PathBuf, String)>,
}

impl ProjectInfo {
    pub fn success_rate(&self) -> f64 {
        let total = self.files.len() + self.failed_files.len();
        if total == 0 {
            return 100.0;
        }
        (self.files.len() as f64 / total as f64) * 100.0
    }
    
    pub fn avg_parse_time(&self) -> Duration {
        if self.files.is_empty() {
            return Duration::from_secs(0);
        }
        self.total_time / self.files.len() as u32
    }
}
```

---

## Version History

- **1.0** (2025-01-02): Initial constitutional document
  - Defined core API contract
  - Established testing requirements
  - Set performance standards
  - Created quality checklist

---

**This document is CANONICAL and MANDATORY for all language parser implementations.**

Any deviations must be:
1. Documented in the module's README
2. Justified with technical reasoning
3. Approved in code review

**When in doubt, follow this guide. Consistency across parsers is more important than individual optimization.**
