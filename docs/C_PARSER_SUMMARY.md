# C Parser Implementation Summary

## Overview

The `codegraph-c` crate provides a C source code parser built on **tree-sitter** for extracting code entities and relationships. It's designed to work with both clean, well-formed C code and real-world code that may have syntax errors (e.g., Linux kernel code with custom macros).

## Architecture

```
codegraph-c/
├── lib.rs              # Public API and re-exports
├── parser_impl.rs      # CodeParser trait implementation
├── extractor.rs        # AST extraction with strict/tolerant modes
├── visitor.rs          # tree-sitter AST traversal
├── preprocessor.rs     # Macro simulation layer
└── mapper.rs           # IR to CodeGraph mapping
```

## Core Features

### Entities Extracted
- **Functions**: Name, signature, parameters, return type, visibility (static→private), complexity metrics
- **Structs**: Name, fields with types, line locations
- **Unions**: Same as structs, tagged with "union" attribute
- **Enums**: Name, enum constants with optional values
- **Includes**: System (`<>`) and local (`""`) includes

### Complexity Metrics
The parser calculates cyclomatic complexity by counting:
- Branches: `if`, `else`, `case`, `default`, `goto`, ternary `?:`
- Loops: `for`, `while`, `do-while`
- Logical operators: `&&`, `||`
- Nesting depth tracking

### Call Graph Extraction
When enabled (`extract_calls: true`), tracks:
- Function calls with callee name and line number
- Caller context (which function contains the call)
- Handles direct calls, field expressions (`obj->method`), and indirect calls

## Parsing Modes

### 1. Strict Mode (Default)
```rust
let ir = extract(source, Path::new("file.c"), &config)?;
```
- Fails on any syntax error
- Best for clean, well-formed code
- Returns `ParserError::SyntaxError` on errors

### 2. Tolerant Mode
```rust
let options = ExtractionOptions::tolerant();
let result = extract_with_options(source, path, &config, &options)?;
```
- Extracts entities even with syntax errors
- tree-sitter marks unparseable sections as ERROR nodes
- Visitor skips ERROR nodes but continues traversing children
- Returns `ExtractionResult` with `is_partial` and `error_count`

### 3. Kernel Mode
```rust
let options = ExtractionOptions::for_kernel_code();
let result = extract_with_options(source, path, &config, &options)?;
```
- Enables preprocessing + tolerant parsing + call extraction
- Best for Linux kernel and system code

## Preprocessor (Macro Simulation)

The preprocessor (`CPreprocessor`) provides lightweight source transformation to help tree-sitter parse code with common macros. It does **not** perform full macro expansion.

### What It Handles

1. **Type Macros** (45+ types recognized)
   - Kernel integers: `u8`, `u16`, `u32`, `u64`, `s8`, `s16`, `s32`, `s64`
   - Endian types: `__le16`, `__le32`, `__be16`, `__be32`
   - Standard types: `size_t`, `ssize_t`, `bool`, `ptrdiff_t`
   - Kernel types: `atomic_t`, `spinlock_t`, `gfp_t`, `dma_addr_t`

2. **Attribute Macros** (50+ stripped)
   - Section: `__init`, `__exit`, `__initdata`, `__devinit`
   - Compiler hints: `__cold`, `__hot`, `__pure`, `__always_inline`
   - Address space: `__user`, `__kernel`, `__iomem`, `__percpu`, `__rcu`
   - Unused: `__maybe_unused`, `__always_unused`
   - Packing: `__packed`, `__aligned`

3. **Preprocessor Directives**
   - Strips: `#define`, `#undef`, `#pragma`, `#error`, `#warning`
   - Preserves: `#include`, `#if`/`#ifdef`/`#else`/`#endif`

4. **Specific Macro Expansions**
   - `container_of(ptr, type, member)` → `((void*)ptr)`
   - `offsetof(type, member)` → `0`
   - `__attribute__((...))` → removed

### Macro Detection
```rust
let macros = preprocessor.analyze_macros(source);
// Returns Vec<MacroInfo> with name, kind, and expansion hint
```

## Test Results (Linux Kernel ICE Driver)

| Mode | Success Rate | Functions | Structs | Syntax Errors |
|------|-------------|-----------|---------|---------------|
| Strict | 27.4% | 322 | 1 | 0 (fails on error) |
| Tolerant | 100% | 3,679 | 54 | 1,394 |
| Kernel | 100% | 3,683 | 53 | 1,398 |

**Key Insight**: Tolerant mode extracts **11x more functions** than strict mode on kernel code.

## Limitations

### 1. No Actual Preprocessing
- Does **not** run a real C preprocessor
- Cannot expand user-defined macros
- Cannot handle conditional compilation (`#ifdef`) logic - code inside all branches is parsed

### 2. Syntax Error Sources (~814 errors in test)
| Category | % of Errors | Status |
|----------|-------------|--------|
| Preprocessor conditionals (`#ifdef`, `#else`) | 30.2% | **Unavoidable** - left as-is |
| `container_of` macro | 6.4% | **Mitigated** |
| Unused attributes (`__always_unused`) | 3.3% | **Mitigated** |
| `offsetof`/`sizeof` expressions | 3.1% | **Partially mitigated** |
| Unknown types (`u8` in parameters) | 2.8% | **Mitigated for declarations** |
| Various other macros | ~54% | **Not handled** |

### 3. Missing Header Information
- Cannot resolve types from external headers
- Function declarations without definitions are not tracked
- Type inference is limited to what's in the source file

### 4. Complex Macro Patterns Not Supported
- `DECLARE_*` / `DEFINE_*` macros (e.g., `DEFINE_MUTEX`)
- `LIST_HEAD` initializations
- Token pasting (`##`) and stringification (`#`)
- Inline assembly (`__asm__`)
- GCC statement expressions (`({ ... })`)

### 5. Call Graph Limitations
- Cannot resolve function pointers (marked as `(*indirect)`)
- Macro-defined function calls appear as macro names
- Calls inside preprocessor conditionals may be duplicated or missed

### 6. No Semantic Analysis
- No type checking or inference
- No control flow analysis beyond complexity counting
- No data flow analysis
- No alias analysis for pointers

### 7. Limited C++ Support
- Designed for C, not C++
- Will fail on C++ syntax (templates, classes, namespaces)

## Usage Examples

### Basic Extraction
```rust
use codegraph_c::CParser;
use codegraph_parser_api::CodeParser;

let parser = CParser::new();
let file_info = parser.parse_file(Path::new("main.c"), &mut graph)?;
```

### Tolerant Parsing for Kernel Code
```rust
use codegraph_c::extractor::{extract_with_options, ExtractionOptions};

let options = ExtractionOptions::for_kernel_code();
let result = extract_with_options(source, path, &config, &options)?;

if result.is_partial {
    println!("Parsed with {} errors", result.error_count);
}
println!("Found {} functions", result.ir.functions.len());
```

### Analyzing Macros
```rust
use codegraph_c::CPreprocessor;

let pp = CPreprocessor::new();
for macro_info in pp.analyze_macros(source) {
    println!("{}: {:?}", macro_info.name, macro_info.kind);
}
```

## Future Improvements

1. **Better macro handling**: Add more kernel macros to the preprocessor
2. **Configurable macro definitions**: Allow users to provide custom macro expansions
3. **Header file simulation**: Provide stub type declarations for common headers
4. **Partial preprocessing**: Handle simple `#define` constants
5. **Better error recovery**: Improve entity extraction around ERROR nodes
6. **C++ subset support**: Basic support for `extern "C"` and simple constructs
