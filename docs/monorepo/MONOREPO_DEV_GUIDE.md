# CodeGraph Monorepo Development Guide

## Overview

This guide covers best practices for developing in the CodeGraph monorepo workspace.

---

## Workspace Structure

```
codegraph/
├── .github/
│   └── workflows/           # CI/CD workflows
├── crates/
│   ├── codegraph/           # Core database
│   ├── codegraph-parser-api/# Shared parser API
│   ├── codegraph-python/    # Python parser
│   ├── codegraph-rust/      # Rust parser
│   └── codegraph-typescript/# TypeScript parser
├── docs/                    # Documentation
├── examples/                # Example projects
├── scripts/                 # Development scripts
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Locked dependencies
├── README.md
├── ARCHITECTURE.md
└── CONTRIBUTING.md
```

---

## Common Commands

### Building

```bash
# Build entire workspace
cargo build --workspace

# Build specific crate
cargo build -p codegraph-python

# Build with optimizations
cargo build --workspace --release

# Build only parser-api and its dependents
cargo build -p codegraph-parser-api --workspace
```

### Testing

```bash
# Test entire workspace
cargo test --workspace

# Test specific crate
cargo test -p codegraph-rust

# Test with output
cargo test --workspace -- --nocapture

# Test specific test
cargo test -p codegraph-python test_parse_function

# Test with coverage (requires tarpaulin)
cargo tarpaulin --workspace --out Html
```

### Code Quality

```bash
# Check formatting
cargo fmt --all -- --check

# Fix formatting
cargo fmt --all

# Run clippy (linter)
cargo clippy --workspace -- -D warnings

# Fix clippy warnings
cargo clippy --workspace --fix

# Check for outdated dependencies
cargo outdated --workspace
```

### Documentation

```bash
# Generate and open docs for entire workspace
cargo doc --workspace --open

# Generate docs for specific crate
cargo doc -p codegraph-parser-api --open

# Check doc tests
cargo test --workspace --doc

# Check for broken links in docs
cargo deadlinks --check-http
```

### Publishing

```bash
# Dry run publish (don't actually publish)
cargo publish -p codegraph-parser-api --dry-run

# Publish a crate
cargo publish -p codegraph-parser-api

# Publish sequence (in order of dependencies)
./scripts/publish.sh
```

---

## Development Workflow

### Daily Development

1. **Pull latest changes**
   ```bash
   git pull origin main
   cargo build --workspace  # Ensure everything builds
   ```

2. **Create feature branch**
   ```bash
   git checkout -b feature/add-go-parser
   ```

3. **Make changes**
   - Edit code
   - Write tests
   - Update documentation

4. **Verify changes**
   ```bash
   cargo build --workspace
   cargo test --workspace
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   ```

5. **Commit changes**
   ```bash
   git add .
   git commit -m "feat(rust): Add support for async functions"
   ```

6. **Push and create PR**
   ```bash
   git push origin feature/add-go-parser
   # Create PR on GitHub
   ```

### Working on Multiple Crates

When changes span multiple crates:

```bash
# Example: Add new entity type to parser-api and use it in Python parser

# 1. Add entity to parser-api
cd crates/codegraph-parser-api
# Edit src/entities/...
cargo test  # Test just this crate

# 2. Update Python parser
cd ../codegraph-python
# Edit to use new entity
cargo test  # Test just this crate

# 3. Test entire workspace
cd ../..
cargo test --workspace

# 4. Commit atomically (single commit for related changes)
git add crates/codegraph-parser-api crates/codegraph-python
git commit -m "feat(parser-api): Add NamespaceEntity, use in Python parser"
```

### Adding a New Parser

1. **Create crate**
   ```bash
   cd crates
   cargo new codegraph-go --lib
   cd ..
   ```

2. **Add to workspace**
   Edit `Cargo.toml`:
   ```toml
   members = [
       "crates/codegraph",
       "crates/codegraph-parser-api",
       "crates/codegraph-python",
       "crates/codegraph-rust",
       "crates/codegraph-typescript",
       "crates/codegraph-go",  # NEW
   ]
   ```

3. **Set up Cargo.toml**
   ```toml
   [package]
   name = "codegraph-go"
   version = "0.1.0"
   edition.workspace = true
   license.workspace = true
   
   [dependencies]
   codegraph.workspace = true
   codegraph-parser-api.workspace = true
   tree-sitter = "0.20"
   tree-sitter-go = "0.20"
   ```

4. **Implement parser**
   - Create `src/parser.rs`
   - Implement `CodeParser` trait
   - Write tests

5. **Document**
   - Add README
   - Add examples
   - Update workspace README

6. **Test and commit**
   ```bash
   cargo build -p codegraph-go
   cargo test -p codegraph-go
   git add crates/codegraph-go Cargo.toml
   git commit -m "feat(go): Add Go language parser"
   ```

---

## Best Practices

### Dependency Management

**DO:**
- Use workspace dependencies for shared crates
- Keep dependencies up to date
- Minimize dependency count
- Use specific versions for parser libraries

**DON'T:**
- Add dependencies without review
- Use deprecated crates
- Create circular dependencies

```toml
# Good: Use workspace dependency
[dependencies]
serde.workspace = true

# Bad: Hardcoded version
[dependencies]
serde = "1.0"
```

### Versioning

**Rules:**
1. parser-api version changes require parser updates
2. Database version changes are rare (backward compatible)
3. Parser versions can change independently
4. Use semantic versioning strictly

**Version Compatibility:**
```
codegraph-parser-api: 0.1.x
  ├─ codegraph-python: 0.2.x (compatible)
  ├─ codegraph-rust: 0.1.x (compatible)
  └─ codegraph-typescript: 0.1.x (compatible)

codegraph-parser-api: 0.2.x
  ├─ codegraph-python: 0.3.x (needs update)
  └─ ...
```

### Testing Strategy

**Test Hierarchy:**
1. **Unit tests** - Each module (run with `cargo test`)
2. **Integration tests** - Cross-module (in `tests/` directory)
3. **Doc tests** - Examples in documentation
4. **Benchmark tests** - Performance (in `benches/` directory)

**Coverage Target:** >85% for all crates

```rust
// Unit test example
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_entity() {
        let func = FunctionEntity::new("test", 1, 10);
        assert_eq!(func.name, "test");
    }
}

// Integration test example (tests/integration_test.rs)
use codegraph::CodeGraph;
use codegraph_python::PythonParser;
use codegraph_parser_api::CodeParser;

#[test]
fn test_parse_real_project() {
    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = PythonParser::new();
    // Test full workflow
}
```

### Documentation Standards

**Every public item must have:**
- Summary line
- Description (if non-trivial)
- Example (if useful)
- Errors (if applicable)

```rust
/// Parse a Python source file and extract entities
///
/// This function reads the file, parses the Python AST, extracts
/// all functions, classes, and relationships, and inserts them
/// into the provided graph.
///
/// # Arguments
/// * `path` - Path to the Python source file
/// * `graph` - Mutable reference to the CodeGraph
///
/// # Returns
/// `FileInfo` containing metadata about extracted entities
///
/// # Errors
/// Returns `ParserError` if:
/// - File cannot be read
/// - Source has syntax errors
/// - Graph insertion fails
///
/// # Example
/// ```rust
/// use codegraph::CodeGraph;
/// use codegraph_python::PythonParser;
/// use codegraph_parser_api::CodeParser;
///
/// let mut graph = CodeGraph::in_memory()?;
/// let parser = PythonParser::new();
/// let info = parser.parse_file("src/main.py", &mut graph)?;
/// println!("Parsed {} functions", info.functions.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_file(&self, path: &Path, graph: &mut CodeGraph) -> Result<FileInfo, ParserError> {
    // Implementation
}
```

### Commit Message Convention

Use conventional commits:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `refactor` - Code restructuring
- `test` - Adding tests
- `chore` - Maintenance

**Scopes:**
- `parser-api` - Changes to parser API
- `python` - Python parser changes
- `rust` - Rust parser changes
- `database` - Core database changes
- `ci` - CI/CD changes

**Examples:**
```
feat(python): Add support for async comprehensions

Add parsing logic for async generator expressions and 
async list/dict/set comprehensions introduced in Python 3.6.

Closes #123

---

fix(parser-api): Correct visibility detection for protected fields

The previous logic incorrectly classified single-underscore fields
as private instead of protected.

---

docs(readme): Update installation instructions

---

chore(deps): Update rustpython-parser to 0.3.1
```

---

## Performance Guidelines

### Optimization Principles

1. **Measure first** - Always benchmark before optimizing
2. **Focus on hot paths** - Profile to find bottlenecks
3. **Batch operations** - Insert nodes/edges in batches
4. **Parallel where possible** - Use rayon for file parsing
5. **Avoid clones** - Use references when possible

### Benchmarking

```bash
# Run benchmarks
cargo bench --workspace

# Run specific benchmark
cargo bench -p codegraph-python parse_1000_files

# Compare benchmarks
cargo bench --workspace -- --save-baseline main
# Make changes
cargo bench --workspace -- --baseline main
```

### Performance Targets

| Operation | Target | Current |
|-----------|--------|---------|
| Parse 1K files | <10s | TBD |
| Parse 10K files | <100s | TBD |
| Single file (100 LOC) | <10ms | TBD |
| Graph insertion (1K nodes) | <100ms | TBD |

---

## CI/CD Pipeline

### Workflow Overview

1. **On Push/PR:**
   - Compile all crates
   - Run all tests
   - Check formatting
   - Run clippy
   - Generate documentation

2. **On Tag:**
   - Run full test suite
   - Publish to crates.io (in order)
   - Create GitHub release
   - Deploy documentation

### Local Pre-Push Check

Create `scripts/pre-push.sh`:

```bash
#!/bin/bash
set -e

echo "🔨 Building workspace..."
cargo build --workspace

echo "🧪 Running tests..."
cargo test --workspace

echo "📝 Checking formatting..."
cargo fmt --all -- --check

echo "🔍 Running clippy..."
cargo clippy --workspace -- -D warnings

echo "📚 Checking documentation..."
cargo doc --workspace --no-deps

echo "✅ All checks passed!"
```

Make it executable and run before pushing:
```bash
chmod +x scripts/pre-push.sh
./scripts/pre-push.sh
```

---

## Debugging Tips

### Build Issues

**Problem:** Crate fails to build after updating parser-api

**Solution:**
```bash
# Clean build artifacts
cargo clean

# Update lock file
cargo update

# Rebuild
cargo build --workspace
```

### Test Failures

**Problem:** Tests fail in CI but pass locally

**Solution:**
```bash
# Run tests with same flags as CI
cargo test --workspace --release

# Check for timing issues
cargo test --workspace -- --test-threads=1
```

### Performance Issues

**Problem:** Parser is slow on large files

**Solution:**
```bash
# Profile the parser
cargo build --release -p codegraph-python
perf record ./target/release/codegraph-python-bench
perf report

# Use cargo-flamegraph
cargo install flamegraph
cargo flamegraph --bench parse_large_file
```

### Dependency Issues

**Problem:** Conflicting dependency versions

**Solution:**
```bash
# Check dependency tree
cargo tree -p codegraph-python

# Find duplicates
cargo tree -d

# Update specific dependency
cargo update -p serde
```

---

## Release Process

### Pre-Release Checklist

- [ ] All tests pass on CI
- [ ] Documentation is up to date
- [ ] CHANGELOG.md updated
- [ ] Version numbers bumped
- [ ] Examples verified
- [ ] Performance benchmarks run

### Publishing Order

Must publish in dependency order:

1. **codegraph** (if changed)
2. **codegraph-parser-api** (if changed)
3. **codegraph-python**
4. **codegraph-rust**
5. **codegraph-typescript**
6. Other parsers...

### Publish Script

Create `scripts/publish.sh`:

```bash
#!/bin/bash
set -e

echo "Publishing codegraph-parser-api..."
cd crates/codegraph-parser-api
cargo publish --dry-run
read -p "Publish parser-api? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "Waiting for crates.io indexing..."
    sleep 30
fi

echo "Publishing codegraph-python..."
cd ../codegraph-python
cargo publish --dry-run
read -p "Publish python? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
fi

# Continue for other crates...
```

---

## Troubleshooting

### Common Issues

#### "Package not found on crates.io"

**Cause:** Just published, not yet indexed

**Fix:** Wait 30-60 seconds and retry

#### "Workspace member has different version"

**Cause:** Version mismatch in Cargo.toml

**Fix:** Ensure all references use same version

#### "Cannot find parser-api in workspace"

**Cause:** Not added to workspace members

**Fix:** Add to `Cargo.toml` members array

#### "Clippy warnings on CI but not local"

**Cause:** Different clippy versions

**Fix:** Run `rustup update` and retry

---

## Resources

### Documentation
- [Cargo Workspace Documentation](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)

### Tools
- [cargo-outdated](https://github.com/kbknapp/cargo-outdated) - Check for outdated deps
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit) - Security audits
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) - Code coverage
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) - Profiling

### External Examples
- [rust-analyzer](https://github.com/rust-lang/rust-analyzer) - Large Rust monorepo
- [tokio](https://github.com/tokio-rs/tokio) - Well-organized workspace
- [serde](https://github.com/serde-rs/serde) - Multiple related crates

---

## Getting Help

- **General Questions:** Open a [GitHub Discussion](https://github.com/anvanster/codegraph/discussions)
- **Bug Reports:** Open a [GitHub Issue](https://github.com/anvanster/codegraph/issues)
- **Security Issues:** Email security@codegraph.dev
- **Feature Requests:** Open a [GitHub Issue](https://github.com/anvanster/codegraph/issues) with `enhancement` label

---

**Happy Coding! 🚀**
