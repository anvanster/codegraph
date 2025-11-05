# codegraph Workspace

[![Crates.io](https://img.shields.io/crates/v/codegraph.svg)](https://crates.io/crates/codegraph)
[![Documentation](https://docs.rs/codegraph/badge.svg)](https://docs.rs/codegraph)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/anvanster/codegraph/workflows/CI/badge.svg)](https://github.com/anvanster/codegraph/actions)

A fast, reliable, and flexible graph database optimized for storing and querying code relationships, with production-ready language parsers.

## Mission

The codegraph ecosystem provides:
- **codegraph**: Fast graph database for storing code relationships
- **codegraph-parser-api**: Unified parser interface and types
- **Language parsers**: Production-ready parsers for Python, Rust, TypeScript, and Go
- A complete solution for building code analysis tools

## Workspace Crates

| Crate | Version | Description | Status |
|-------|---------|-------------|--------|
| [`codegraph`](crates/codegraph) | 0.1.1 | Graph database core | ✅ Stable |
| [`codegraph-parser-api`](crates/codegraph-parser-api) | 0.1.0 | Parser trait and types | ✅ Stable |
| [`codegraph-python`](crates/codegraph-python) | 0.2.0 | Python parser | ✅ Stable |
| [`codegraph-rust`](crates/codegraph-rust) | 0.1.0 | Rust parser | ✅ Stable |
| [`codegraph-typescript`](crates/codegraph-typescript) | 0.1.0 | TypeScript/JavaScript parser | ✅ Stable |
| [`codegraph-go`](crates/codegraph-go) | 0.1.0 | Go parser | ✅ Stable |

## Quick Start

### Using the Complete Solution (Database + Parser)

```toml
[dependencies]
codegraph = "0.1.1"
codegraph-parser-api = "0.1.0"
codegraph-python = "0.2.0"  # or rust, typescript, go
```

```rust
use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_python::PythonParser;
use std::path::Path;

// Create parser and graph
let parser = PythonParser::new();
let mut graph = CodeGraph::open("./my_project.graph")?;

// Parse a file
let file_info = parser.parse_file(Path::new("main.py"), &mut graph)?;
println!("Parsed: {} functions, {} classes",
    file_info.functions.len(),
    file_info.classes.len()
);

// Parse entire project
let project_info = parser.parse_directory(Path::new("./src"), &mut graph)?;
println!("Parsed {} files in {:?}",
    project_info.files.len(),
    project_info.total_parse_time
);

// Query the graph
let neighbors = graph.get_neighbors(&file_info.file_id)?;
```

### Language-Specific Examples

<details>
<summary><b>Python Parser</b></summary>

```rust
use codegraph_python::PythonParser;
use codegraph_parser_api::{CodeParser, ParserConfig};

let config = ParserConfig {
    include_tests: false,      // Skip test_ functions
    include_private: false,     // Skip _private functions
    parallel: true,             // Use parallel parsing
    ..Default::default()
};

let parser = PythonParser::with_config(config);
let mut graph = CodeGraph::open("project.graph")?;

// Parse Python project
let info = parser.parse_directory(Path::new("./src"), &mut graph)?;
```
</details>

<details>
<summary><b>Rust Parser</b></summary>

```rust
use codegraph_rust::RustParser;
use codegraph_parser_api::CodeParser;

let parser = RustParser::new();
let mut graph = CodeGraph::open("project.graph")?;

// Parse Rust source
let info = parser.parse_file(Path::new("main.rs"), &mut graph)?;

// Extracts: functions, structs, enums, traits, impl blocks, use statements
```
</details>

<details>
<summary><b>TypeScript Parser</b></summary>

```rust
use codegraph_typescript::TypeScriptParser;
use codegraph_parser_api::CodeParser;

let parser = TypeScriptParser::new();
let mut graph = CodeGraph::open("project.graph")?;

// Supports .ts, .tsx, .js, .jsx files
let info = parser.parse_file(Path::new("App.tsx"), &mut graph)?;

// Extracts: functions, classes, interfaces, imports, JSX components
```
</details>

<details>
<summary><b>Go Parser</b></summary>

```rust
use codegraph_go::GoParser;
use codegraph_parser_api::CodeParser;

let parser = GoParser::new();
let mut graph = CodeGraph::open("project.graph")?;

// Parse Go package
let info = parser.parse_directory(Path::new("./pkg"), &mut graph)?;

// Extracts: functions, structs, interfaces, imports
```
</details>

## Core Principles

### 🔌 Unified Parser API
**"One trait to parse them all."**

All language parsers implement the same `CodeParser` trait, providing:
- Consistent API across languages
- Standardized entity types (functions, classes, imports)
- Uniform error handling and metrics
- Drop-in interchangeability

### ⚡ Performance First
**"Sub-100ms queries or it didn't happen."**

- Single node lookup: <1ms
- Neighbor query: <10ms
- Graph traversal (depth=5): <50ms
- 100K node graphs are practical

### 🧪 Test-Driven Development
**"If it's not tested, it's broken."**

- 421+ tests across entire workspace
- codegraph: 39 tests (85% coverage)
- parser-api: 12 tests with comprehensive entity validation
- Python: 111 tests (~90% coverage)
- Rust: 64 tests (40 unit + 24 integration)
- TypeScript: 63 tests (40 unit + 23 integration)
- Go: 54 tests (34 unit + 20 integration)
- Every public API tested with TDD methodology

### 🪄 Zero Magic
**"Explicit over implicit, always."**

- No global state
- No automatic file scanning
- No convention-over-configuration
- Explicit error handling (no panics in library code)
- No unsafe code

### 💾 Persistence is Primary
**"Graphs outlive processes."**

- RocksDB backend for production
- Crash-safe with write-ahead logging
- Atomic batch operations
- Memory backend for testing only

## Using Just the Database

If you want to use your own parsers:

```toml
[dependencies]
codegraph = "0.1.1"
```

```rust
use codegraph::{CodeGraph, Node, NodeType, Edge, EdgeType};
use std::path::Path;

// Create a persistent graph
let mut graph = CodeGraph::open(Path::new("./my_project.graph"))?;

// Add nodes manually
let file_id = graph.add_file(Path::new("src/main.rs"), "rust")?;
let func_id = graph.add_node(NodeType::Function, properties)?;

// Create relationships
graph.add_edge(file_id, func_id, EdgeType::Contains, properties)?;

// Query the graph
let neighbors = graph.get_neighbors(&file_id)?;
```

## Architecture

codegraph is organized in clear layers:

```
User Tools (parsers, analysis)
    ↓
Code Helpers (convenience API)
    ↓
Query Builder (fluent interface)
    ↓
Core Graph (nodes, edges, algorithms)
    ↓
Storage Backend (RocksDB, memory)
```

Each layer:
- Has well-defined boundaries
- Can be tested independently
- Doesn't leak abstractions
- Has minimal dependencies on upper layers

## Features

- **Persistent Storage**: Production-ready RocksDB backend
- **Type-Safe API**: Rust's type system prevents common errors
- **Schema-less Properties**: Flexible JSON properties on nodes and edges
- **Efficient Queries**: O(1) neighbor lookups with adjacency indexing
- **Explicit Operations**: No hidden behavior or magical conventions
- **Comprehensive Tests**: 85% test coverage (983/1158 lines)
- **Zero Unsafe Code**: Memory-safe by default

## What We Provide

### ✅ Complete Solution
- **Graph Database**: Fast, persistent storage for code relationships
- **Parser API**: Unified interface for all language parsers
- **Language Parsers**: Production-ready Python, Rust, TypeScript, and Go parsers
- **Analysis Foundation**: Building blocks for custom code analysis tools

### ❌ Out of Scope
- **Semantic Analysis**: No type inference or advanced static analysis
- **IDE Integration**: No LSP server or editor plugins
- **Build System**: No compilation or dependency resolution
- **Complete Framework**: You build the analysis logic, we provide the infrastructure

## Performance Targets

| Operation | Target | Actual |
|-----------|--------|--------|
| Node lookup | <1ms | ✅ ~7ns (1000x better!) |
| Neighbor query | <10ms | ✅ ~410ns - 40µs |
| BFS traversal (depth=5) | <50ms | ✅ ~5ms |
| Batch insert (10K nodes) | <500ms | ✅ ~7ms |
| 100K node + 500K edge load | <5s | ✅ ~3.3s |

## Development

### Workspace Commands

```bash
# Build all crates
cargo build --workspace --release

# Test all crates (421+ tests)
cargo test --workspace

# Test specific parser
cargo test -p codegraph-python
cargo test -p codegraph-rust

# Generate documentation for all crates
cargo doc --workspace --open
```

### Code Quality

```bash
# Format all code
cargo fmt --all

# Lint with clippy (all warnings as errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check test coverage
cargo tarpaulin --workspace --exclude-files "tests/*"

# Run individual crate checks
cargo clippy -p codegraph-go --all-targets --all-features -- -D warnings
```

### Individual Crate Development

```bash
# Work on specific parser
cd crates/codegraph-typescript
cargo test
cargo clippy -- -D warnings

# Add features to parser API
cd crates/codegraph-parser-api
cargo test
cargo doc --open
```

## Examples

See the [`examples/`](examples/) directory for complete examples:

- `basic_usage.rs` - Creating and querying a simple graph
- `call_graph.rs` - Function call analysis with syn integration
- `dependency_tree.rs` - File dependency and circular dependency analysis
- `impact_analysis.rs` - Complex query patterns for impact analysis
- `visualize.rs` - Exporting graphs to DOT, JSON, CSV, and RDF formats

## API Overview

### Core Operations

```rust
// Node operations
let node_id = graph.add_node(NodeType::Function, properties)?;
let node = graph.get_node(node_id)?;
graph.delete_node(node_id)?;

// Edge operations
let edge_id = graph.add_edge(source, target, EdgeType::Calls, properties)?;
let neighbors = graph.get_neighbors(node_id, Direction::Outgoing)?;

// Batch operations
graph.add_nodes_batch(nodes)?;
graph.add_edges_batch(edges)?;
```

### Helper Functions

```rust
use codegraph::helpers;

// Code-specific operations
let file_id = helpers::add_file(&mut graph, "main.rs", "rust")?;
let func_id = helpers::add_function(&mut graph, file_id, "main", 10, 20)?;
helpers::add_call(&mut graph, func1_id, func2_id, Some(15))?;

// Relationship queries
let callers = helpers::get_callers(&graph, func_id)?;
let deps = helpers::get_file_dependencies(&graph, file_id)?;
```

### Query Builder

```rust
// Fluent query interface
let results = graph.query()
    .node_type(NodeType::Function)
    .in_file("src/main.rs")
    .property("visibility", "public")
    .name_contains("test")
    .execute()?;
```

### Graph Algorithms

```rust
// Transitive analysis
let all_deps = helpers::transitive_dependencies(&graph, file_id, Some(5))?;
let all_dependents = helpers::transitive_dependents(&graph, file_id, None)?;

// Call chains
let paths = helpers::call_chain(&graph, from_func, to_func, Some(10))?;

// Circular dependencies
let cycles = helpers::circular_deps(&graph)?;
```

### Export Formats

```rust
use codegraph::export;

// Graphviz DOT
export::export_dot(&graph, &mut output)?;

// D3.js JSON
export::export_json(&graph, &mut output)?;

// CSV (nodes and edges)
export::export_csv_nodes(&graph, &mut output)?;
export::export_csv_edges(&graph, &mut output)?;

// RDF N-Triples
export::export_triples(&graph, &mut output)?;
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Before contributing, please:
1. Follow TDD methodology
2. Ensure all tests pass
3. Run `cargo fmt` and `cargo clippy`

## License

codegraph is licensed under the **Apache License 2.0**, which means:

✅ **You can:**
- Use in commercial products
- Modify and distribute
- Use in proprietary software

✅ **You must:**
- Include a copy of the license
- Disclose significant changes (in a CHANGES file)
- Include the patent grant notice

✅ **You can't:**
- Hold us liable
- Claim we endorse your product

**This is a truly open license.** There's no "gotcha" later where we switch 
to GPL or a commercial model. Apache-2.0 is forever.

## Code of Conduct

This project adheres to the Rust Code of Conduct. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **v0.x**: API may change between minor versions (with deprecation warnings)
- **v1.0+**: Stability guaranteed, breaking changes only in major versions

Current version: **0.1.1** (Initial release + formatting fixes)

## Support

- **Issues**: [GitHub Issues](https://github.com/anvanster/codegraph/issues)
- **Discussions**: [GitHub Discussions](https://github.com/anvanster/codegraph/discussions)

## Roadmap

### v0.2.0 (Current Release)
- [x] Unified parser API with standardized types
- [x] Python parser with comprehensive test coverage
- [x] Rust parser with full AST extraction
- [x] TypeScript/JavaScript parser with JSX/TSX support
- [x] Go parser with import tracking
- [x] 421+ tests across all crates

### v0.3-0.5 (Near-term)
- [ ] Query language improvements (graph patterns, filters)
- [ ] More export formats (GraphML, Cypher, Neo4j)
- [ ] Performance optimizations (batch operations, caching)
- [ ] Parser enhancements (semantic relationships, type information)

### v0.6-0.9 (Medium-term)
- [ ] Incremental updates (watch mode, delta parsing)
- [ ] Change tracking and diff analysis
- [ ] Statistics and metrics API
- [ ] CLI tool for parsing and querying
- [ ] Additional language parsers (Java, C++, C#)

### v1.0+ (Long-term)
- [ ] Schema validation and constraints
- [ ] Full-text search integration
- [ ] Compression options for large graphs
- [ ] Distributed graphs for multi-repo analysis

## Acknowledgments

This project draws inspiration from:
- Rust Language governance model
- SQLite's reliability principles
- Redis project philosophy
- Kubernetes governance structure

---

Built with ❤️ in Rust
