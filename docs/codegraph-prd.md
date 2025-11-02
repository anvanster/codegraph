Product Requirements Document: codegraph
Version: 1.0
Date: 2025-01-01
Status: Draft
Owner: Engineering

Executive Summary
codegraph is a specialized graph database for code analysis, designed to store and query relationships between code elements across any programming language. Unlike general-purpose graph databases, codegraph provides code-specific node types, relationship semantics, and query patterns optimized for dependency analysis, call graphs, and impact assessment.
Key Differentiation: We provide the storage and query layer—users bring their own parsers (tree-sitter, syn, swc, etc.). This reduces scope while enabling maximum flexibility.
Target Timeline: 3 months to v0.1.0
Target Market: Tool builders, researchers, architects, refactoring tool developers
Pricing Model: Open source core + premium integrations

Vision & Goals
Vision Statement

"The Rust-native storage foundation for every code analysis tool."

Primary Goals

Performance: Sub-100ms queries on graphs with 100K+ nodes
Persistence: Reliable RocksDB-backed storage with incremental updates
Simplicity: Intuitive API that feels natural for code relationships
Flexibility: Parser-agnostic design supporting any language
Quality: Production-ready with comprehensive test coverage

Non-Goals (v0.1.0)

❌ Built-in language parsers (users provide parsed data)
❌ Semantic analysis or type inference
❌ LSP server implementation
❌ Real-time synchronization with editors
❌ Cloud-hosted service


Target Users
Primary Personas
1. Tool Builder (Tom)

Building a custom refactoring tool for their company's codebase
Needs persistent storage of code relationships
Doesn't want to deal with graph database internals
Pain: Existing solutions (Neo4j, Kuzu) are overkill or have FFI overhead

2. Researcher (Reese)

Analyzing architectural patterns across 100+ open-source projects
Needs to query large code graphs efficiently
Wants to export data for visualization and analysis
Pain: No good way to store and query code relationships at scale

3. Platform Engineer (Pat)

Building internal developer tools for a large engineering org
Needs dependency tracking and impact analysis
Requires persistence and incremental updates
Pain: Building graph infrastructure is undifferentiated heavy lifting


Product Requirements
Core Features
1. Graph Database Foundation
FR-1.1: Node Management

Nodes represent code entities (files, functions, classes, etc.)
Each node has:

Unique ID (u64)
Type discriminator (enum: CodeFile, Function, Class, etc.)
Key-value properties (strings, ints, floats, bools, lists)


CRUD operations: create, read, update, delete
Batch operations for bulk inserts

FR-1.2: Edge Management

Edges represent relationships (imports, calls, contains, etc.)
Each edge has:

Unique ID (u64)
Source and target node IDs
Type discriminator (enum: Imports, Calls, Contains, etc.)
Optional properties for metadata


Directed graph model
Support for multiple edges between same nodes

FR-1.3: Property System

Flexible schema-less properties on nodes and edges
Supported types: String, Int, Float, Bool, StringList, IntList, Null
Type-safe access methods
Fluent builder API for construction

FR-1.4: Storage Backends

RocksDB Backend: Persistent on-disk storage

Column families for nodes/edges/metadata
Atomic batch writes
Crash recovery


Memory Backend: For testing and temporary graphs
Pluggable backend interface for future extensions

FR-1.5: Graph Algorithms

Breadth-First Search (BFS) with depth limit
Depth-First Search (DFS) with depth limit
Strongly Connected Components (Tarjan's algorithm for cycle detection)
Path finding (all paths between two nodes)
Neighbor queries with edge type filtering

2. Code-Specific Helpers
FR-2.1: Code Node Constructors
rustpub fn add_file(&mut self, path: &Path, language: &str) -> Result<NodeId>
pub fn add_function(&mut self, file_id: NodeId, name: &str, line_start: usize, line_end: usize) -> Result<NodeId>
pub fn add_class(&mut self, file_id: NodeId, name: &str, line_start: usize, line_end: usize) -> Result<NodeId>
pub fn add_module(&mut self, name: &str) -> Result<NodeId>
FR-2.2: Relationship Helpers
rustpub fn add_call(&mut self, caller: NodeId, callee: NodeId, line: usize) -> Result<EdgeId>
pub fn add_import(&mut self, from_file: NodeId, to_file: NodeId, symbols: Vec<String>) -> Result<EdgeId>
pub fn link_to_file(&mut self, entity: NodeId, file: NodeId) -> Result<EdgeId>
FR-2.3: Query Conveniences
rustpub fn get_callers(&self, function: NodeId) -> Result<Vec<NodeId>>
pub fn get_callees(&self, function: NodeId) -> Result<Vec<NodeId>>
pub fn get_functions_in_file(&self, file: NodeId) -> Result<Vec<NodeId>>
pub fn get_file_dependencies(&self, file: NodeId) -> Result<Vec<NodeId>>
pub fn get_file_dependents(&self, file: NodeId) -> Result<Vec<NodeId>>
FR-2.4: Transitive Queries
rustpub fn transitive_dependencies(&self, file: NodeId, max_depth: i32) -> Result<Vec<NodeId>>
pub fn transitive_dependents(&self, file: NodeId, max_depth: i32) -> Result<Vec<NodeId>>
pub fn call_chain(&self, from: NodeId, to: NodeId, max_depth: i32) -> Result<Vec<Vec<NodeId>>>
3. Query Builder
FR-3.1: Fluent Query API
rustgraph.query()
    .node_type(NodeType::Function)
    .in_file("src/main.rs")
    .name_contains("test_")
    .property("is_public", true)
    .execute()?
```

**FR-3.2: Supported Filters**
- Node type filtering
- File path filtering (exact or pattern)
- Property existence and value matching
- Name substring matching
- Custom predicate functions

**FR-3.3: Query Results**
- Return Vec<NodeId> for further processing
- Lazy evaluation where possible
- Sorted by relevance (configurable)

#### 4. Export & Visualization

**FR-4.1: DOT Format Export**
- Generate Graphviz-compatible DOT files
- Configurable node labels and colors
- Edge styling by type
- Subgraph support for file grouping

**FR-4.2: JSON Export**
- Schema: `{ nodes: [...], edges: [...] }`
- Compatible with D3.js and similar libraries
- Include all properties or filtered subset

**FR-4.3: CSV Export**
- Nodes as CSV (id, type, properties...)
- Edges as CSV (id, from, to, type, properties...)
- Suitable for pandas/R analysis

**FR-4.4: Triple Export**
- RDF-style triples for semantic web tools
- Format: (subject, predicate, object)
- Property flattening

#### 5. Performance & Reliability

**FR-5.1: Performance Targets**
- Graph with 100K nodes, 500K edges:
  - Single node lookup: <1ms
  - Neighbor query: <10ms
  - BFS traversal (depth=5): <50ms
  - Full graph scan: <500ms
- Indexing throughput: 10K+ nodes/sec on modern hardware

**FR-5.2: Memory Efficiency**
- Memory usage: ~50 bytes/node + ~30 bytes/edge (in-memory)
- RocksDB: Compressed storage with LSM tree compaction
- Batch operations to reduce write amplification

**FR-5.3: Reliability**
- No data loss on process crash (RocksDB WAL)
- Atomic batch operations
- Graceful degradation on storage errors
- Comprehensive error types with context

---

## Technical Architecture

### High-Level Design
```
┌─────────────────────────────────────────┐
│         User Code (Parser Logic)        │
│   (tree-sitter, syn, swc, custom)      │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           codegraph Public API          │
│  • add_file(), add_function()          │
│  • query().execute()                    │
│  • get_callers(), transitive_deps()    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│            Core Graph Engine            │
│  • Node/Edge CRUD                       │
│  • Property management                  │
│  • Adjacency indexing                   │
│  • Algorithm implementations            │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          Storage Backend Trait          │
│  ┌──────────────┬──────────────────┐   │
│  │ RocksDB      │  Memory          │   │
│  │ Backend      │  Backend         │   │
│  └──────────────┴──────────────────┘   │
└─────────────────────────────────────────┘
```

### Module Structure
```
codegraph/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API surface
│   ├── error.rs            # GraphError type
│   ├── property.rs         # PropertyMap, PropertyValue
│   ├── node.rs             # Node, NodeId, NodeType
│   ├── edge.rs             # Edge, EdgeId, EdgeType, Direction
│   ├── graph.rs            # CodeGraph implementation
│   ├── algorithms.rs       # BFS, DFS, SCC, path finding
│   ├── query.rs            # QueryBuilder API
│   ├── export.rs           # DOT, JSON, CSV, Triple exports
│   ├── code_helpers.rs     # add_file, add_function, etc.
│   └── storage/
│       ├── mod.rs
│       ├── backend.rs      # StorageBackend trait
│       ├── memory.rs       # MemoryBackend
│       └── rocksdb.rs      # RocksDBBackend
├── examples/
│   ├── call_graph.rs       # Build call graph from syn
│   ├── dependency_tree.rs  # Analyze file dependencies
│   ├── impact_analysis.rs  # Find affected files
│   ├── visualize.rs        # Export to Graphviz
│   └── rust_parser.rs      # Complete Rust parser integration
└── benches/
    └── graph_operations.rs # Performance benchmarks
```

### Data Model

**Node Storage:**
```
Key: "n:{node_id}"
Value: bincode-serialized Node struct
```

**Edge Storage:**
```
Key: "e:{edge_id}"
Value: bincode-serialized Edge struct
Adjacency Index (in-memory):
ruststruct AdjacencyIndex {
    outgoing: HashMap<NodeId, HashSet<EdgeId>>,
    incoming: HashMap<NodeId, HashSet<EdgeId>>,
    typed_index: HashMap<(NodeId, EdgeType, Direction), HashSet<EdgeId>>,
}

Development Approach
Test-Driven Development (TDD)
All features MUST be developed using TDD methodology:

Write tests first before implementing features
Red-Green-Refactor cycle:

RED: Write a failing test for the desired behavior
GREEN: Implement minimum code to make test pass
REFACTOR: Clean up code while keeping tests green


Test coverage targets:

Core graph operations: 95%+ coverage
Storage backends: 90%+ coverage
Code helpers: 85%+ coverage
Export functions: 80%+ coverage



Test Categories:
Unit Tests (fast, isolated)

Property system conversions
Node/Edge serialization
Individual graph operations
Query filter logic
Export format generation

Integration Tests (with backends)

RocksDB persistence and recovery
Batch operations across transactions
Index rebuilding from storage
Concurrent access patterns

Performance Tests (benchmarks)

Large graph operations (100K+ nodes)
Query performance under load
Memory usage profiling
Storage backend comparison

Example TDD Workflow:
rust// 1. Write the test first (RED)
#[test]
fn test_add_function_creates_contains_edge() {
    let mut graph = CodeGraph::in_memory();
    let file = graph.add_file(Path::new("test.rs"), "rust").unwrap();
    let func = graph.add_function(file, "main", 1, 10).unwrap();
    
    // Should create a Contains edge from file to function
    let edges = graph.get_edges_between(file, func).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].edge_type, EdgeType::Contains);
}

// 2. Implement minimum code to pass (GREEN)
impl CodeGraph {
    pub fn add_function(&mut self, file_id: NodeId, name: &str, 
                        line_start: usize, line_end: usize) -> Result<NodeId> {
        let func_id = self.add_node(
            NodeType::Function,
            PropertyMap::new()
                .with("name", name)
                .with("line_start", line_start as i64)
                .with("line_end", line_end as i64)
        )?;
        self.add_edge(file_id, func_id, EdgeType::Contains, PropertyMap::new())?;
        Ok(func_id)
    }
}

// 3. Refactor as needed while keeping tests green
Reference Implementation:

Use existing test patterns from native/rust-core/src/codegraph/ tests
Follow test organization from forge-search/src/lib.rs tests
Adopt benchmark structure from workspace if available


Implementation Timeline
Month 1: Core Foundation
Week 1-2: Graph Engine Polish

Review and optimize existing graph.rs implementation
Add missing edge cases in adjacency indexing
Write comprehensive unit tests for all core operations
Benchmark and profile hot paths
Deliverable: Core graph with 95%+ test coverage

Week 3-4: Code Helpers

Implement code_helpers.rs module
Add convenience constructors (add_file, add_function, add_class)
Implement relationship helpers (add_call, add_import)
Write tests for each helper function
Deliverable: Code-specific API layer

Month 2: Query & Export
Week 5-6: Query Builder

Design and implement QueryBuilder API
Add filters (node type, file path, properties, name matching)
Implement result sorting and pagination
Write integration tests with real graph data
Deliverable: Fluent query interface

Week 7-8: Export Formats

Implement DOT export with Graphviz styling
Implement JSON export for web visualization
Implement CSV export for data analysis
Add triple export for RDF tools
Write tests and example usage for each format
Deliverable: Multiple export formats

Month 3: Polish & Release
Week 9-10: Documentation & Examples

Write comprehensive API documentation
Create 5+ working examples:

Rust call graph using syn
Dependency tree analyzer
Impact assessment tool
Graphviz visualization
Python parser using tree-sitter-python


Write integration guide for common parsers
Deliverable: Production-ready documentation

Week 11-12: Release Preparation

Performance optimization based on benchmarks
API stabilization (breaking changes now or never)
Prepare crates.io package
Write announcement blog post
Create GitHub release with examples
Deliverable: codegraph v0.1.0 on crates.io


Success Metrics
Technical Metrics (Month 1)

✅ 90%+ test coverage on core modules
✅ All benchmarks passing performance targets
✅ Zero unsafe code (or fully documented rationale)
✅ No clippy warnings with pedantic lints

Adoption Metrics (Month 4)

✅ 200+ GitHub stars
✅ 50+ downloads/week on crates.io
✅ 3+ community contributions (PRs or issues with good feedback)
✅ 2+ external projects using codegraph

Quality Metrics (Ongoing)

✅ <5 bugs per 1000 users
✅ <24hr response time on GitHub issues
✅ 95%+ test coverage maintained
✅ Documentation coverage for all public APIs


Out of Scope (v0.1.0)
Explicitly Not Included
Language Parsers

No built-in tree-sitter integration
No syn/swc wrappers
Users bring their own parser
Rationale: Reduces scope, avoids version conflicts, maximizes flexibility

Incremental Indexing

No file watcher integration
No delta updates from git diffs
Manual re-indexing required
Future: v0.2.0 feature

Semantic Analysis

No type inference
No scope resolution
No symbol table
Future: artifact-sitter project (separate)

Distributed/Cloud Features

No multi-machine clustering
No hosted service
No real-time sync
Future: Commercial offering possibility

Language-Specific Optimizations

No Rust-specific optimizations (macro expansion, trait resolution)
No JavaScript-specific features (module resolution)
Rationale: Keep core language-agnostic


API Design Examples
Basic Usage
rustuse codegraph::{CodeGraph, NodeType, EdgeType};

// Create or open a graph
let mut graph = CodeGraph::open("./my_project.graph")?;

// Add files (user parses, we store)
let main_rs = graph.add_file(Path::new("src/main.rs"), "rust")?;
let lib_rs = graph.add_file(Path::new("src/lib.rs"), "rust")?;

// Add functions
let main_fn = graph.add_function(main_rs, "main", 1, 10)?;
let helper = graph.add_function(lib_rs, "helper", 5, 15)?;

// Record relationships
graph.add_call(main_fn, helper, 7)?;
graph.add_import(main_rs, lib_rs, vec!["helper".to_string()])?;

// Query
let callers = graph.get_callers(helper)?;
println!("Functions calling helper: {:?}", callers);

// Export for visualization
std::fs::write("graph.dot", graph.export_dot()?)?;
Query Builder Usage
rust// Find all test functions in a specific file
let test_functions = graph.query()
    .node_type(NodeType::Function)
    .in_file("tests/integration_test.rs")
    .name_contains("test_")
    .execute()?;

// Find public functions that call private functions
let public_fns = graph.query()
    .node_type(NodeType::Function)
    .property("visibility", "public")
    .execute()?;

for func in public_fns {
    let callees = graph.get_callees(func)?;
    // ... analyze callees
}
Integration with Parsers
rustuse syn::{parse_file, Item};

// User's parsing logic
let source = std::fs::read_to_string("src/lib.rs")?;
let ast = parse_file(&source)?;

let mut graph = CodeGraph::open("./project.graph")?;
let file_id = graph.add_file(Path::new("src/lib.rs"), "rust")?;

for item in ast.items {
    match item {
        Item::Fn(func) => {
            let name = func.sig.ident.to_string();
            let line = func.sig.ident.span().start().line;
            graph.add_function(file_id, &name, line, line + 10)?;
        }
        // ... other items
    }
}

Dependencies
Required Crates
toml[dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"
thiserror = "1.0"

# Storage
rocksdb = "0.22"

# Utilities
indexmap = "2.0"  # Ordered maps for deterministic output

[dev-dependencies]
tempfile = "3.10"
criterion = "0.5"  # Benchmarking
```

### Reference Codebases

**Existing implementations to reference:**
1. `forge-search/src/graph.rs` - Graph structure and algorithms
2. `forge-search/src/chunker.rs` - Code parsing patterns (informational)
3. `native/rust-core/src/codegraph/` - Current foundation (starting point)
4. `mcp-context-server` - Integration patterns for tools

**Study but don't copy:**
- `forge-search/src/embeddings.rs` - Caching patterns
- `forge-search/src/indexer.rs` - File scanning patterns

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Performance doesn't meet targets | High | Medium | Benchmark early, optimize hot paths, consider alternative algorithms |
| RocksDB stability issues | High | Low | Comprehensive testing, graceful degradation, document known issues |
| API design doesn't feel natural | Medium | Medium | Early user feedback, iterate on examples, dogfood in own tools |
| Adoption slower than expected | Medium | High | Focus on documentation quality, create compelling examples, engage community early |
| Scope creep (adding parsers) | High | High | Strict adherence to "no built-in parsers" principle, redirect to examples |

---

## Open Questions

1. **Graph schema versioning:** How do we handle schema evolution when users upgrade?
   - **Proposed:** Semantic versioning, migration guides, backwards compatibility for 0.x series

2. **Concurrency model:** Should we support concurrent reads/writes from multiple threads?
   - **Proposed:** v0.1.0 is single-writer, v0.2.0 adds read-write lock support

3. **Node ID stability:** Should IDs persist across graph rebuilds?
   - **Proposed:** IDs are ephemeral, use content-based keys (file path + line) for stability

4. **Property schema:** Should we enforce property schemas per node type?
   - **Proposed:** No enforcement in v0.1.0, but provide optional validation helpers

5. **Export size limits:** What's the largest graph we should support exporting to DOT/JSON?
   - **Proposed:** Warn above 10K nodes, error above 100K nodes, suggest chunking

---

## Appendix A: EdgeType Semantics

| EdgeType | Meaning | Example |
|----------|---------|---------|
| `Imports` | File A imports File B | `main.rs` imports `lib.rs` |
| `ImportsFrom` | File A imports symbols from File B | `main.rs` imports `{foo, bar}` from `lib.rs` |
| `Contains` | Parent contains child | File contains Function |
| `Calls` | Function A calls Function B | `main()` calls `helper()` |
| `Invokes` | Function invokes method | `obj.method()` |
| `Instantiates` | Function creates instance | `new Class()` |
| `Extends` | Class A extends Class B | `class Child extends Parent` |
| `Implements` | Class implements interface | `impl Trait for Struct` |
| `Uses` | Generic usage relationship | Variable uses Type |
| `Defines` | Module defines entity | Module defines Function |
| `References` | Generic reference | Variable references constant |

---

## Appendix B: Performance Benchmarks

All benchmarks run on: AMD Ryzen 9 / 32GB RAM / NVMe SSD

**Target Performance:**
```
Graph Size: 100K nodes, 500K edges

Node Operations:
  insert_node                 1.2 µs
  get_node                    800 ns
  update_node                 1.5 µs
  delete_node (with edges)    45 µs

Edge Operations:
  insert_edge                 1.8 µs
  get_edge                    900 ns
  get_edges_between           12 µs

Traversal:
  neighbors (outgoing)        8 µs
  neighbors_typed             10 µs
  bfs_traversal (depth=5)     45 ms
  dfs_traversal (depth=5)     38 ms

Algorithms:
  find_callers                15 µs
  transitive_deps (depth=5)   120 ms
  circular_deps (100 nodes)   250 ms

Storage:
  batch_insert (1K nodes)     25 ms
  flush_to_disk              50 ms
  load_from_disk             180 ms

Appendix C: Example Projects (Post-Release)
Community contributions we'd like to see:

codegraph-rust - First-party Rust parser using syn
codegraph-js - JavaScript/TypeScript parser using swc
codegraph-py - Python parser using tree-sitter
codegraph-viz - Web-based graph visualizer
codegraph-cli - Command-line tool for common queries
codegraph-lsp - LSP server showing code relationships in editors

