#!/usr/bin/env bash
# Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# CI Pre-commit Script for codegraph-python
# Run this before committing to main to ensure code quality

echo "🔍 Running CI checks..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILED=0

# Function to run a check
run_check() {
    local name="$1"
    local cmd="$2"
    
    echo -e "${YELLOW}▶ $name${NC}"
    if eval "$cmd"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        echo ""
        return 0
    else
        echo -e "${RED}✗ $name failed${NC}"
        echo ""
        FAILED=1
        return 1
    fi
}

# 1. Clippy - Lint checks
run_check "Clippy (strict mode)" \
    "cargo clippy --all-targets --all-features -- -D warnings"

# 2. Format check
run_check "Format check" \
    "cargo fmt --all -- --check"

# 3. Tests
run_check "Tests" \
    "cargo test --all-features"

# 4. Benchmarks
run_check "Benchmarks" \
    "cargo bench --no-fail-fast"

# 5. Documentation
run_check "Documentation" \
    "cargo doc --no-deps --all-features"

# 6. Coverage (requires cargo-tarpaulin)
if command -v cargo-tarpaulin &> /dev/null; then
    run_check "Code coverage" \
        "cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Xml"
else
    echo -e "${YELLOW}⚠ Skipping coverage - cargo-tarpaulin not installed${NC}"
    echo "  Install with: cargo install cargo-tarpaulin"
    echo ""
fi

# Summary
echo "================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All CI checks passed!${NC}"
    echo "Ready to commit to main."
    exit 0
else
    echo -e "${RED}✗ Some CI checks failed.${NC}"
    echo "Please fix the issues before committing."
    exit 1
fi
