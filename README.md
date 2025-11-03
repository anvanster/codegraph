# codegraph

[![Crates.io](https://img.shields.io/crates/v/codegraph.svg)](https://crates.io/crates/codegraph)
[![Documentation](https://docs.rs/codegraph/badge.svg)](https://docs.rs/codegraph)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A fast, reliable, and flexible graph database optimized for storing and querying code relationships.

## Mission

codegraph provides a fast, reliable, and flexible graph database optimized for storing and querying code relationships, enabling tool builders to focus on analysis logic rather than infrastructure.

## Core Principles

### 🔌 Parser Agnostic
**"Bring your own parser, we'll handle the graph."**

codegraph does NOT include built-in language parsers. You integrate your own parsers (tree-sitter, syn, swc, etc.), and we provide the storage and query infrastructure.

### ⚡ Performance First
**"Sub-100ms queries or it didn't happen."**

- Single node lookup: <1ms
- Neighbor query: <10ms
- Graph traversal (depth=5): <50ms
- 100K node graphs are practical

### 🧪 Test-Driven Development
**"If it's not tested, it's broken."**

- 115 tests with comprehensive coverage (39 lib + 70 integration/unit + 6 doctests)
- Every public API is tested
- Benchmarks ensure performance targets
- 85% test coverage (983/1158 lines)

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

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
codegraph = "0.1.1"
```

### Basic Usage

```rust
use codegraph::{CodeGraph, Node, NodeType, Edge, EdgeType};
use std::path::Path;

// Create a persistent graph
let mut graph = CodeGraph::open(Path::new("./my_project.graph"))?;

// Add a file node (explicit, no magic)
let file_id = graph.add_file(Path::new("src/main.rs"), "rust")?;

// Add a function node
let mut func_node = Node::new(NodeType::Function);
func_node.set_property("name", serde_json::json!("main"));
func_node.set_property("line", serde_json::json!(10));
let func_id = graph.add_node(func_node)?;

// Create a relationship (file contains function)
let edge = Edge::new(file_id, func_id, EdgeType::Contains);
graph.add_edge(edge)?;

// Query the graph
let neighbors = graph.get_neighbors(&file_id)?;
println!("File contains {} entities", neighbors.len());
```

### Parser Integration Example

```rust
// Example with tree-sitter (you provide the parser)
use tree_sitter::{Parser, Language};

extern "C" { fn tree_sitter_rust() -> Language; }

let mut parser = Parser::new();
parser.set_language(unsafe { tree_sitter_rust() }).unwrap();

let source_code = std::fs::read_to_string("src/main.rs")?;
let tree = parser.parse(&source_code, None).unwrap();

// You extract entities from the AST
// codegraph stores the relationships
let mut graph = CodeGraph::open("./project.graph")?;
let file_id = graph.add_file("src/main.rs", "rust")?;

// Walk the tree and add nodes/edges as you see fit
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

## What We Are (and Aren't)

### We Are ✅
- A graph database optimized for code relationships
- A storage layer for tool builders
- Language-agnostic
- Production-ready

### We Are Not ❌
- A parser (no AST extraction)
- A semantic analyzer (no type inference)
- An IDE integration (no LSP server)
- A complete framework (you build the analysis logic)

## Performance Targets

| Operation | Target | Actual |
|-----------|--------|--------|
| Node lookup | <1ms | ✅ ~7ns (1000x better!) |
| Neighbor query | <10ms | ✅ ~410ns - 40µs |
| BFS traversal (depth=5) | <50ms | ✅ ~5ms |
| Batch insert (10K nodes) | <500ms | ✅ ~7ms |
| 100K node + 500K edge load | <5s | ✅ ~3.3s |

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Documentation

```bash
cargo doc --open
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy -- -D warnings

# Check test coverage
cargo tarpaulin

# Run all CI checks locally (recommended before pushing)
./scripts/ci-checks.sh
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

### v0.2-0.5 (Near-term)
- [ ] Query language improvements
- [ ] More export formats (GraphML, Cypher)
- [ ] Performance optimizations
- [ ] First-party parser helper crates

### v0.6-0.9 (Medium-term)
- [ ] Incremental updates
- [ ] Change tracking
- [ ] Statistics and metrics API
- [ ] CLI tool

### v1.0+ (Long-term)
- [ ] Schema validation
- [ ] Full-text search integration
- [ ] Compression options
- [ ] Distributed graphs (maybe)

## Acknowledgments

This project draws inspiration from:
- Rust Language governance model
- SQLite's reliability principles
- Redis project philosophy
- Kubernetes governance structure

---

Built with ❤️ in Rust
