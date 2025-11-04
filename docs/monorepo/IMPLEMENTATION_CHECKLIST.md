# CodeGraph Migration Implementation Checklist

Use this checklist to track progress through the migration to monorepo + parser-api architecture.

---

## Week 1: Foundation

### Phase 1: Monorepo Setup (Days 1-2)

- [ ] Create workspace directory structure
  ```bash
  mkdir -p codegraph-workspace/crates
  ```

- [ ] Create workspace `Cargo.toml`
  - [ ] Define workspace members
  - [ ] Define shared dependencies
  - [ ] Set workspace-level metadata

- [ ] Copy existing `codegraph` crate
  - [ ] Copy to `crates/codegraph/`
  - [ ] Update Cargo.toml to use workspace dependencies
  - [ ] Verify builds: `cargo build -p codegraph`
  - [ ] Verify tests: `cargo test -p codegraph`

- [ ] Initialize Git repository
  - [ ] `git init`
  - [ ] Create `.gitignore`
  - [ ] Initial commit

- [ ] Verify workspace
  - [ ] `cargo build --workspace` succeeds
  - [ ] `cargo test --workspace` passes

### Phase 2: Parser API Crate (Days 3-5)

- [ ] Create `codegraph-parser-api` crate
  ```bash
  cd crates && cargo new codegraph-parser-api --lib
  ```

- [ ] Set up Cargo.toml
  - [ ] Add dependencies (codegraph, serde, thiserror)
  - [ ] Set workspace metadata

- [ ] Create module structure
  - [ ] `src/traits.rs` - CodeParser trait
  - [ ] `src/config.rs` - ParserConfig
  - [ ] `src/metrics.rs` - ParserMetrics
  - [ ] `src/errors.rs` - ParserError
  - [ ] `src/ir.rs` - CodeIR
  - [ ] `src/entities/` - Entity types
  - [ ] `src/relationships/` - Relationship types

- [ ] Implement core types
  - [ ] ParserError enum with thiserror
  - [ ] ParserConfig struct with defaults
  - [ ] ParserMetrics struct with calculations
  - [ ] CodeIR struct

- [ ] Implement entity types
  - [ ] FunctionEntity with builder pattern
  - [ ] ClassEntity with builder pattern
  - [ ] ModuleEntity
  - [ ] TraitEntity
  - [ ] Parameter struct
  - [ ] Field struct

- [ ] Implement relationship types
  - [ ] CallRelation
  - [ ] ImportRelation
  - [ ] InheritanceRelation
  - [ ] ImplementationRelation

- [ ] Implement CodeParser trait
  - [ ] Define trait methods
  - [ ] Add documentation
  - [ ] Define FileInfo return type
  - [ ] Define ProjectInfo return type

- [ ] Update lib.rs
  - [ ] Re-export all public types
  - [ ] Add crate-level documentation

- [ ] Write unit tests
  - [ ] Test entity builders
  - [ ] Test config defaults
  - [ ] Test metrics calculations
  - [ ] Test CodeIR operations
  - [ ] Aim for >90% coverage

- [ ] Verify API crate
  - [ ] `cargo build -p codegraph-parser-api` succeeds
  - [ ] `cargo test -p codegraph-parser-api` passes
  - [ ] `cargo clippy -p codegraph-parser-api` no warnings
  - [ ] `cargo doc -p codegraph-parser-api` builds

---

## Week 2: Python Parser Migration

### Phase 3: Migrate codegraph-python

- [ ] Copy existing crate
  ```bash
  cp -r /path/to/codegraph-python crates/
  ```

- [ ] Update Cargo.toml
  - [ ] Bump version to 0.2.0
  - [ ] Add `codegraph-parser-api` dependency
  - [ ] Use workspace dependencies

- [ ] Remove duplicate code
  - [ ] Delete `src/entities/` (now in parser-api)
  - [ ] Delete `src/relationships/` (now in parser-api)
  - [ ] Update all imports to use parser-api types

- [ ] Update extractor
  - [ ] Change return type to `CodeIR`
  - [ ] Use parser-api entity types
  - [ ] Use parser-api error types

- [ ] Implement CodeParser trait
  - [ ] Create `src/parser_impl.rs`
  - [ ] Implement `PythonParser` struct
  - [ ] Implement all trait methods:
    - [ ] `language()`
    - [ ] `file_extensions()`
    - [ ] `parse_file()`
    - [ ] `parse_source()`
    - [ ] `config()`
    - [ ] `metrics()`
    - [ ] `reset_metrics()`

- [ ] Add backward compatibility layer
  - [ ] Keep old `Parser` struct (deprecated)
  - [ ] Wrap new `PythonParser` internally
  - [ ] Add deprecation warnings

- [ ] Update builder module
  - [ ] Create `ir_to_graph()` method
  - [ ] Batch insert nodes
  - [ ] Batch insert edges
  - [ ] Return FileInfo

- [ ] Update all tests
  - [ ] Use `CodeParser` trait
  - [ ] Use new `PythonParser`
  - [ ] Verify all integration tests pass
  - [ ] Add tests for deprecated API

- [ ] Update documentation
  - [ ] Add migration guide to README
  - [ ] Update examples
  - [ ] Add deprecation notices
  - [ ] Document new API

- [ ] Verify migration
  - [ ] `cargo build -p codegraph-python` succeeds
  - [ ] `cargo test -p codegraph-python` passes
  - [ ] `cargo clippy -p codegraph-python` (expect deprecation warnings)
  - [ ] `cargo doc -p codegraph-python` builds

---

## Week 3: New Parsers

### Phase 4: Rust Parser

- [ ] Create crate
  ```bash
  cargo new crates/codegraph-rust --lib
  ```

- [ ] Set up Cargo.toml
  - [ ] Add dependencies (parser-api, syn)
  - [ ] Set version to 0.1.0

- [ ] Implement parser structure
  - [ ] Create `src/parser.rs` with `RustParser`
  - [ ] Create `src/visitor.rs` for AST walking
  - [ ] Create `src/mapper.rs` for AST → IR

- [ ] Implement CodeParser trait
  - [ ] All required methods
  - [ ] Rust-specific logic

- [ ] Extract entities
  - [ ] Functions (including methods)
  - [ ] Structs and enums (as classes)
  - [ ] Traits
  - [ ] Modules
  - [ ] Impl blocks

- [ ] Extract relationships
  - [ ] Function calls
  - [ ] use statements (imports)
  - [ ] Trait implementations
  - [ ] Struct inheritance (rare, but handle)

- [ ] Write comprehensive tests
  - [ ] Parse simple functions
  - [ ] Parse structs with methods
  - [ ] Parse traits
  - [ ] Parse modules
  - [ ] Handle errors gracefully

- [ ] Documentation
  - [ ] README with examples
  - [ ] API documentation
  - [ ] Usage guide

- [ ] Verify
  - [ ] Build and test pass
  - [ ] No clippy warnings
  - [ ] Documentation complete

### Phase 4: TypeScript Parser

- [ ] Create crate
  ```bash
  cargo new crates/codegraph-typescript --lib
  ```

- [ ] Set up Cargo.toml
  - [ ] Add dependencies (parser-api, swc or tree-sitter)

- [ ] Implement parser
  - [ ] `TypeScriptParser` struct
  - [ ] Visitor for TS AST
  - [ ] Mapper for IR

- [ ] Extract entities
  - [ ] Functions (including arrow functions)
  - [ ] Classes
  - [ ] Interfaces
  - [ ] Modules/namespaces

- [ ] Extract relationships
  - [ ] Function calls
  - [ ] Imports
  - [ ] Class inheritance
  - [ ] Interface implementations

- [ ] Test and document
  - [ ] Comprehensive tests
  - [ ] README and examples
  - [ ] API docs

---

## Week 4: Polish & Release

### Phase 5: CI/CD Setup

- [ ] Create GitHub Actions workflow
  - [ ] `.github/workflows/ci.yml`
  - [ ] Build all crates
  - [ ] Run all tests
  - [ ] Run clippy
  - [ ] Check formatting

- [ ] Add code coverage
  - [ ] Set up tarpaulin or grcov
  - [ ] Upload to codecov.io
  - [ ] Add coverage badge to README

- [ ] Create publish workflow
  - [ ] `.github/workflows/publish.yml`
  - [ ] Publish on version tags
  - [ ] Publish parser-api first
  - [ ] Publish parsers sequentially

- [ ] Set up branch protection
  - [ ] Require CI pass before merge
  - [ ] Require reviews
  - [ ] Protect main branch

### Phase 6: Documentation

- [ ] Create workspace README
  - [ ] Overview of all crates
  - [ ] Quick start guide
  - [ ] Links to individual crates

- [ ] Update individual READMEs
  - [ ] codegraph - No changes
  - [ ] codegraph-parser-api - Usage guide
  - [ ] codegraph-python - Migration guide
  - [ ] New parsers - Getting started

- [ ] Create ARCHITECTURE.md
  - [ ] Design decisions
  - [ ] Crate relationships
  - [ ] Extension points

- [ ] Create MIGRATION_GUIDE.md
  - [ ] Step-by-step instructions
  - [ ] Breaking changes
  - [ ] Rollback plan

- [ ] Create CONTRIBUTING.md
  - [ ] How to add new parsers
  - [ ] Code style guide
  - [ ] Testing requirements

- [ ] Update CHANGELOG.md
  - [ ] Document all changes in 0.2.0
  - [ ] Migration notes
  - [ ] Breaking changes

### Phase 7: Release

- [ ] Pre-release checks
  - [ ] All tests pass on CI
  - [ ] Documentation is complete
  - [ ] Examples work
  - [ ] Changelog updated
  - [ ] Version numbers correct

- [ ] Publish parser-api
  - [ ] `cd crates/codegraph-parser-api`
  - [ ] `cargo publish --dry-run`
  - [ ] `cargo publish`
  - [ ] Verify on crates.io

- [ ] Publish codegraph-python
  - [ ] Wait for crates.io indexing (30s)
  - [ ] `cd crates/codegraph-python`
  - [ ] `cargo publish --dry-run`
  - [ ] `cargo publish`
  - [ ] Verify on crates.io

- [ ] Publish new parsers
  - [ ] codegraph-rust
  - [ ] codegraph-typescript
  - [ ] Verify all on crates.io

- [ ] Create GitHub release
  - [ ] Tag: v0.2.0
  - [ ] Release notes from CHANGELOG
  - [ ] Highlight breaking changes

- [ ] Announce release
  - [ ] Reddit /r/rust
  - [ ] Twitter/X
  - [ ] This Week in Rust
  - [ ] Company blog (if applicable)

---

## Post-Migration Tasks

### Performance Optimization

- [ ] Benchmark current performance
  - [ ] 1K files baseline
  - [ ] 10K files baseline
  - [ ] Identify bottlenecks

- [ ] Optimize database insertion
  - [ ] Add batch insert API to codegraph
  - [ ] Use batch inserts in parsers
  - [ ] Benchmark improvement

- [ ] Optimize parallel parsing
  - [ ] Profile rayon usage
  - [ ] Tune worker count
  - [ ] Test on large codebases

- [ ] Add performance tests to CI
  - [ ] Set performance targets
  - [ ] Fail CI if targets missed
  - [ ] Track performance over time

### Community & Growth

- [ ] Create examples repository
  - [ ] Real-world use cases
  - [ ] Integration examples
  - [ ] Visualization examples

- [ ] Write blog posts
  - [ ] Architecture decisions
  - [ ] Performance results
  - [ ] Use case tutorials

- [ ] Engage community
  - [ ] Respond to issues
  - [ ] Review PRs promptly
  - [ ] Foster contributions

- [ ] Plan v0.3.0
  - [ ] Remove deprecated APIs
  - [ ] Add requested features
  - [ ] Improve documentation

---

## Success Metrics

Track these metrics post-migration:

- [ ] **Build time:** <5 minutes for full workspace
- [ ] **Test time:** <2 minutes for all tests
- [ ] **Performance:** 1K files in <10 seconds
- [ ] **Coverage:** >85% across all crates
- [ ] **Downloads:** Track weekly on crates.io
- [ ] **GitHub stars:** Monitor growth
- [ ] **Issues/PRs:** Response time <48 hours
- [ ] **Community parsers:** Track external implementations

---

## Notes & Decisions Log

Use this section to track important decisions and notes during migration:

### Decision 1: [Date]
**Question:** Should we use tree-sitter or native parsers?
**Decision:** Use native parsers (syn, rustpython-parser) for better type information
**Rationale:** Native parsers provide richer AST with type annotations

### Decision 2: [Date]
**Question:** Parallel parsing in trait or in implementation?
**Decision:** Default sequential in trait, optional parallel in implementation
**Rationale:** Allows flexibility, not all languages benefit from parallelism

### Decision 3: [Date]
**Question:** Version compatibility between parser-api and parsers?
**Decision:** Use `^0.1` for v0.x, strict major version match for v1.x
**Rationale:** Allow iteration in v0.x, enforce stability in v1.x

---

## Emergency Contacts

- **Lead Developer:** [Your contact]
- **CI/CD Issues:** [DevOps contact]
- **Crates.io Publishing:** [Account owner]
- **Documentation:** [Docs maintainer]

---

**Last Updated:** [Date]  
**Migration Status:** [Not Started / In Progress / Complete]
