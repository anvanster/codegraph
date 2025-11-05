# Parser Improvements Implementation Plan

## Overview

This document outlines the implementation plan for 5 pending parser improvements identified during e2e test development.

## Priority Assessment

### High Priority (Critical for Core Functionality)
1. **TypeScript Method Extraction** - Core functionality gap
2. **Go Individual Import Extraction** - Data accuracy issue
3. **Rust Import Nodes in Mapper** - Feature parity with other parsers

### Medium Priority (Important for Completeness)
4. **TypeScript Full Import Parsing** - Better dependency tracking
5. **Parser Metrics Consistency** - API consistency

### Low Priority (Nice to Have)
6. **TypeScript JSX/TSX Support** - Edge case, requires significant configuration

---

## 1. TypeScript Method Extraction

### Current State
- Classes are detected and extracted
- Methods inside classes are not extracted
- Visitor only matches top-level function_declaration nodes

### Implementation Plan

#### Phase 1: Analysis (1-2 hours)
- [ ] Study tree-sitter-typescript AST for class methods
- [ ] Identify node types: `method_definition`, `public_field_definition`
- [ ] Review how methods differ from functions (constructor, getters, setters)
- [ ] Check test cases in visitor.rs unit tests

#### Phase 2: Visitor Enhancement (2-3 hours)
**File**: `crates/codegraph-typescript/src/visitor.rs`

```rust
fn visit_class(&mut self, node: Node) {
    // ... existing code ...

    // Visit class body for methods
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "method_definition" => self.visit_method(child, &name),
                "field_definition" | "public_field_definition" => {
                    // Extract fields if needed
                }
                _ => {}
            }
        }
    }
}

fn visit_method(&mut self, node: Node, class_name: &str) {
    let method_name = node.child_by_field_name("name")
        .map(|n| self.node_text(n))
        .unwrap_or_else(|| "anonymous".to_string());

    // Extract method details
    let is_static = /* check for static keyword */;
    let is_async = /* check for async keyword */;
    let visibility = /* extract visibility */;

    let func = FunctionEntity {
        name: method_name.clone(),
        parent_class: Some(class_name.to_string()),
        // ... other fields ...
    };

    self.functions.push(func);
}
```

#### Phase 3: Mapper Integration (1 hour)
**File**: `crates/codegraph-typescript/src/mapper.rs`

- Methods with `parent_class` should link to class node
- Add Contains edge from class to method

#### Phase 4: Testing (2 hours)
- [ ] Update existing tests to verify method extraction
- [ ] Add tests for: constructors, static methods, getters/setters, private methods
- [ ] Test method signatures with TypeScript types

**Estimated Total**: 6-8 hours

---

## 2. TypeScript Full Import Parsing

### Current State
- Import statements detected as single text block
- No parsing of import specifiers or symbols
- Limited metadata extraction

### Implementation Plan

#### Phase 1: Analysis (1 hour)
- [ ] Map import statement types:
  - Default imports: `import React from 'react'`
  - Named imports: `import { useState, useEffect } from 'react'`
  - Namespace imports: `import * as Utils from './utils'`
  - Type imports: `import type { User } from './types'`
  - Side effect: `import './styles.css'`

#### Phase 2: Import Parser (3-4 hours)
**File**: `crates/codegraph-typescript/src/visitor.rs`

```rust
fn visit_import(&mut self, node: Node) {
    let source = node.child_by_field_name("source")
        .map(|n| self.node_text(n))
        .unwrap_or_default();

    // Remove quotes from source
    let imported = source.trim_matches(|c| c == '"' || c == '\'').to_string();

    // Extract import specifiers
    let mut symbols = Vec::new();
    let mut alias = None;
    let mut is_wildcard = false;

    if let Some(clause) = node.child_by_field_name("import_clause") {
        // Default import
        if let Some(default) = clause.child_by_field_name("name") {
            symbols.push(self.node_text(default));
        }

        // Named imports
        if let Some(named) = clause.child_by_field_name("named_imports") {
            symbols.extend(self.extract_named_imports(named));
        }

        // Namespace import
        if let Some(namespace) = clause.child_by_field_name("namespace_import") {
            is_wildcard = true;
            if let Some(alias_node) = namespace.child_by_field_name("name") {
                alias = Some(self.node_text(alias_node));
            }
        }
    }

    let import = ImportRelation {
        importer: "current_module".to_string(),
        imported,
        symbols,
        is_wildcard,
        alias,
    };

    self.imports.push(import);
}

fn extract_named_imports(&self, node: Node) -> Vec<String> {
    let mut imports = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "import_specifier" {
            if let Some(name) = child.child_by_field_name("name") {
                imports.push(self.node_text(name));
            }
        }
    }

    imports
}
```

#### Phase 3: Testing (2 hours)
- [ ] Test all import types
- [ ] Test import aliases
- [ ] Test type-only imports
- [ ] Test multiple imports from same source

**Estimated Total**: 6-7 hours

---

## 3. Go Individual Import Extraction

### Current State
- Import blocks extracted as single text blob
- Individual imports not parsed
- Missing import aliases and path extraction

### Implementation Plan

#### Phase 1: Analysis (1 hour)
- [ ] Study tree-sitter-go AST structure
- [ ] Node types: `import_declaration`, `import_spec`, `import_spec_list`
- [ ] Handle single imports vs import blocks

#### Phase 2: Visitor Enhancement (2-3 hours)
**File**: `crates/codegraph-go/src/visitor.rs`

```rust
fn visit_import(&mut self, node: Node) {
    // Check if single import or import block
    if let Some(spec_list) = node.child_by_field_name("specs") {
        // Import block: import ( ... )
        let mut cursor = spec_list.walk();
        for child in spec_list.children(&mut cursor) {
            if child.kind() == "import_spec" {
                self.extract_import_spec(child);
            }
        }
    } else if let Some(spec) = node.child_by_field_name("spec") {
        // Single import: import "fmt"
        self.extract_import_spec(spec);
    }
}

fn extract_import_spec(&mut self, node: Node) {
    let mut alias = None;
    let mut is_wildcard = false;

    // Check for alias
    if let Some(name) = node.child_by_field_name("name") {
        let alias_text = self.node_text(name);
        if alias_text == "." {
            is_wildcard = true;
        } else if alias_text != "_" {
            alias = Some(alias_text);
        }
    }

    // Extract import path
    let path = node.child_by_field_name("path")
        .map(|n| self.node_text(n))
        .unwrap_or_default();

    // Remove quotes
    let imported = path.trim_matches('"').to_string();

    let import = ImportRelation {
        importer: "current_package".to_string(),
        imported,
        symbols: Vec::new(), // Go doesn't have named imports
        is_wildcard,
        alias,
    };

    self.imports.push(import);
}
```

#### Phase 3: Testing (1-2 hours)
- [ ] Update test expectations from 1 import to actual count
- [ ] Test import blocks with multiple imports
- [ ] Test import aliases (including . and _)
- [ ] Test single vs block imports

**Estimated Total**: 4-6 hours

---

## 4. Rust Import Nodes in Mapper

### Current State
- Visitor extracts imports into IR
- Mapper ignores imports (see mapper.rs:175-176)
- No Module nodes created for imported crates

### Implementation Plan

#### Phase 1: Analysis (1 hour)
- [ ] Review Go mapper implementation (reference for import handling)
- [ ] Decide on node structure: Module nodes for crates/modules
- [ ] Plan edge type: Imports edge from file to module

#### Phase 2: Mapper Enhancement (2-3 hours)
**File**: `crates/codegraph-rust/src/mapper.rs`

```rust
// Around line 172-176, replace TODO with implementation:

// Add import relationships
for import in &ir.imports {
    let imported_module = &import.imported;

    // Create or get module node
    let import_id = if let Some(&existing_id) = node_map.get(imported_module) {
        existing_id
    } else {
        let is_external = !imported_module.starts_with("super::")
            && !imported_module.starts_with("crate::")
            && !imported_module.starts_with("self::");

        let props = PropertyMap::new()
            .with("name", imported_module.clone())
            .with("is_external", is_external.to_string());

        let id = graph.add_node(NodeType::Module, props)
            .map_err(|e| ParserError::GraphError(e.to_string()))?;
        node_map.insert(imported_module.clone(), id);
        id
    };

    import_ids.push(import_id);

    // Create import edge from file to module
    let mut edge_props = PropertyMap::new();
    if let Some(ref alias) = import.alias {
        edge_props = edge_props.with("alias", alias.clone());
    }
    if import.is_wildcard {
        edge_props = edge_props.with("is_wildcard", "true");
    }
    if !import.symbols.is_empty() {
        edge_props = edge_props.with("symbols", import.symbols.join(","));
    }

    graph.add_edge(file_id, import_id, EdgeType::Imports, edge_props)
        .map_err(|e| ParserError::GraphError(e.to_string()))?;
}
```

#### Phase 3: Testing (1-2 hours)
- [ ] Update test expectations for import count
- [ ] Verify Module nodes created
- [ ] Verify Import edges created
- [ ] Test external vs internal module detection

**Estimated Total**: 4-6 hours

---

## 5. Parser Metrics Consistency

### Current State
- Only `parse_file` updates metrics
- `parse_source` doesn't update metrics (by design or oversight?)
- Inconsistent API behavior

### Implementation Plan

#### Phase 1: Design Decision (30 min)
**Option A**: Update metrics in both methods
- Pros: Consistent behavior, easier to understand
- Cons: May double-count if parse_source is called by parse_file

**Option B**: Only update in parse_file
- Pros: Clear separation, no double-counting
- Cons: Inconsistent, surprising behavior

**Option C**: Add metrics parameter to parse_source
- Pros: Explicit control, flexible
- Cons: API change, more complex

**Recommendation**: Option B with clear documentation

#### Phase 2: Implementation (1 hour per parser)

**If choosing Option A** (update both methods):

For each parser (`go`, `typescript`, `rust`):
```rust
fn parse_source(&self, source: &str, file_path: &Path, graph: &mut CodeGraph)
    -> Result<FileInfo, ParserError>
{
    let start = Instant::now();
    let ir = extractor::extract(source, file_path, &self.config)?;
    let mut file_info = self.ir_to_graph(&ir, graph, file_path)?;

    file_info.parse_time = start.elapsed();
    file_info.line_count = source.lines().count();
    file_info.byte_count = source.len();

    // Add metrics update
    self.update_metrics(true, file_info.parse_time, file_info.entity_count(), 0);

    Ok(file_info)
}
```

#### Phase 3: Documentation (30 min)
- [ ] Document metrics behavior in CodeParser trait
- [ ] Update test expectations
- [ ] Add docs to each parser implementation

**Estimated Total**: 4-5 hours (all parsers)

---

## 6. TypeScript JSX/TSX Support

### Current State
- Parser uses tree-sitter-typescript
- JSX syntax not fully supported
- Requires TSX language variant configuration

### Implementation Plan

#### Phase 1: Research (2-3 hours)
- [ ] Study tree-sitter-typescript TSX language
- [ ] Investigate file extension detection (.tsx vs .ts)
- [ ] Research language switching or dual parser approach
- [ ] Check if current failures are syntax or visitor issues

#### Phase 2: Parser Configuration (3-4 hours)
**File**: `crates/codegraph-typescript/src/extractor.rs`

```rust
pub fn extract(source: &str, file_path: &Path, config: &ParserConfig)
    -> Result<CodeIR, ParserError>
{
    let mut parser = Parser::new();

    // Detect language based on file extension
    let language = if file_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "tsx" || e == "jsx")
        .unwrap_or(false)
    {
        tree_sitter_typescript::language_tsx()
    } else {
        tree_sitter_typescript::language_typescript()
    };

    parser.set_language(language)
        .map_err(|e| ParserError::ParseError(file_path.to_path_buf(), e.to_string()))?;

    // ... rest of implementation
}
```

#### Phase 3: Visitor Adjustments (2-3 hours)
- [ ] Handle JSX element nodes if needed
- [ ] Extract JSX components as functions
- [ ] Test with React component patterns

#### Phase 4: Testing (2 hours)
- [ ] Create .tsx test files
- [ ] Test React components, hooks
- [ ] Test JSX expressions and fragments

**Estimated Total**: 9-12 hours

**Note**: This is lower priority due to complexity and edge case nature.

---

## Implementation Order & Timeline

### Phase 1: High Priority Items (2-3 weeks)
**Week 1**:
- [ ] Go Individual Import Extraction (4-6 hours)
- [ ] Rust Import Nodes in Mapper (4-6 hours)
- [ ] TypeScript Method Extraction (6-8 hours)

**Week 2-3**:
- [ ] TypeScript Full Import Parsing (6-7 hours)
- [ ] Parser Metrics Consistency (4-5 hours)

### Phase 2: Low Priority Items (1 week, optional)
**Week 4**:
- [ ] TypeScript JSX/TSX Support (9-12 hours)

### Total Estimated Effort
- **High Priority**: 24-32 hours (3-4 days)
- **Medium Priority**: 10-12 hours (1-2 days)
- **Low Priority**: 9-12 hours (1-2 days)
- **Total**: 43-56 hours (5-7 days)

---

## Testing Strategy

### Unit Tests
- Each visitor change needs unit tests in visitor.rs
- Each mapper change needs unit tests in mapper.rs

### Integration Tests
- Update existing e2e tests to verify new functionality
- Add new test cases for edge cases
- Ensure backwards compatibility

### Validation Tests
- Run full test suite after each implementation
- Verify no regressions in existing functionality
- Test with real-world code samples

---

## Success Criteria

### For Each Item
- [ ] Implementation complete and tested
- [ ] All existing tests still pass
- [ ] New tests added for new functionality
- [ ] Documentation updated
- [ ] Code reviewed and approved

### Overall
- [ ] All e2e tests passing with updated expectations
- [ ] No known limitations remaining (or documented)
- [ ] Parser feature parity achieved across languages
- [ ] Performance benchmarks maintained or improved

---

## Risks & Mitigation

### Risk 1: Breaking Changes
- **Mitigation**: Comprehensive test coverage before changes
- **Mitigation**: Feature flags for experimental features

### Risk 2: Performance Degradation
- **Mitigation**: Benchmark before and after
- **Mitigation**: Optimize hot paths

### Risk 3: Tree-sitter API Changes
- **Mitigation**: Pin to specific versions
- **Mitigation**: Document version dependencies

### Risk 4: Time Estimation Accuracy
- **Mitigation**: Start with smaller, well-defined tasks
- **Mitigation**: Regular progress reviews

---

## Dependencies

### External
- tree-sitter 0.20
- tree-sitter-typescript 0.20
- tree-sitter-go 0.20
- syn (for Rust parsing)

### Internal
- codegraph-parser-api (stable)
- codegraph (stable)

---

## Next Steps

1. Review and approve this plan
2. Prioritize items based on project needs
3. Assign development resources
4. Create tracking issues for each item
5. Begin implementation in priority order
6. Regular progress updates and reviews
