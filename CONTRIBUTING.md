# Contributing to codegraph

Thank you for your interest in contributing to codegraph! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to the Rust Code of Conduct. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Constitution

Before contributing, please read our [Constitution](.github/prompts/speckit.constitution.prompt.md) which defines our core principles and governance model.

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git

### Clone and Build

```bash
git clone https://github.com/anvanster/codegraph.git
cd codegraph
cargo build
cargo test
```

## Contribution Process

### 1. Test-Driven Development (Required)

All code MUST be developed using TDD methodology:

1. **RED**: Write failing test first
2. **GREEN**: Implement minimum code to pass
3. **REFACTOR**: Clean up while keeping tests green

**Pull requests without tests will be rejected.**

### 2. Code Quality Standards

Before submitting:

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy -- -D warnings

# Run all tests
cargo test

# Check documentation
cargo doc --no-deps
```

All checks must pass.

### 3. Pull Request Process

1. **Fork** the repository
2. **Create a branch** for your feature/fix
   ```bash
   git checkout -b feature/my-new-feature
   ```
3. **Write tests first** (TDD)
4. **Implement** your changes
5. **Ensure all tests pass**
6. **Commit** with clear messages
   ```bash
   git commit -m "Add feature: description"
   ```
7. **Push** to your fork
   ```bash
   git push origin feature/my-new-feature
   ```
8. **Open a Pull Request** with:
   - Clear description of changes
   - Link to related issue (if any)
   - Test coverage information
   - Performance impact (if applicable)

### 4. Commit Message Guidelines

Follow conventional commits:

```
<type>: <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Adding tests
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Maintenance tasks

Examples:
```
feat: add GraphML export format
fix: prevent panic on empty graph traversal
docs: update README with parser examples
test: add integration tests for RocksDB backend
```

## Types of Contributions

### Bug Reports

Open an issue with:
- Clear description of the bug
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version)
- Minimal code example

### Feature Requests

Open an issue with:
- Clear use case description
- Proposed API design
- Alignment with constitution principles
- Alternative approaches considered

For major features, an RFC may be required.

### Code Contributions

#### Level 1: Minor Changes (Bug fixes, docs, tests)
- Any maintainer can approve and merge
- No RFC needed
- Keep PR description clear

#### Level 2: Feature Additions (New APIs, helpers)
- RFC as GitHub issue required
- At least 2 maintainer approvals
- 7-day comment period
- Update docs and tests

#### Level 3: Breaking Changes (API changes, removals)
- Formal RFC required
- All maintainers must approve
- 14-day comment period
- Deprecation cycle required
- Migration guide mandatory

## Architecture Guidelines

### Layered Design

Respect the layer boundaries:
```
User Tools (you build this)
    ↓
Code Helpers (convenience API)
    ↓
Query Builder (fluent interface)
    ↓
Core Graph (nodes, edges)
    ↓
Storage Backend (RocksDB, memory)
```

### Zero Magic Rule

❌ **Bad** (Magic):
```rust
// Auto-detects language, scans directory, parses files
let graph = CodeGraph::from_directory("./src")?;
```

✅ **Good** (Explicit):
```rust
let mut graph = CodeGraph::open("./project.graph")?;
let file_id = graph.add_file(Path::new("main.rs"), "rust")?;
// User explicitly parses and adds entities
```

### Error Handling

Always use `Result<T>` for fallible operations:

```rust
pub fn get_node(&self, id: &NodeId) -> Result<Node> {
    // Implementation
}
```

Never panic in library code. Use proper error types.

### Documentation

Every public item must be documented:

```rust
/// Get a node by ID.
///
/// # Errors
///
/// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
///
/// # Example
///
/// ```
/// let node = graph.get_node(&node_id)?;
/// ```
pub fn get_node(&self, id: &NodeId) -> Result<Node> {
    // Implementation
}
```

## Testing Guidelines

### Coverage Requirements

- Core graph: 95%+
- Storage backends: 90%+
- Helpers: 85%+
- Export functions: 80%+

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_behavior() {
        // Arrange
        let mut graph = CodeGraph::in_memory().unwrap();
        
        // Act
        let result = graph.add_node(node);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
use codegraph::*;

#[test]
fn test_end_to_end_workflow() {
    // Test complete workflows
}
```

## Performance Benchmarks

For performance-critical changes, include benchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_add_node(c: &mut Criterion) {
    c.bench_function("add_node", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

criterion_group!(benches, benchmark_add_node);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
```

## Release Process

Maintainers follow this process for releases:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag: `v0.x.y`
5. Push to crates.io: `cargo publish`
6. Create GitHub release with notes

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: Open a GitHub Issue
- **Security**: Email maintainers directly (see SECURITY.md)

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project README

Thank you for contributing to codegraph! 🎉
