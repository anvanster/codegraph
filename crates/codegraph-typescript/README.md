# codegraph-typescript

TypeScript/JavaScript parser for CodeGraph - extracts code entities and relationships from TS/JS source files.

## Features

- ✅ Parse TypeScript and JavaScript files (.ts, .tsx, .js, .jsx)
- ✅ Extract functions (including arrow functions, async functions)
- ✅ Extract classes and interfaces
- ✅ Track imports and exports
- ✅ Full integration with `codegraph-parser-api`

## Quick Start

```rust
use codegraph::CodeGraph;
use codegraph_parser_api::CodeParser;
use codegraph_typescript::TypeScriptParser;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = CodeGraph::in_memory()?;
    let parser = TypeScriptParser::new();

    let file_info = parser.parse_file(
        Path::new("src/index.ts"),
        &mut graph
    )?;

    println!("Parsed {} functions", file_info.functions.len());
    println!("Parsed {} classes", file_info.classes.len());

    Ok(())
}
```

## Supported Features

- Functions (regular, arrow, async, generator)
- Classes (including methods, properties, constructors)
- Interfaces
- Import/export statements
- TypeScript type annotations
- JSX/TSX syntax

## License

Apache-2.0
