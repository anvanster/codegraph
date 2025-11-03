# CI Scripts

This directory contains scripts to run CI checks locally before pushing.

## ci-checks.sh

Runs all CI checks that are performed in GitHub Actions:

1. **Clippy** - Linting with all warnings as errors
2. **Format Check** - Code formatting verification
3. **Benchmarks** - Performance benchmarks
4. **Documentation** - Documentation build with warnings as errors
5. **Coverage** - Code coverage (optional, requires cargo-tarpaulin)

### Usage

```bash
# Run all checks
./scripts/ci-checks.sh
```

### Install Optional Dependencies

For code coverage:

```bash
cargo install cargo-tarpaulin
```

### Exit Codes

- `0` - All checks passed
- Non-zero - One or more checks failed

The script stops at the first failing check.
