#!/bin/bash
# CI Checks - Run this before pushing to ensure CI passes

set -e  # Exit on first error

echo "============================================"
echo "Running CI checks locally..."
echo "============================================"
echo ""

# Function to print section headers
print_section() {
    echo ""
    echo "============================================"
    echo "$1"
    echo "============================================"
}

# 1. Clippy
print_section "1/5 Running Clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# 2. Format check
print_section "2/5 Checking formatting..."
cargo fmt --all -- --check

# 3. Benchmarks
print_section "3/5 Running benchmarks..."
cargo bench --no-fail-fast

# 4. Documentation
print_section "4/5 Building documentation..."
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# 5. Code coverage (optional - only if tarpaulin is installed)
print_section "5/5 Running code coverage..."
if command -v cargo-tarpaulin &> /dev/null; then
    cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Xml
else
    echo "⚠️  cargo-tarpaulin not installed. Skipping coverage check."
    echo "   Install with: cargo install cargo-tarpaulin"
fi

echo ""
echo "============================================"
echo "✅ All CI checks passed!"
echo "============================================"
