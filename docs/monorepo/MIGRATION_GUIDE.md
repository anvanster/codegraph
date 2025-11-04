# CodeGraph Migration Guide

## Overview

This guide walks through migrating from the current independent crates structure to a monorepo with shared `codegraph-parser-api`.

**Timeline:** 4 weeks  
**Current Version:** codegraph v0.1.1, codegraph-python v0.1.0  
**Target Version:** codegraph v0.1.1 (no changes), codegraph-parser-api v0.1.0, codegraph-python v0.2.0

---

## Prerequisites

- Rust 1.70+ installed
- Git repository access
- Local copies of existing crates:
  - `codegraph`
  - `codegraph-python`

---

## Phase 1: Create Monorepo Structure (Week 1, Day 1-2)

### Step 1.1: Create Workspace Directory

```bash
# Create new workspace directory
mkdir codegraph-workspace
cd codegraph-workspace

# Initialize git
git init
```

### Step 1.2: Create Workspace Cargo.toml

Create `Cargo.toml` at the root:

```toml
[workspace]
resolver = "2"

members = [
    "crates/codegraph",
    "crates/codegraph-parser-api",
    "crates/codegraph-python",
]

[workspace.package]
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/anvanster/codegraph"
authors = ["anvanster"]

[workspace.dependencies]
# Shared dependencies across all crates
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
rocksdb = "0.22"
log = "0.4"

# Internal crates
codegraph = { path = "crates/codegraph", version = "0.1.1" }
codegraph-parser-api = { path = "crates/codegraph-parser-api", version = "0.1.0" }
```

### Step 1.3: Create Crates Directory

```bash
mkdir crates
```

### Step 1.4: Copy Existing Codegraph Crate

```bash
# Copy existing codegraph crate
cp -r /path/to/existing/codegraph crates/codegraph

# Verify it builds
cd crates/codegraph
cargo build
cargo test
cd ../..
```

**Update `crates/codegraph/Cargo.toml`** to use workspace dependencies:

```toml
[package]
name = "codegraph"
version = "0.1.1"
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "A fast, reliable graph database optimized for storing code relationships"
keywords = ["graph", "code-analysis", "database"]

[dependencies]
rocksdb.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
log.workspace = true
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
tempfile = "3.0"
```

### Step 1.5: Verify Workspace Builds

```bash
# From workspace root
cargo build
cargo test
```

**Expected:** All tests pass, no compilation errors.

---

## Phase 2: Create Parser API Crate (Week 1, Day 3-5)

### Step 2.1: Create Crate Structure

```bash
cd crates
cargo new codegraph-parser-api --lib
cd ..
```

### Step 2.2: Set Up Cargo.toml

Edit `crates/codegraph-parser-api/Cargo.toml`:

```toml
[package]
name = "codegraph-parser-api"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Shared API and types for CodeGraph language parsers"
keywords = ["parser", "code-analysis", "ast"]
categories = ["parser-implementations"]

[dependencies]
codegraph.workspace = true
serde.workspace = true
thiserror.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

### Step 2.3: Create Module Structure

```bash
cd crates/codegraph-parser-api/src

# Create modules
mkdir entities relationships

# Create files
touch traits.rs config.rs metrics.rs errors.rs ir.rs
touch entities/mod.rs entities/function.rs entities/class.rs entities/module.rs entities/trait_.rs
touch relationships/mod.rs relationships/calls.rs relationships/imports.rs 
touch relationships/inheritance.rs relationships/implementations.rs
```

### Step 2.4: Copy Entity Types from codegraph-python

**From:** `codegraph-python/src/entities/*`  
**To:** `codegraph-parser-api/src/entities/*`

```bash
# This is done manually, copying and adapting the code
# Reference PARSER_API_SPEC.md for exact definitions
```

### Step 2.5: Implement Core Modules

Follow the specifications from `PARSER_API_SPEC.md`:

1. **src/errors.rs** - Define ParserError enum
2. **src/config.rs** - Define ParserConfig struct
3. **src/metrics.rs** - Define ParserMetrics struct
4. **src/entities/** - Copy and adapt from codegraph-python
5. **src/relationships/** - Copy and adapt from codegraph-python
6. **src/ir.rs** - Define CodeIR struct
7. **src/traits.rs** - Define CodeParser trait

### Step 2.6: Update lib.rs

Edit `crates/codegraph-parser-api/src/lib.rs`:

```rust
//! CodeGraph Parser API
//!
//! Shared trait and types for building CodeGraph language parsers.

pub mod config;
pub mod entities;
pub mod errors;
pub mod ir;
pub mod metrics;
pub mod relationships;
pub mod traits;

// Re-export commonly used types
pub use config::ParserConfig;
pub use entities::{ClassEntity, Field, FunctionEntity, ModuleEntity, Parameter, TraitEntity};
pub use errors::{ParserError, ParserResult};
pub use ir::CodeIR;
pub use metrics::ParserMetrics;
pub use relationships::{
    CallRelation, ImplementationRelation, ImportRelation, InheritanceRelation,
};
pub use traits::{CodeParser, FileInfo, ProjectInfo};
```

### Step 2.7: Write Tests

Create `crates/codegraph-parser-api/src/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_entity_builder() {
        let func = FunctionEntity::new("test_fn", 1, 10)
            .with_signature("fn test_fn() -> bool")
            .with_visibility("public")
            .test_fn();

        assert_eq!(func.name, "test_fn");
        assert_eq!(func.line_start, 1);
        assert_eq!(func.line_end, 10);
        assert!(func.is_test);
    }

    #[test]
    fn test_class_entity_builder() {
        let class = ClassEntity::new("MyClass", 5, 50)
            .with_bases(vec!["BaseClass".to_string()])
            .abstract_class();

        assert_eq!(class.name, "MyClass");
        assert_eq!(class.base_classes.len(), 1);
        assert!(class.is_abstract);
    }

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert!(!config.skip_private);
        assert!(!config.skip_tests);
        assert!(config.include_docs);
    }

    #[test]
    fn test_parser_metrics_calculations() {
        let mut metrics = ParserMetrics::default();
        metrics.files_attempted = 10;
        metrics.files_succeeded = 8;
        metrics.files_failed = 2;

        assert_eq!(metrics.success_rate(), 0.8);
    }

    #[test]
    fn test_code_ir_counts() {
        let mut ir = CodeIR::new(PathBuf::from("test.py"));
        ir.add_function(FunctionEntity::new("test", 1, 5));
        ir.add_class(ClassEntity::new("Test", 10, 20));

        assert_eq!(ir.entity_count(), 2);
    }
}
```

### Step 2.8: Verify API Crate Builds

```bash
# From workspace root
cargo build -p codegraph-parser-api
cargo test -p codegraph-parser-api
cargo doc -p codegraph-parser-api --open
```

**Expected:** 
- ✅ Builds successfully
- ✅ All tests pass
- ✅ Documentation generates

---

## Phase 3: Migrate codegraph-python (Week 2)

### Step 3.1: Copy Python Parser to Workspace

```bash
cp -r /path/to/existing/codegraph-python crates/codegraph-python
```

### Step 3.2: Update Cargo.toml

Edit `crates/codegraph-python/Cargo.toml`:

```toml
[package]
name = "codegraph-python"
version = "0.2.0"  # Bump minor version
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Python parser for CodeGraph"
keywords = ["python", "parser", "code-analysis"]

[dependencies]
# Use workspace dependencies
codegraph.workspace = true
codegraph-parser-api.workspace = true  # NEW
serde.workspace = true
thiserror.workspace = true

# Python-specific
rustpython-parser = "0.3"

[dev-dependencies]
tempfile = "3.0"
```

### Step 3.3: Remove Duplicate Entity Definitions

**Delete these files** (now in parser-api):
- `src/entities/function.rs`
- `src/entities/class.rs`
- `src/entities/file.rs` (becomes ModuleEntity)
- `src/entities/trait_.rs`
- `src/relationships/calls.rs`
- `src/relationships/imports.rs`
- `src/relationships/inheritance.rs`
- `src/relationships/implementations.rs`

**Update imports** to use parser-api types:

```rust
// Old
use crate::entities::{FunctionEntity, ClassEntity, ModuleEntity};

// New
use codegraph_parser_api::entities::{FunctionEntity, ClassEntity, ModuleEntity};
```

### Step 3.4: Update Extractor to Use CodeIR

Edit `crates/codegraph-python/src/extractor.rs`:

```rust
use codegraph_parser_api::{CodeIR, ParserError};  // Use API types
use std::path::Path;

pub fn extract(
    source: &str, 
    file_path: &Path, 
    config: &ParserConfig
) -> Result<CodeIR, ParserError> {
    let mut ir = CodeIR::new(file_path.to_path_buf());
    
    // Parse Python AST
    let ast = rustpython_parser::ast::Suite::parse(source, &file_path.display().to_string())
        .map_err(|e| ParserError::SyntaxError(
            file_path.to_path_buf(),
            0,
            0,
            e.to_string()
        ))?;

    // Extract entities
    for stmt in ast {
        match stmt {
            // ... extract functions, classes, etc into ir
        }
    }

    Ok(ir)
}
```

### Step 3.5: Implement CodeParser Trait

Create `crates/codegraph-python/src/parser_impl.rs`:

```rust
use codegraph_parser_api::{
    CodeParser, FileInfo, ParserConfig, ParserError, ParserMetrics, ProjectInfo,
};
use codegraph::{CodeGraph, NodeId};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Python language parser
pub struct PythonParser {
    config: ParserConfig,
    metrics: ParserMetrics,
}

impl PythonParser {
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            metrics: ParserMetrics::default(),
        }
    }

    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            metrics: ParserMetrics::default(),
        }
    }

    /// Helper: Convert IR to graph nodes/edges
    fn ir_to_graph(&self, ir: CodeIR, graph: &mut CodeGraph) -> Result<FileInfo, ParserError> {
        // Implementation from current builder.rs
        // ...
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
        
        // Read file
        let source = std::fs::read_to_string(path)
            .map_err(|e| ParserError::IoError(path.to_path_buf(), e))?;

        // Check file size
        if source.len() > self.config.max_file_size {
            return Err(ParserError::FileTooLarge(path.to_path_buf(), source.len()));
        }

        // Parse to IR
        let ir = crate::extractor::extract(&source, path, &self.config)?;

        // Insert into graph
        let mut file_info = self.ir_to_graph(ir, graph)?;
        file_info.parse_time = start.elapsed();

        Ok(file_info)
    }

    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        let start = Instant::now();

        let ir = crate::extractor::extract(source, file_path, &self.config)?;
        let mut file_info = self.ir_to_graph(ir, graph)?;
        file_info.parse_time = start.elapsed();

        Ok(file_info)
    }

    fn config(&self) -> &ParserConfig {
        &self.config
    }

    fn metrics(&self) -> ParserMetrics {
        self.metrics.clone()
    }

    fn reset_metrics(&mut self) {
        self.metrics = ParserMetrics::default();
    }
}
```

### Step 3.6: Update Public API (lib.rs)

Edit `crates/codegraph-python/src/lib.rs`:

```rust
//! Python parser for CodeGraph

// Re-export parser API types for convenience
pub use codegraph_parser_api::{
    CodeParser, FileInfo, ParserConfig, ParserError, ProjectInfo,
};

mod extractor;
mod parser_impl;
mod visitor;

pub use parser_impl::PythonParser;

// Keep old API for backward compatibility (deprecated)
#[deprecated(since = "0.2.0", note = "Use PythonParser with CodeParser trait")]
pub struct Parser {
    inner: PythonParser,
}

#[allow(deprecated)]
impl Parser {
    pub fn new() -> Self {
        Self {
            inner: PythonParser::new(),
        }
    }

    pub fn parse_file(
        &self,
        path: &std::path::Path,
        graph: &mut codegraph::CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        self.inner.parse_file(path, graph)
    }

    // ... other deprecated methods
}
```

### Step 3.7: Update Tests

Update all tests in `crates/codegraph-python/tests/`:

```rust
use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_python::PythonParser;

#[test]
fn test_parse_simple_function() {
    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = PythonParser::new();
    
    let source = r#"
def hello():
    print("Hello, world!")
"#;
    
    let result = parser.parse_source(
        source,
        std::path::Path::new("test.py"),
        &mut graph,
    );
    
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.functions.len(), 1);
}
```

### Step 3.8: Verify Migration

```bash
# Build
cargo build -p codegraph-python

# Run tests
cargo test -p codegraph-python

# Check for deprecation warnings
cargo clippy -p codegraph-python

# Generate docs
cargo doc -p codegraph-python --open
```

**Expected:**
- ✅ All tests pass
- ✅ Deprecation warnings for old API (expected)
- ✅ Documentation builds

### Step 3.9: Update README

Edit `crates/codegraph-python/README.md`:

Add migration notice:

```markdown
## Version 0.2.0 Migration

codegraph-python v0.2.0 uses the new `codegraph-parser-api` for consistency across parsers.

### New API (Recommended)
```rust
use codegraph_parser_api::CodeParser;
use codegraph_python::PythonParser;

let parser = PythonParser::new();
let info = parser.parse_file(path, &mut graph)?;
```

### Old API (Deprecated)
```rust
use codegraph_python::Parser;  // Deprecated

let parser = Parser::new();
let info = parser.parse_file(path, &mut graph)?;
```

The old API will be removed in v0.3.0.
```

---

## Phase 4: Create New Parsers (Week 3-4)

### Step 4.1: Create Rust Parser

```bash
cd crates
cargo new codegraph-rust --lib
```

**Cargo.toml:**
```toml
[package]
name = "codegraph-rust"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[dependencies]
codegraph.workspace = true
codegraph-parser-api.workspace = true
serde.workspace = true
thiserror.workspace = true

# Rust parser
syn = { version = "2.0", features = ["full", "visit"] }
```

**Implement:**
1. Create `src/parser.rs` implementing `CodeParser`
2. Create `src/visitor.rs` for AST walking
3. Create `src/mapper.rs` for AST → IR mapping
4. Write comprehensive tests

### Step 4.2: Create TypeScript Parser

```bash
cargo new codegraph-typescript --lib
```

Similar structure to Rust parser, using `swc_ecma_parser` or `tree-sitter`.

### Step 4.3: Add to Workspace

Update workspace `Cargo.toml`:

```toml
members = [
    "crates/codegraph",
    "crates/codegraph-parser-api",
    "crates/codegraph-python",
    "crates/codegraph-rust",      # NEW
    "crates/codegraph-typescript", # NEW
]
```

---

## Phase 5: CI/CD Setup (Week 4)

### Step 5.1: Update GitHub Actions

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
      
      - name: Build all crates
        run: cargo build --workspace
      
      - name: Run all tests
        run: cargo test --workspace
      
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
      
      - name: Check formatting
        run: cargo fmt --all -- --check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Generate coverage
        run: cargo tarpaulin --workspace --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

### Step 5.2: Add Publish Workflow

Create `.github/workflows/publish.yml`:

```yaml
name: Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
      
      - name: Publish codegraph-parser-api
        run: |
          cd crates/codegraph-parser-api
          cargo publish --token ${{ secrets.CARGO_TOKEN }}
      
      - name: Wait for crates.io
        run: sleep 30
      
      - name: Publish codegraph-python
        run: |
          cd crates/codegraph-python
          cargo publish --token ${{ secrets.CARGO_TOKEN }}
      
      # Repeat for other parsers...
```

---

## Phase 6: Documentation & Release (Week 4)

### Step 6.1: Create Workspace README

Create root `README.md`:

```markdown
# CodeGraph Workspace

A monorepo containing the CodeGraph database and language parsers.

## Structure

- **codegraph** - Core graph database
- **codegraph-parser-api** - Shared parser API
- **codegraph-python** - Python parser
- **codegraph-rust** - Rust parser
- **codegraph-typescript** - TypeScript parser

## Quick Start

```bash
# Clone
git clone https://github.com/anvanster/codegraph
cd codegraph

# Build all
cargo build --workspace

# Test all
cargo test --workspace
```

## Using Parsers

```rust
use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_python::PythonParser;

let mut graph = CodeGraph::open("./project.graph")?;
let parser = PythonParser::new();
parser.parse_directory(Path::new("./src"), &mut graph)?;
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)
```

### Step 6.2: Update CHANGELOG

Create `CHANGELOG.md`:

```markdown
# Changelog

## [0.2.0] - 2025-MM-DD

### Added
- **codegraph-parser-api**: New shared API for all parsers
- Monorepo structure for core parsers
- `CodeParser` trait for type-safe parser implementations

### Changed
- **codegraph-python**: Migrated to use `codegraph-parser-api`
- **codegraph-python**: Old `Parser` API deprecated (will be removed in 0.3.0)

### Migration Guide
See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for details.
```

### Step 6.3: Publish to crates.io

```bash
# 1. Tag the release
git tag v0.2.0
git push origin v0.2.0

# 2. Publish parser-api first
cd crates/codegraph-parser-api
cargo publish

# 3. Wait for crates.io indexing
sleep 30

# 4. Publish updated parsers
cd ../codegraph-python
cargo publish
```

---

## Verification Checklist

After migration, verify:

- [ ] Workspace builds: `cargo build --workspace`
- [ ] All tests pass: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Documentation builds: `cargo doc --workspace --open`
- [ ] CI/CD pipeline passes
- [ ] Published to crates.io successfully
- [ ] Old API still works (with deprecation warnings)
- [ ] New trait-based API works
- [ ] All examples updated

---

## Rollback Plan

If migration fails:

1. Keep existing crates published (v0.1.x)
2. Work in feature branch
3. Don't publish until fully tested
4. Maintain backward compatibility

---

## Common Issues & Solutions

### Issue: Circular Dependencies

**Problem:** Parser depends on database, database depends on parser-api

**Solution:** 
- Database should NOT depend on parser-api
- Only parsers depend on parser-api

### Issue: Version Conflicts

**Problem:** Workspace member versions out of sync

**Solution:**
```bash
# Use workspace version
[package]
version.workspace = true  # Instead of hardcoded version
```

### Issue: Tests Failing After Migration

**Problem:** Tests use old entity types

**Solution:**
```rust
// Update imports
use codegraph_parser_api::entities::FunctionEntity;  // Not from local crate
```

---

## Next Steps

After successful migration:

1. Create parsers for Rust, TypeScript, Go
2. Optimize batch insertion in database
3. Add parallel parsing benchmarks
4. Publish blog post about architecture
5. Update documentation site

---

## Support

Questions? Open an issue on GitHub or reach out in discussions.
