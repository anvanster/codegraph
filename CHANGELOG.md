# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Cyclomatic Complexity Analysis (`codegraph-parser-api` v0.2.0)
- New `ComplexityMetrics` struct for comprehensive complexity analysis
- `ComplexityBuilder` for incremental complexity tracking during AST traversal
- Metrics tracked: cyclomatic complexity, branches, loops, logical operators, max nesting depth, exception handlers, early returns
- Letter grading system (A-F) based on industry-standard thresholds
- Integration with `FunctionEntity` via optional `complexity` field
- 11 unit tests for complexity calculation

#### Python Parser Complexity (`codegraph-python` v0.3.0)
- Full AST-based cyclomatic complexity calculation
- Handles: if/elif/else, while, for, with, try/except, match statements
- List comprehension complexity tracking
- Boolean operator analysis (and, or)
- Nesting depth tracking
- 7 new complexity tests

#### TypeScript Parser Complexity (`codegraph-typescript` v0.3.0)
- Full AST-based cyclomatic complexity calculation using tree-sitter
- Handles: if/else, switch/case, ternary expressions, for/while/do loops
- Try/catch exception handling
- Logical operator analysis (&&, ||)
- Nesting depth tracking
- 9 new complexity tests

#### Graph Property Storage
- Complexity metrics stored as node properties in graph
- Properties: complexity, complexity_grade, complexity_branches, complexity_loops, complexity_logical_ops, complexity_nesting, complexity_exceptions, complexity_early_returns

### Changed
- `codegraph-parser-api` bumped to v0.2.0 (new complexity module)
- `codegraph-python` bumped to v0.3.0 (complexity integration)
- `codegraph-typescript` bumped to v0.3.0 (complexity integration)
- `codegraph-go` bumped to v0.1.2 (compatibility with new FunctionEntity)
- `codegraph-rust` bumped to v0.1.1 (compatibility with new FunctionEntity)
- Python builder now uses direct PropertyMap for richer function metadata
- TypeScript mapper includes complexity properties when available

## [0.2.0] - Previous Release

### Added

#### Monorepo Architecture
- Migrated to workspace structure with 6 crates
- Unified build system with shared dependencies
- Workspace-level configuration and metadata
- 421+ tests across all crates

#### Parser API (`codegraph-parser-api` v0.1.0)
- `CodeParser` trait defining standard parser interface
- Unified entity types: `FunctionEntity`, `ClassEntity`, `ModuleEntity`, `TraitEntity`
- Unified relationship types: `CallRelation`, `ImportRelation`, `InheritanceRelation`, `ImplementationRelation`
- `ParserConfig` with resource limits and feature flags
- `ParserMetrics` for tracking parse performance
- `CodeIR` intermediate representation for language-agnostic parsing
- Builder patterns for ergonomic entity construction
- Thread-safe design (Send + Sync)
- Comprehensive documentation and examples

#### Python Parser (`codegraph-python` v0.2.0)
- Complete migration to `CodeParser` trait
- 111 tests with ~90% test coverage
- Config-based filtering (include_tests, include_private)
- Comprehensive entity extraction (functions, classes, methods, imports)
- Async function support
- Property and decorator tracking
- Backward compatibility with deprecated API

#### Rust Parser (`codegraph-rust` v0.1.0)
- Full implementation of `CodeParser` trait
- 64 tests (40 unit + 24 integration)
- Extract functions, structs, enums, traits, impl blocks
- Individual use statement parsing with full paths
- Support for simple, aliased, grouped, glob, and nested imports
- Visibility tracking (public, private, crate, pub(crate))
- Generic and lifetime parameter extraction
- Method extraction from impl blocks

#### TypeScript Parser (`codegraph-typescript` v0.1.0)
- Full implementation of `CodeParser` trait
- 63 tests (40 unit + 23 integration)
- Support for .ts, .tsx, .js, .jsx files
- JSX/TSX support with file extension detection
- Extract functions, classes, interfaces, methods
- Individual import symbol extraction (named, default, namespace)
- Class method extraction with parameters
- Export tracking (default and named exports)
- Async/await function detection

#### Go Parser (`codegraph-go` v0.1.0)
- Full implementation of `CodeParser` trait
- 54 tests (34 unit + 20 integration)
- Extract functions, structs, interfaces, methods
- Individual import extraction from import blocks
- Support for named imports, dot imports, and aliased imports
- Method receiver tracking
- Pointer receiver detection
- Exported/unexported entity tracking

### Changed

#### codegraph-python Breaking Changes
- **BREAKING**: `Parser` struct deprecated in favor of `PythonParser`
- **BREAKING**: Entity types now come from `codegraph-parser-api`
- **BREAKING**: Config fields renamed (`skip_tests` → `include_tests`, `skip_private` → `include_private`)
- Migration path: Use `PythonParser::new()` instead of `Parser::new()`
- Old API still available with deprecation warnings

#### Test Coverage
- Workspace total: 421+ tests (from 115)
- codegraph: 39 tests (unchanged, 85% coverage)
- parser-api: 12 tests
- Python: 111 tests (~90% coverage)
- Rust: 64 tests
- TypeScript: 63 tests
- Go: 54 tests

#### Documentation
- Added workspace README with all parsers
- Updated ARCHITECTURE.md with completed phases
- Updated MIGRATION_GUIDE.md with completion status
- Added parser-specific usage examples
- Comprehensive API documentation for all parsers

### Fixed
- All clippy warnings across workspace resolved
- Formatting consistency with `cargo fmt`
- Python parser now respects ParserConfig filtering
- Import extraction provides symbol-level granularity (all parsers)
- TypeScript method extraction from classes
- Edge cases in deeply nested code structures

### Performance
- Parser metrics tracked across all implementations
- Parallel parsing support in ParserConfig
- Efficient AST traversal in all parsers
- Minimal allocations during parsing

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
