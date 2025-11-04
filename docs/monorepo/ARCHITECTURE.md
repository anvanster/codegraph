# CodeGraph Architecture v0.2.0

## Executive Summary

This document describes the architectural evolution of CodeGraph from independent crates to a unified monorepo with a shared parser API. This change enables:

- **Type-safe parser contracts** via shared traits
- **Consistent entity models** across all language parsers
- **Atomic development** of core parsers
- **Clear extension points** for community parsers
- **SaaS-ready design** with built-in telemetry and resource limits

## Design Principles

### 1. Parser Agnosticism (Maintained)
The `codegraph` database remains completely parser-agnostic. It stores nodes and edges without language-specific knowledge.

### 2. Contract-First Development (New)
All parsers implement the `CodeParser` trait from `codegraph-parser-api`, ensuring compile-time compatibility.

### 3. Separation of Concerns
```
┌─────────────────────────────────────────────────┐
│  User Applications & Tools                      │
└─────────────────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
    ┌────▼─────┐          ┌─────▼────┐
    │ Parsers  │          │ Database │
    │ (Traits) │◄─────────┤ API      │
    └──────────┘          └──────────┘
         │
    ┌────▼─────────┐
    │ Parser API   │
    │ (Contract)   │
    └──────────────┘
```

### 4. Open Core Model
- **Core parsers** (5-10 languages): Maintained in monorepo, shipped with tool
- **Community parsers**: External crates implementing the trait
- **SaaS parsers**: Subset of core + vetted community parsers

## Architecture Overview

### Crate Structure

```
codegraph-workspace/
├── crates/
│   ├── codegraph/              # Graph database (unchanged API)
│   │   ├── src/
│   │   └── Cargo.toml
│   │
│   ├── codegraph-parser-api/   # NEW: Shared trait & types
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs       # CodeParser trait
│   │   │   ├── entities/       # Shared entity types
│   │   │   ├── relationships/  # Shared relationship types
│   │   │   ├── config.rs       # ParserConfig
│   │   │   ├── metrics.rs      # ParserMetrics
│   │   │   └── errors.rs       # Parser errors
│   │   └── Cargo.toml
│   │
│   ├── codegraph-python/       # Migrated to use API
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── parser.rs       # Implements CodeParser
│   │   │   └── visitor.rs      # Python-specific AST visitor
│   │   └── Cargo.toml
│   │
│   ├── codegraph-rust/         # New parser
│   ├── codegraph-typescript/   # New parser
│   └── codegraph-go/           # New parser
│
├── Cargo.toml                  # Workspace root
├── ARCHITECTURE.md             # This file
├── MIGRATION_GUIDE.md          # Step-by-step migration
└── PARSER_API_SPEC.md          # Detailed API specification
```

### Dependency Graph

```
codegraph-python ─────┐
                      │
codegraph-rust ───────┼──► codegraph-parser-api ──► codegraph
                      │            │
codegraph-typescript ─┘            │
                                   ▼
                            serde, thiserror
```

**Key Properties:**
- No circular dependencies
- Parser API depends only on `codegraph`
- All parsers depend on `codegraph-parser-api`
- Parsers never depend on each other

## Core Components

### 1. codegraph-parser-api

**Purpose:** Define the contract that all parsers must follow.

**Responsibilities:**
- Define `CodeParser` trait
- Provide shared entity types (FunctionEntity, ClassEntity, etc.)
- Provide shared relationship types (CallRelation, ImportRelation, etc.)
- Define configuration options (ParserConfig)
- Define metrics and telemetry (ParserMetrics)
- Provide parser errors

**Key Design Decisions:**
- **Language-agnostic entities:** All entities use generic types that map to any language
- **Serde support:** All types derive Serialize/Deserialize for wire protocols
- **Builder pattern:** Entities use builders for ergonomic construction
- **Version stability:** Follows semantic versioning strictly (0.x allows breaking changes)

### 2. codegraph (Database)

**Changes:** None to core API! Backward compatible.

**Future Enhancements (Post-migration):**
- Batch insertion API for improved performance
- Transaction support
- Query optimization hints from parser metrics

### 3. Language Parsers

**Structure (all parsers follow this pattern):**
```
codegraph-{language}/
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── parser.rs           # Implements CodeParser trait
│   ├── visitor.rs          # Language-specific AST walking
│   ├── mapper.rs           # Maps language AST → IR entities
│   └── tests.rs            # Parser-specific tests
└── Cargo.toml
```

**Responsibilities:**
- Implement `CodeParser` trait
- Parse source code using language-specific parser (tree-sitter, syn, etc.)
- Extract entities into shared IR types
- Extract relationships
- Provide language-specific configuration

## The CodeParser Trait

### Interface

```rust
pub trait CodeParser: Send + Sync {
    /// Get the language identifier (e.g., "python", "rust", "typescript")
    fn language(&self) -> &str;
    
    /// Get supported file extensions (e.g., [".py", ".pyw"])
    fn file_extensions(&self) -> &[&str];
    
    /// Parse a single file and add to graph
    fn parse_file(
        &self,
        path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError>;
    
    /// Parse source code string and add to graph
    fn parse_source(
        &self,
        source: &str,
        file_path: &Path,
        graph: &mut CodeGraph,
    ) -> Result<FileInfo, ParserError>;
    
    /// Parse multiple files (default: sequential, can override for parallel)
    fn parse_files(
        &self,
        paths: &[PathBuf],
        graph: &mut CodeGraph,
    ) -> Result<ProjectInfo, ParserError> {
        // Default implementation with sequential parsing
    }
    
    /// Get parser configuration
    fn config(&self) -> &ParserConfig;
    
    /// Get accumulated metrics
    fn metrics(&self) -> ParserMetrics;
    
    /// Check if parser can handle this file (based on extension, etc.)
    fn can_parse(&self, path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e));
        
        ext.map(|e| self.file_extensions().contains(&e.as_str()))
            .unwrap_or(false)
    }
}
```

### Return Types

```rust
/// Information about a parsed file
pub struct FileInfo {
    pub file_path: PathBuf,
    pub file_id: NodeId,
    pub functions: Vec<NodeId>,
    pub classes: Vec<NodeId>,
    pub imports: Vec<NodeId>,
    pub parse_time: Duration,
    pub line_count: usize,
}

/// Information about a parsed project
pub struct ProjectInfo {
    pub files: Vec<FileInfo>,
    pub total_functions: usize,
    pub total_classes: usize,
    pub total_parse_time: Duration,
    pub failed_files: Vec<(PathBuf, String)>,
}
```

## Entity Model

All parsers produce these standardized entities:

### Core Entities

1. **FunctionEntity**
   - Represents functions, methods, closures
   - Language-agnostic representation
   - Includes: name, signature, visibility, location, async flag, etc.

2. **ClassEntity**
   - Represents classes, structs, interfaces, traits
   - Includes: name, bases, methods, fields, abstract flag, etc.

3. **ModuleEntity**
   - Represents files, modules, packages
   - Includes: path, imports, exports, docstrings

4. **TraitEntity**
   - Represents protocols, interfaces, traits
   - Includes: name, required methods, implementations

### Relationships

1. **CallRelation** - Function A calls Function B
2. **ImportRelation** - Module A imports Module B
3. **InheritanceRelation** - Class A inherits from Class B
4. **ImplementationRelation** - Class A implements Trait B

## Configuration & Metrics

### ParserConfig

```rust
pub struct ParserConfig {
    /// Skip private entities (e.g., Python _private)
    pub skip_private: bool,
    
    /// Skip test files/functions
    pub skip_tests: bool,
    
    /// Maximum file size to parse (bytes)
    pub max_file_size: usize,
    
    /// Timeout per file
    pub timeout_per_file: Option<Duration>,
    
    /// Enable parallel parsing
    pub parallel: bool,
    
    /// Number of parallel workers (None = num_cpus)
    pub parallel_workers: Option<usize>,
}
```

### ParserMetrics

```rust
pub struct ParserMetrics {
    /// Total files attempted
    pub files_attempted: usize,
    
    /// Files successfully parsed
    pub files_succeeded: usize,
    
    /// Files that failed parsing
    pub files_failed: usize,
    
    /// Total parse time
    pub total_parse_time: Duration,
    
    /// Average parse time per file
    pub avg_parse_time: Duration,
    
    /// Total entities extracted
    pub total_entities: usize,
    
    /// Total relationships extracted
    pub total_relationships: usize,
    
    /// Memory usage (if available)
    pub peak_memory_bytes: Option<usize>,
}
```

## Performance Strategy

### Target: <10 seconds for 1,000 files

**Bottleneck Analysis:**
1. **Parsing:** ~30-50% of time (language-dependent)
2. **Graph insertion:** ~40-60% of time
3. **I/O:** ~10-20% of time

**Optimization Strategies:**

#### 1. Parallel Parsing
```rust
impl CodeParser for PythonParser {
    fn parse_files(&self, paths: &[PathBuf], graph: &mut CodeGraph) 
        -> Result<ProjectInfo> {
        if self.config.parallel {
            // Parse in parallel, batch insert
            let chunks: Vec<CodeIR> = paths
                .par_iter()
                .map(|p| self.parse_to_ir(p))
                .collect();
            
            // Single-threaded batch insert
            for ir in chunks {
                self.ir_to_graph(ir, graph)?;
            }
        }
    }
}
```

#### 2. Batch Insertion
- Extract entities to IR first
- Bulk insert nodes in batches
- Bulk insert edges in batches
- Reduces RocksDB write amplification

#### 3. Incremental Parsing
```rust
fn parse_incremental(
    &self,
    changed_files: &[PathBuf],
    deleted_files: &[PathBuf],
    graph: &mut CodeGraph,
) -> Result<IncrementalInfo>
```

#### 4. Priority Parsing
```rust
fn parse_priority(
    &self,
    priority_paths: &[PathBuf],    // Parse immediately
    background_paths: &[PathBuf],  // Parse in background
    graph: &mut CodeGraph,
) -> Result<PriorityParseInfo>
```

## Migration Path

### Phase 1: Foundation (Week 1)
- [ ] Create `codegraph-parser-api` crate
- [ ] Define `CodeParser` trait
- [ ] Move entity types from `codegraph-python` to API crate
- [ ] Add comprehensive tests for API crate

### Phase 2: Monorepo Setup (Week 2)
- [ ] Create workspace `Cargo.toml`
- [ ] Move `codegraph` into `crates/codegraph/`
- [ ] Move `codegraph-python` into `crates/codegraph-python/`
- [ ] Update CI/CD for monorepo
- [ ] Verify all tests pass

### Phase 3: Python Migration (Week 2-3)
- [ ] Update `codegraph-python` to depend on `codegraph-parser-api`
- [ ] Implement `CodeParser` trait
- [ ] Remove duplicate entity definitions
- [ ] Update tests
- [ ] Publish `codegraph-python` v0.2.0

### Phase 4: New Parsers (Week 3-4)
- [ ] Implement `codegraph-rust` using trait from day 1
- [ ] Implement `codegraph-typescript` using trait from day 1
- [ ] Add comprehensive integration tests
- [ ] Benchmark performance

### Phase 5: Optimization (Week 4)
- [ ] Add batch insertion to `codegraph` database
- [ ] Optimize parallel parsing
- [ ] Add telemetry and metrics
- [ ] Performance testing with large codebases

## SaaS Considerations

### Resource Limits

```rust
pub struct SaaSConfig {
    pub max_files_per_request: usize,
    pub max_total_size: usize,
    pub timeout: Duration,
    pub rate_limit: RateLimit,
}
```

### Telemetry

```rust
pub trait ParserTelemetry {
    fn on_file_start(&self, path: &Path);
    fn on_file_complete(&self, path: &Path, duration: Duration);
    fn on_file_error(&self, path: &Path, error: &ParserError);
}
```

### Parser Registry

```rust
pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn CodeParser>>,
}

impl ParserRegistry {
    pub fn register<P: CodeParser + 'static>(&mut self, parser: P);
    pub fn get(&self, language: &str) -> Option<&dyn CodeParser>;
    pub fn detect_language(&self, path: &Path) -> Option<&dyn CodeParser>;
}
```

## Testing Strategy

### Unit Tests
- Each entity type
- Each relationship type
- Config and metrics

### Integration Tests
- Parser trait implementation
- End-to-end parsing
- Graph insertion verification

### Performance Tests
- Parse 1,000 files in <10s
- Memory usage under 1GB
- No memory leaks

### Compatibility Tests
- Ensure parsers follow trait contract
- Cross-language consistency
- Backward compatibility

## Versioning Strategy

### codegraph-parser-api
- **0.1.x:** Initial API, may have breaking changes
- **0.2.x:** Stabilize core entities
- **1.0.0:** Stable API, semantic versioning enforced

### Language Parsers
- Follow API version compatibility
- Can release independently
- Must specify supported API version

### Compatibility Matrix

```toml
# codegraph-rust/Cargo.toml
[dependencies]
codegraph-parser-api = "0.1"  # Compatible with 0.1.x

# codegraph-python/Cargo.toml
[dependencies]
codegraph-parser-api = "0.1"  # Compatible with 0.1.x
```

## Future Enhancements

### v0.3.0: Advanced Features
- [ ] Semantic analysis hooks
- [ ] Custom entity types
- [ ] Plugin system
- [ ] LSP integration

### v0.4.0: Distributed Parsing
- [ ] Remote parser workers
- [ ] Cloud-based parsing
- [ ] Streaming results

### v1.0.0: Production Ready
- [ ] Stable API
- [ ] Full SaaS integration
- [ ] Commercial support
- [ ] Enterprise features

## Conclusion

This architecture provides:
- **Type safety** through shared traits
- **Consistency** through shared entity models
- **Performance** through parallel parsing and batch insertion
- **Extensibility** through clear plugin points
- **Maintainability** through monorepo structure

The design supports both the open-source library use case and future SaaS business model while maintaining the core principle of parser agnosticism.
