# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-11-02

### Added

#### Core Graph Database (User Story 1)
- Persistent graph storage using RocksDB backend
- In-memory storage backend for testing
- Node and Edge operations with property maps
- Batch operations for nodes and edges
- Neighbor queries with directional support (Outgoing, Incoming, Both)
- Thread-safe operations with atomic counters
- Property system with type-safe getters (String, Int, Float, Bool, Lists)
- Support for 8 NodeTypes: CodeFile, Function, Class, Module, Variable, Type, Interface, Generic
- Support for 11 EdgeTypes: Imports, ImportsFrom, Contains, Calls, Invokes, Instantiates, Extends, Implements, Uses, Defines, References

#### Code-Specific Helpers (User Story 2)
- `add_file()`, `add_function()`, `add_class()`, `add_method()`, `add_module()` - Create code entities
- `add_function_with_metadata()` - Support for visibility, async, test markers
- `add_call()`, `add_import()` - Create relationships with metadata
- `get_callers()`, `get_callees()` - Query function call relationships
- `get_functions_in_file()` - List all functions in a file
- `get_file_dependencies()`, `get_file_dependents()` - File dependency queries

#### Query Builder (User Story 3)
- Fluent query interface with method chaining
- Filters: `node_type()`, `in_file()`, `file_pattern()`, `property()`, `name_contains()`, `custom()`
- Query execution: `execute()`, `count()`, `exists()`
- Query optimization (node_type filter first, then properties, then custom)
- Support for glob patterns in file matching
- Performance: Sub-millisecond queries on 10K node graphs

#### Export Formats (User Story 4)
- **DOT format**: Graphviz visualization with customizable colors and labels
- **JSON format**: D3.js-compatible nodes/links format
- **CSV format**: Separate exports for nodes and edges
- **RDF Triples**: N-Triples format for semantic web tools
- Size warnings for large graphs (>10K nodes)
- Filtered export support

#### Graph Algorithms (User Story 5)
- Breadth-First Search (BFS) with depth limiting
- Depth-First Search (DFS) with iterative implementation
- Tarjan's Strongly Connected Components (cycle detection)
- Path finding: `find_all_paths()` between nodes
- Transitive dependency analysis: `transitive_dependencies()`, `transitive_dependents()`
- Call chain analysis: `call_chain()` between functions
- Circular dependency detection: `circular_deps()`
- All algorithms handle cycles gracefully

### Performance

- **Node lookup**: ~7 ns (1000x better than 1ms target)
- **Neighbor queries**: 
  - 10 neighbors: ~410 ns
  - 100 neighbors: ~4 µs
  - 1000 neighbors: ~40 µs
- **Batch insert**: 
  - 100 nodes: ~482 µs
  - 1000 nodes: ~968 µs
  - 10,000 nodes: ~7 ms
- **Large graphs**: Successfully handles 100K nodes + 500K edges in 3.3s

### Testing

- 115 total tests (39 lib + 70 integration/unit + 6 doctests)
- 97% test coverage on core modules
- TDD methodology used throughout
- Integration tests for RocksDB persistence, crash recovery, large graphs
- Comprehensive benchmark suite

### Code Quality

- No unsafe code (enforced with `#![deny(unsafe_code)]`)
- All clippy pedantic lints addressed
- Comprehensive API documentation with examples
- Module-level documentation for all modules

### Examples

- `basic_usage.rs` - Core graph operations
- `call_graph.rs` - Function call analysis
- `dependency_tree.rs` - File dependency analysis
- `impact_analysis.rs` - Complex query patterns
- `visualize.rs` - All export formats

### Documentation

- Complete API documentation for all public interfaces
- Architecture overview and design principles
- Quickstart guide with common patterns
- Contributing guidelines

[0.1.1]: https://github.com/anvanster/codegraph/releases/tag/v0.1.1
