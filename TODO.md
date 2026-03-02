Last updated: 2026-03-01

# codegraph-monorepo TODO

## High Priority

### 14. LSP `is_public` doesn't read parser `visibility` property
- All 13 parsers store `visibility` as a string ("public"/"protected"/"private")
- LSP server's `get_symbol_info` only checks boolean `is_public` or `exported`, defaults to `true`
- Result: Python `_protected_function` and `__private_function` both show `is_public: true`
- Fix: LSP should fall back to `visibility` string when `is_public` bool is absent
- Affects: `engine.rs` lines 405-409, 989-991, and `node_to_symbol_info`

## Medium Priority

### 6. Expand test coverage for recent features
- cpp: system include distinction (`is_system` property) — added in `1256a12`
- ruby: `require_relative` tracking — added in `1256a12`
- typescript: triple-slash reference directives — added in `1256a12`
- php: dynamic include/require resolution — added in `25c4dba`
- python: `is_external` fix — added in `25c4dba`
- rust: mod declaration import edges — added in `d92e2ae`
- Integration tests exist but could use edge-case coverage

### 7. codegraph-tcl grammar robustness
- tree-sitter-tcl uses a vendored ABI v15→v14 patched grammar
- Position-0 splitting is worked around with sibling-stitching in visitor
- Consider: upstream grammar fix, or switching to a maintained grammar fork
- Collapsed word_list and fragmented body issues are handled but fragile

### 15. Remaining parser capability gaps
- **TypeScript**: arrow functions stored as `arrow_function` instead of variable name (e.g., `fetchData`)
- **C#, Java, Kotlin, Ruby, Swift, PHP**: no parameter extraction, no return type extraction
- **C#, Java, PHP, Ruby, Swift**: no enum-as-class extraction
- **Kotlin, Ruby, Swift**: no ABC/trait detection from class hierarchy
- Use Rust parser as gold standard reference for feature parity

## Future

### 8. Additional language parsers
- No Lua, Perl, Scala, Dart, or Zig parsers yet
- Add based on demand / stellarion user requests

### 9. Cross-language import resolution
- Current parsers track imports within their own language
- No cross-language linking (e.g., Python calling C extensions, TypeScript importing WASM)

### 10. Graph algorithm extensions
- Current: BFS, DFS, cycle detection
- Potential: strongly connected components, topological sort, PageRank for code importance

### 11. Storage backend alternatives
- Current: RocksDB (production), BTreeMap (tests)
- Potential: SQLite for zero-config, PostgreSQL for multi-user

## Completed

### ~~13. Align parser capabilities across TypeScript, Python, Go, and C++~~
- TypeScript: enum extraction (as ClassEntity), arrow function async detection (`4384d7d`)
- Python: enum detection from base classes, ABC/Protocol → TraitEntity, visibility from name prefixes
- Go: parameter extraction (incl. variadic), return type, receiver type, visibility, test detection
- C++: C++20 coroutine detection via co_await/co_return/co_yield and return type patterns
- Versions bumped: TS 0.4.2, Python 0.4.3, Go 0.1.6, C++ 0.2.2

### ~~12. Missing Calls edge extraction in Rust and Go parsers~~
- Rust: added call_expression handling in visitor, including macro invocation bodies (`78fad1d`, `be9a66f`)
- Go: added call extraction (`78fad1d`)
- C: was already fixed separately

### ~~1. Fix codegraph-c `test_parse_syntax_error` test~~
- Fixed: switched `parse_source()` from tolerant mode to strict `extract()`, matching other parsers
- All 144 codegraph-c tests now pass

### ~~2. Add import relationships in codegraph-python builder~~
- Fixed: `parser_impl.rs` already had import mapping; added matching implementation to `builder.rs`
- Module nodes created with `is_external` flag, Imports edges with alias/symbols/wildcard properties

### ~~3. Complete async and decorator capture in codegraph-python~~
- Fixed: `parser_impl.rs` stored `is_async`/`is_static` as strings instead of bools; now uses proper types
- Added `is_test` and `attributes` (decorators) properties to function nodes
- Extractor now assigns decorators to `func.attributes`
- Test assertions completed for both async detection and decorator extraction

### ~~4. Consistent error handling for syntax errors across all parsers~~
- Added `has_error()` check to cpp, csharp, java, kotlin, php, ruby, swift extractors (`f34b749`)
- TCL intentionally excluded — vendored grammar produces ERROR nodes for keywords, resolved by sibling-stitching
- Removed misleading "tolerant parsing" comment from PHP extractor
- 12 of 13 parsers now enforce strict syntax error checking; TCL documented as exception
- Added `test_syntax_error` integration tests to all 7 parsers

### ~~5. Dead `#[allow(dead_code)]` across visitor modules~~
- Removed unused `config: ParserConfig` field from 11 visitors (all except Rust) (`9e396b1`)
- Removed debug `print_tree`/`dump_ast` helpers + their test callers from Kotlin, Ruby, Tcl
- Removed dead Python `VisitorContext`, `type_to_string`, `default_to_string`
- Updated C `extract_with_options` signature (removed config param) and doctest
- 3 intentional annotations remain: EdaCommand enum (write-only fields), C Ifdef/Ifndef fields
- 25 files changed, -249 lines net

## Workspace Health

- **Total tests**: 1098 passing, 0 failing, 4 ignored
- **Crates**: 15 (core + parser-api + 13 language parsers)
- **Clippy**: clean
- **Recent features**: cyclomatic complexity (all 13 parsers), TypeReference IR, cross-file type resolution
