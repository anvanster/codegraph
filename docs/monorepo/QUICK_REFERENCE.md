# CodeGraph Quick Reference Guide

A visual cheat sheet for the CodeGraph monorepo architecture.

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────┐
│                    User Application                      │
│  (Analysis tools, LSP servers, CI/CD integrations)      │
└─────────────────────────────────────────────────────────┘
                         │
          ┌──────────────┴──────────────┐
          │                             │
          ▼                             ▼
┌──────────────────┐          ┌──────────────────┐
│  Language Parsers│          │  CodeGraph DB    │
│                  │          │                  │
│  • Python        │◄─────────┤  • Nodes         │
│  • Rust          │  Uses    │  • Edges         │
│  • TypeScript    │          │  • Queries       │
│  • Go            │          │  • RocksDB       │
│  • ...           │          │                  │
└──────────────────┘          └──────────────────┘
          │
          │ Implements
          ▼
┌──────────────────┐
│  Parser API      │
│                  │
│  • CodeParser    │
│  • Entities      │
│  • Relationships │
│  • Config        │
│  • Metrics       │
└──────────────────┘
```

---

## Crate Dependency Graph

```
        codegraph-python
              │
              │ depends on
              ▼
        codegraph-parser-api
              │
              │ depends on
              ▼
            codegraph
              │
              │ depends on
              ▼
      ┌───────┴───────┬─────────┐
      ▼               ▼         ▼
   serde         thiserror   rocksdb
```

**Key Point:** No circular dependencies!

---

## Data Flow

### Parsing Pipeline

```
Source Code (.py, .rs, .ts)
        │
        ▼
[Language-Specific Parser]
        │ (tree-sitter, syn, swc, etc.)
        ▼
    Raw AST
        │
        ▼
 [AST Visitor/Walker]
        │
        ▼
  CodeIR (Intermediate Representation)
        │
        │ Contains:
        │ • FunctionEntity[]
        │ • ClassEntity[]
        │ • CallRelation[]
        │ • ImportRelation[]
        ▼
[IR → Graph Mapper]
        │
        ▼
CodeGraph Database
        │
        │ Stores as:
        │ • Nodes (entities)
        │ • Edges (relationships)
        ▼
   [Query API]
        │
        ▼
    Results
```

---

## Entity Type Mapping

### Cross-Language Mapping

| Concept | Python | Rust | TypeScript | Go |
|---------|--------|------|------------|-----|
| **Function** | `def` | `fn` | `function` | `func` |
| **Class** | `class` | `struct`/`enum` | `class` | `struct` |
| **Method** | class method | `impl` method | class method | receiver method |
| **Interface** | `Protocol` | `trait` | `interface` | `interface` |
| **Module** | `.py` file | `mod` | `.ts` file | `package` |
| **Field** | class attr | struct field | class property | struct field |
| **Import** | `import`/`from` | `use` | `import`/`require` | `import` |

### Entity Hierarchy

```
ModuleEntity (File)
  │
  ├─► FunctionEntity
  │     └─► Parameters
  │
  ├─► ClassEntity
  │     ├─► Methods (FunctionEntity[])
  │     └─► Fields
  │
  └─► TraitEntity
        └─► RequiredMethods (FunctionEntity[])
```

---

## Command Cheat Sheet

### Build Commands

| Command | Description |
|---------|-------------|
| `cargo build --workspace` | Build all crates |
| `cargo build -p codegraph-python` | Build specific crate |
| `cargo build --release` | Build with optimizations |
| `cargo clean` | Remove build artifacts |

### Test Commands

| Command | Description |
|---------|-------------|
| `cargo test --workspace` | Run all tests |
| `cargo test -p codegraph-python` | Test specific crate |
| `cargo test test_name` | Run specific test |
| `cargo test -- --nocapture` | Show println! output |
| `cargo test --doc` | Run doc tests |

### Code Quality

| Command | Description |
|---------|-------------|
| `cargo fmt --all` | Format code |
| `cargo clippy --workspace` | Run linter |
| `cargo doc --workspace --open` | Generate docs |
| `cargo tarpaulin --workspace` | Code coverage |

### Publishing

| Command | Description |
|---------|-------------|
| `cargo publish --dry-run` | Test publish |
| `cargo publish` | Publish to crates.io |
| `cargo package --list` | List files to publish |

---

## File Structure Template

### New Parser Crate

```
crates/codegraph-{language}/
├── src/
│   ├── lib.rs              # Public API
│   ├── parser.rs           # CodeParser impl
│   ├── visitor.rs          # AST walking
│   ├── mapper.rs           # AST → IR mapping
│   └── tests.rs            # Unit tests
├── tests/
│   └── integration_test.rs # Integration tests
├── benches/
│   └── parse_bench.rs      # Benchmarks
├── examples/
│   └── basic_usage.rs      # Usage examples
├── Cargo.toml
└── README.md
```

### Parser Implementation Pattern

```rust
// src/parser.rs

use codegraph_parser_api::{
    CodeParser, FileInfo, ParserConfig, ParserError, ParserMetrics,
};
use codegraph::CodeGraph;
use std::path::Path;

pub struct LanguageParser {
    config: ParserConfig,
    metrics: ParserMetrics,
}

impl LanguageParser {
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
}

impl CodeParser for LanguageParser {
    fn language(&self) -> &str {
        "language-name"
    }

    fn file_extensions(&self) -> &[&str] {
        &[".ext"]
    }

    fn parse_file(
        &self,
        path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        // 1. Read file
        // 2. Parse to AST
        // 3. Extract to CodeIR
        // 4. Insert into graph
        // 5. Return FileInfo
        todo!()
    }

    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError> {
        // Similar to parse_file
        todo!()
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

---

## Common Patterns

### Pattern 1: Extract Entities to IR

```rust
use codegraph_parser_api::{CodeIR, FunctionEntity, ClassEntity};

fn extract_to_ir(ast: &AST, file_path: &Path) -> CodeIR {
    let mut ir = CodeIR::new(file_path.to_path_buf());
    
    for node in ast.nodes() {
        match node {
            Node::Function(f) => {
                let func = FunctionEntity::new(f.name, f.line_start, f.line_end)
                    .with_signature(f.signature)
                    .with_visibility(f.visibility);
                ir.add_function(func);
            }
            Node::Class(c) => {
                let class = ClassEntity::new(c.name, c.line_start, c.line_end)
                    .with_bases(c.bases);
                ir.add_class(class);
            }
            // ... other nodes
        }
    }
    
    ir
}
```

### Pattern 2: Batch Insert to Graph

```rust
use codegraph::{CodeGraph, Node, Edge, NodeType, EdgeType};

fn ir_to_graph(ir: CodeIR, graph: &mut CodeGraph) -> Result<FileInfo> {
    // 1. Create file node
    let file_node = Node::new(NodeType::File)
        .with_property("path", ir.file_path.display().to_string());
    let file_id = graph.add_node(file_node)?;
    
    // 2. Add function nodes
    let mut function_ids = Vec::new();
    for func in ir.functions {
        let func_node = Node::new(NodeType::Function)
            .with_property("name", func.name)
            .with_property("line_start", func.line_start);
        let func_id = graph.add_node(func_node)?;
        
        // Add edge: file contains function
        graph.add_edge(Edge::new(file_id, func_id, EdgeType::Contains))?;
        
        function_ids.push(func_id);
    }
    
    // 3. Add relationships
    for call in ir.calls {
        // Find caller and callee IDs
        // Add call edge
    }
    
    Ok(FileInfo {
        file_path: ir.file_path,
        file_id,
        functions: function_ids,
        // ...
    })
}
```

### Pattern 3: Parallel Parsing

```rust
use rayon::prelude::*;

fn parse_files_parallel(
    &self,
    paths: &[PathBuf],
    graph: &mut CodeGraph,
) -> Result<ProjectInfo> {
    // 1. Parse to IR in parallel (read-only, no graph access)
    let irs: Vec<CodeIR> = paths
        .par_iter()
        .filter_map(|path| {
            self.parse_to_ir(path).ok()
        })
        .collect();
    
    // 2. Insert to graph sequentially (requires mutable access)
    let mut files = Vec::new();
    for ir in irs {
        match self.ir_to_graph(ir, graph) {
            Ok(info) => files.push(info),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(ProjectInfo {
        files,
        // ...
    })
}
```

---

## Error Handling

### Standard Error Flow

```rust
use codegraph_parser_api::{ParserError, ParserResult};
use std::path::PathBuf;

// Reading file
let source = std::fs::read_to_string(path)
    .map_err(|e| ParserError::IoError(path.to_path_buf(), e))?;

// Parsing syntax
let ast = parse(source)
    .map_err(|e| ParserError::SyntaxError(
        path.to_path_buf(),
        e.line,
        e.column,
        e.message,
    ))?;

// File size check
if source.len() > config.max_file_size {
    return Err(ParserError::FileTooLarge(
        path.to_path_buf(),
        source.len(),
    ));
}

// Graph operations
graph.add_node(node)
    .map_err(|e| ParserError::GraphError(e.to_string()))?;
```

---

## Testing Checklist

### Unit Tests
- [ ] Entity creation
- [ ] Entity builders
- [ ] Config defaults
- [ ] Metrics calculations
- [ ] Error construction

### Integration Tests
- [ ] Parse simple file
- [ ] Parse complex file
- [ ] Parse entire directory
- [ ] Handle syntax errors
- [ ] Handle missing files
- [ ] Parallel parsing

### Performance Tests
- [ ] Parse 100 files <1s
- [ ] Parse 1K files <10s
- [ ] Parse 10K files <100s
- [ ] Memory usage <1GB

---

## Versioning Quick Reference

### Version Bumping Rules

| Change | Database | Parser API | Parsers |
|--------|----------|------------|---------|
| Fix bug | Patch | Patch | Patch |
| Add optional feature | Minor | Minor | Minor |
| Add required trait method | - | Minor | Major |
| Change trait signature | - | Major | Major |
| Remove deprecated API | - | Major | Major |

### Compatibility Matrix

```
Parser API 0.1.x ←─── Python 0.2.x
                 ├─── Rust 0.1.x
                 └─── TypeScript 0.1.x

Parser API 0.2.x ←─── Python 0.3.x (needs update)
                 ├─── Rust 0.2.x (needs update)
                 └─── TypeScript 0.2.x (needs update)
```

---

## Git Workflow

### Branch Naming

```
feature/add-go-parser
fix/python-async-parsing
docs/update-readme
refactor/entity-builder
test/add-integration-tests
chore/update-deps
```

### Commit Template

```
<type>(<scope>): <subject>

<body>

<footer>
```

Example:
```
feat(rust): Add async function support

Parse async fn declarations and mark functions with
is_async flag in FunctionEntity.

Closes #42
```

---

## Performance Optimization Checklist

- [ ] Profile with `cargo flamegraph`
- [ ] Check allocations with `heaptrack`
- [ ] Use `Cow` for strings when possible
- [ ] Batch graph insertions
- [ ] Use parallel parsing for >100 files
- [ ] Cache AST parsing results
- [ ] Avoid unnecessary clones
- [ ] Use references over owned values
- [ ] Benchmark before/after changes

---

## Troubleshooting Guide

### Problem: Build fails after updating parser-api

**Solution:**
```bash
cargo clean
cargo update -p codegraph-parser-api
cargo build --workspace
```

### Problem: Tests hang

**Solution:**
```bash
# Run with timeout
cargo test -- --test-threads=1 --timeout=10

# Check for deadlocks
RUST_BACKTRACE=1 cargo test
```

### Problem: Slow parsing

**Solution:**
```bash
# Profile
cargo flamegraph --bench parse_bench

# Check metrics
let metrics = parser.metrics();
println!("Avg time: {:?}", metrics.avg_parse_time());
```

### Problem: High memory usage

**Solution:**
```bash
# Track allocations
cargo build --release
heaptrack ./target/release/my-parser

# Check for leaks
valgrind --leak-check=full ./target/release/my-parser
```

---

## Resource Links

### Official Documentation
- [CodeGraph Crate](https://crates.io/crates/codegraph)
- [Parser API Crate](https://crates.io/crates/codegraph-parser-api)
- [Python Parser](https://crates.io/crates/codegraph-python)

### Rust Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Guide](https://doc.rust-lang.org/cargo/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Parser Resources
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [syn crate](https://docs.rs/syn/)
- [swc](https://swc.rs/)

---

## Quick Metrics

### Target Performance

```
┌─────────────────────┬──────────┬─────────┐
│ Benchmark           │ Target   │ Current │
├─────────────────────┼──────────┼─────────┤
│ Parse 1 file (100 LOC) │ <10ms  │   TBD   │
│ Parse 100 files     │ <1s      │   TBD   │
│ Parse 1K files      │ <10s     │   TBD   │
│ Parse 10K files     │ <100s    │   TBD   │
│ Memory (1K files)   │ <500MB   │   TBD   │
└─────────────────────┴──────────┴─────────┘
```

### Code Quality Targets

```
┌──────────────────────┬──────────┐
│ Metric               │ Target   │
├──────────────────────┼──────────┤
│ Test Coverage        │ >85%     │
│ Build Time (full)    │ <5 min   │
│ Test Time (all)      │ <2 min   │
│ Clippy Warnings      │ 0        │
│ Doc Coverage         │ 100%     │
└──────────────────────┴──────────┘
```

---

**Keep this guide handy for quick lookups during development!**

For detailed information, see:
- [ARCHITECTURE.md](ARCHITECTURE.md) - Full architecture details
- [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) - Step-by-step migration
- [PARSER_API_SPEC.md](PARSER_API_SPEC.md) - Complete API specification
- [MONOREPO_DEV_GUIDE.md](MONOREPO_DEV_GUIDE.md) - Development workflows
