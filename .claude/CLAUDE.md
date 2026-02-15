# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Build entire workspace
cargo build --workspace

# Test entire workspace (421+ tests)
cargo test --workspace

# Test a specific crate
cargo test -p codegraph-python
cargo test -p codegraph-rust

# Run a single test by name
cargo test -p codegraph-python -- test_name

# Lint (all warnings are errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format
cargo fmt --all

# Format check (no changes)
cargo fmt --all -- --check

# CI checks (clippy + format check; use --full for benchmarks, docs, coverage)
./scripts/ci-checks.sh
```

Run `./scripts/ci-checks.sh` after making significant changes to the code.

## Architecture

This is a Rust workspace for a code graph database with language parsers. The workspace has three layers of crates:

### Core Crates

- **`codegraph`** - Graph database with pluggable storage backends (RocksDB for production, in-memory BTreeMap for tests). Key types: `CodeGraph`, `Node`, `Edge`, `NodeType`, `EdgeType`, `PropertyMap`. Modules: `graph/` (types, algorithms with BFS/DFS/cycle detection), `storage/` (trait-based backends), `export/` (DOT, JSON, CSV, RDF), `helpers.rs` (code-specific convenience API), `query.rs` (fluent query builder).

- **`codegraph-parser-api`** - Defines the `CodeParser` trait that all language parsers implement, plus shared types: `FileInfo`, `ProjectInfo`, `ParserConfig`, `ParserMetrics`, `CodeIR` (intermediate representation), and entity types (`FunctionEntity`, `ClassEntity`, `TraitEntity`, etc.). The `CodeIR` serves as the bridge between language-specific ASTs and the graph.

### Language Parser Crates

All parsers (`codegraph-python`, `codegraph-rust`, `codegraph-typescript`, `codegraph-go`, `codegraph-c`, `codegraph-cpp`, `codegraph-csharp`, `codegraph-java`, `codegraph-kotlin`, `codegraph-php`, `codegraph-ruby`, `codegraph-swift`) follow the same internal structure:

| Module | Purpose |
|--------|---------|
| `parser_impl.rs` | `CodeParser` trait implementation |
| `extractor.rs` | Source code -> `CodeIR` using tree-sitter |
| `visitor.rs` | Tree-sitter AST traversal with context tracking |
| `mapper.rs` | `CodeIR` -> graph nodes and edges |

The data flow is: **Source code -> tree-sitter AST -> Visitor -> CodeIR -> Mapper -> CodeGraph**

### Key Design Patterns

- **CodeIR decoupling**: Language-specific parsing produces a universal `CodeIR`, which is then mapped to graph nodes/edges. This separates parsing concerns from graph construction.
- **StorageBackend trait**: `put`/`get`/`delete`/`scan_prefix`/`write_batch` interface. Use `CodeGraph::open(path)` for RocksDB, `CodeGraph::in_memory()` for tests.
- **Thread-safe parsers**: All parsers are `Send + Sync` with `Mutex<ParserMetrics>` for concurrent use with rayon.
- **Two-tier storage**: In-memory caches (HashMaps for nodes/edges, adjacency indexes) backed by persistent RocksDB.

## Conventions

- Follows TDD: write failing test first, then implement, then refactor.
- No panics in library code; always return `Result<T>`.
- No unsafe code.
- Commit messages use conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `perf:`, `chore:`.
- Tests live both inline (`#[cfg(test)] mod tests`) and in `tests/` directories (integration tests).
- Use `CodeGraph::in_memory()` for test graphs, not RocksDB.
