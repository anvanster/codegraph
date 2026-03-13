# Tree-Sitter Upgrade Impact Assessment: 0.22 → 0.24+

**Date**: 2026-03-13
**Scope**: All 13 language parsers in codegraph-monorepo + codegraph-vscode extension
**Target**: tree-sitter 0.24.x (recommended over 0.25/0.26)

## Motivation

- tree-sitter-c 0.23+ on tree-sitter 0.24 core provides improved error recovery, beneficial for ESXi driver code parsing
- Current tree-sitter-rust is pinned to `=0.20.3` due to duplicate symbol issues — upgrading the core resolves this
- Newer grammar versions improve handling of GCC extensions and non-standard C patterns

## Current State: 13 Parsers on tree-sitter 0.22

| Parser | Language Crate | Version | Language Init Pattern | Risk |
|--------|---------------|---------|----------------------|------|
| C | tree-sitter-c | 0.21 | `tree_sitter_c::language()` | Low |
| C++ | tree-sitter-cpp | 0.22 | `tree_sitter_cpp::language()` | Low |
| C# | tree-sitter-c-sharp | 0.21 | `tree_sitter_c_sharp::language()` | Low |
| Go | tree-sitter-go | 0.21 | `tree_sitter_go::language()` | Low |
| Java | tree-sitter-java | 0.21 | `tree_sitter_java::language()` | Low |
| Kotlin | tree-sitter-kotlin | 0.3 | `tree_sitter_kotlin::language()` | Medium |
| PHP | tree-sitter-php | 0.22 | `tree_sitter_php::language_php()` | Low |
| Python | tree-sitter-python | 0.21 | `tree_sitter_python::language()` | Low |
| Ruby | tree-sitter-ruby | 0.21 | `tree_sitter_ruby::language()` | Low |
| **Rust** | tree-sitter-rust | **=0.20.3 (pinned)** | `tree_sitter_rust::language()` | **High** |
| Swift | tree-sitter-swift | 0.5 | `tree_sitter_swift::language()` | Medium |
| TypeScript | tree-sitter-typescript | 0.21 | `language_typescript()` / `language_tsx()` | Low |
| **TCL** | **vendored (custom)** | **ABI v14** | `Language::from_raw(tree_sitter_tcl())` | **High** |

## Breaking API Changes (0.22 → 0.24+)

### 1. Language Type Change (Critical — affects ALL parsers)

In tree-sitter 0.24+, grammar crates switched from exporting a `language()` function returning `tree_sitter::Language` to exporting a `LANGUAGE` constant of type `tree_sitter_language::LanguageFn`. You must call `.into()` to convert:

```rust
// Current (0.22):
let language = tree_sitter_c::language();
parser.set_language(&language)?;

// After (0.24+):
let language: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();
parser.set_language(&language)?;
```

**Impact**: Every `extractor.rs` and every test helper in every `visitor.rs` needs this change. ~60+ call sites across 13 parsers.

**Effort**: Mechanical find-and-replace. ~2-3 hours.

### 2. `Language::from_raw()` Change (Critical — affects TCL)

The vendored TCL grammar uses `Language::from_raw()` which changed signature in 0.24. The new API uses `LanguageFn` or requires different FFI bridging.

```rust
// Current ts_tcl.rs:
unsafe { tree_sitter::Language::from_raw(tree_sitter_tcl() as *const _) }
```

This needs adaptation to the new `Language` construction API. The vendored `parser.c` and `scanner.c` also need ABI compatibility verification — the grammar currently targets ABI v14, and tree-sitter 0.24+ still supports v14 but the Rust binding path changed.

**Impact**: `ts_tcl.rs` rewrite + verify vendored C source ABI compatibility + test ERROR node stitching still works.

**Effort**: 4-8 hours (including testing the error resolution pipeline).

### 3. `parse()` Signature (No impact)

`parser.parse(source, None)` remains stable through 0.24. In 0.25+, `parse_with()` was deprecated in favor of `parse_with_options()`, but codegraph does not use `parse_with()` anywhere — only `parse()`, which is unaffected.

### 4. Node API (No change)

All Node methods used across the codebase remain stable:
- `node.kind()`, `node.children()`, `node.walk()`
- `node.child_by_field_name()`, `node.utf8_text()`
- `node.is_error()`, `node.has_error()`, `node.is_missing()`
- `node.start_position()`, `node.end_position()`

## Per-Parser Risk Assessment

### HIGH RISK

#### TCL (codegraph-tcl) — Effort: 1-2 days

- Vendored grammar with custom `build.rs` + FFI bindings in `ts_tcl.rs`
- `Language::from_raw()` API change needs adaptation
- Must verify ABI v14 vendored grammar still loads correctly with new tree-sitter core
- Must regression-test the ERROR node → keyword resolution pipeline:
  - `resolve_error_keyword()` in `visitor.rs`
  - `resolve_error_keyword_with_source()` for SDC/EDA commands
  - Sibling stitching logic (lines 177-214)
- The v14/v15 split behavior may change if tree-sitter's error recovery improved — could affect which nodes produce ERROR vs proper parse nodes
- **Key risk**: Error resolution logic depends on specific ERROR node placement; grammar behavior might subtly change with new tree-sitter core

#### Rust (codegraph-rust) — Effort: 4-8 hours

- Currently pinned to `=0.20.3` because `0.20.4+` pulled in tree-sitter 0.20 causing duplicate symbols
- Upgrading tree-sitter core to 0.24 means tree-sitter-rust MUST also be upgraded to a 0.24-compatible version
- The pin reason (duplicate symbols) should be resolved since both will target the same tree-sitter version
- **Key risk**: If tree-sitter-rust grammar changed node names between 0.20.3 and latest, visitor `match` arms may miss nodes or match incorrectly

### MEDIUM RISK

#### Kotlin (codegraph-kotlin) — Effort: 2-4 hours

- tree-sitter-kotlin 0.3 is a third-party grammar (not official tree-sitter-grammars org)
- Need to verify a 0.24-compatible version exists on crates.io
- May need to find alternative crate or vendor the grammar

#### Swift (codegraph-swift) — Effort: 2-4 hours

- tree-sitter-swift 0.5 is also third-party
- Same concern: need a version targeting tree-sitter 0.24+
- If no compatible version exists, must vendor or fork

#### TypeScript (codegraph-typescript) — Effort: 1-2 hours

- Uses `language_typescript()` and `language_tsx()` — need to verify these still exist or became `LANGUAGE_TYPESCRIPT` / `LANGUAGE_TSX` constants
- Likely straightforward but has two language variants to update

#### PHP (codegraph-php) — Effort: 1-2 hours

- Uses `language_php()` — similar to TypeScript, verify if the multi-language pattern changed to a constant

### LOW RISK (7 parsers)

C, C++, C#, Go, Java, Python, Ruby — Effort: ~30 min each

- Standard `language()` → `LANGUAGE.into()` change
- Official grammar crates, well-maintained, will have 0.24-compatible versions
- Pure mechanical update

## Grammar Node Name Changes (Semantic Risk)

Upgrading language grammars may change AST node names. Visitors use `match node.kind()` extensively. Key areas to verify:

| Parser | Risk | Notes |
|--------|------|-------|
| Rust | Must verify ~50 match arms in `visitor.rs` | Jumping from 0.20.3 to latest — largest version gap |
| Kotlin | Third-party grammar, could rename freely | Check changelog carefully |
| Swift | Third-party grammar, could rename freely | Check changelog carefully |
| C | Low — `function_definition`, `struct_specifier` etc. stable | Official grammar, minor version bump |
| Others (official) | Low — node names generally stable across minor versions | Spot-check after upgrade |

## Files Requiring Changes

### Every parser (13 crates) — extractor.rs

```
crates/codegraph-{lang}/src/extractor.rs  — language() → LANGUAGE.into()
```

### Parsers with test helpers in visitor.rs

```
crates/codegraph-c/src/visitor.rs:698         — tree_sitter_c::language()
crates/codegraph-cpp/src/visitor.rs:989       — tree_sitter_cpp::language()
crates/codegraph-csharp/src/visitor.rs:961    — tree_sitter_c_sharp::language()
crates/codegraph-csharp/src/visitor.rs:1045   — tree_sitter_c_sharp::language()
crates/codegraph-go/src/visitor.rs            — ~20 call sites in tests
crates/codegraph-java/src/visitor.rs:874      — tree_sitter_java::language()
crates/codegraph-kotlin/src/visitor.rs:1063   — tree_sitter_kotlin::language()
crates/codegraph-ruby/src/visitor.rs:651      — tree_sitter_ruby::language()
crates/codegraph-rust/src/visitor.rs:945      — tree_sitter_rust::language()
crates/codegraph-swift/src/visitor.rs:958     — tree_sitter_swift::language()
crates/codegraph-tcl/src/visitor.rs:1648      — crate::ts_tcl::language()
```

### TCL-specific files

```
crates/codegraph-tcl/src/ts_tcl.rs            — Language::from_raw() rewrite
crates/codegraph-tcl/build.rs                 — verify tree-sitter header path
crates/codegraph-tcl/tree-sitter-tcl-src/     — verify ABI compatibility of vendored C sources
```

### Cargo.toml files (all 13 crates)

```
crates/codegraph-{lang}/Cargo.toml            — bump tree-sitter and tree-sitter-{lang} versions
```

### Workspace root

```
Cargo.toml                                    — if tree-sitter is a workspace dependency
```

## Aggregate Effort Estimate

| Category | Effort | Confidence |
|----------|--------|------------|
| Core API migration (`language()` → `LANGUAGE.into()`) across all parsers | 3-4 hours | High |
| TCL vendored grammar + FFI + error pipeline testing | 1-2 days | Medium |
| Rust parser unpin + upgrade + node name verification | 4-8 hours | Medium |
| Kotlin/Swift third-party grammar compatibility check | 4-8 hours | Low |
| TypeScript/PHP multi-language variant check | 2-4 hours | High |
| Full regression test suite across all 13 parsers | 1 day | High |
| **Total** | **4-6 days** | Medium |

## Why Target 0.24 (Not 0.25 or 0.26)

1. **0.24 has the `Language`/`LanguageFn` change but keeps `parse()` stable** — no further API churn
2. **0.25+ deprecates `parse_with()`** — not used here, but grammar crates may have internal changes
3. **0.24 is what Zed editor migrated to first** — meaning more grammar crates are tested against it
4. **0.25 had WASM ABI issues** (tree-sitter/tree-sitter#3938) — caused out-of-bounds memory accesses with older grammar ABIs
5. **tree-sitter-c 0.23+ on 0.24 core** gives improved error recovery for ESXi code without the risks of bleeding edge

## Recommended Migration Order

1. **Low-risk parsers first** (C, C++, Go, Java, Python, Ruby, C#) — build confidence, verify the `LANGUAGE.into()` pattern works
2. **TypeScript, PHP** — verify multi-language constant patterns
3. **Rust** — unpin and upgrade, verify node names
4. **Kotlin, Swift** — third-party compatibility; may require vendoring
5. **TCL** — last and most complex; needs dedicated testing of error resolution pipeline

## Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Kotlin/Swift grammar crates don't have 0.24 versions | Medium | Blocks 2 parsers | Vendor the grammars like TCL |
| TCL error stitching breaks with new error recovery | Low-Medium | Breaks Tcl parsing quality | Dedicated test suite with known ERROR patterns |
| Rust node names changed | Low | Misses entities in Rust files | Diff node-types.json between 0.20.3 and latest |
| tree-sitter 0.24 vendored C header path changed | Low | TCL build.rs fails | Check `DEP_TREE_SITTER_INCLUDE` behavior |
| Regression in parse quality for any language | Low | Silent extraction failures | Run full test suite + compare entity counts on real codebases |

## References

- [tree-sitter releases](https://github.com/tree-sitter/tree-sitter/releases)
- [Zed tree-sitter 0.24 migration PR #24492](https://github.com/zed-industries/zed/pull/24492)
- [Zed tree-sitter 0.25 migration PR #24340](https://github.com/zed-industries/zed/pull/24340)
- [tree-sitter versioning conflict issue #3095](https://github.com/tree-sitter/tree-sitter/issues/3095)
- [tree-sitter 0.22.6 semver breakage #3487](https://github.com/tree-sitter/tree-sitter/issues/3487)
- [tree-sitter v0.21.0 Language API change](https://github.com/tree-sitter/tree-sitter/releases/tag/v0.21.0)
- [tree-sitter-python 0.23 breakage #280](https://github.com/tree-sitter/tree-sitter-python/issues/280)
- [tree-sitter Rust API docs](https://docs.rs/tree-sitter)
