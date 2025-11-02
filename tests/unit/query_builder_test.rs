//! Unit tests for QueryBuilder
//!
//! These tests verify the fluent query interface for finding code patterns.

use codegraph::{CodeGraph, NodeType, helpers};

#[test]
fn test_query_builder_node_type_filter() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    // Add various node types
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    let func1_id = helpers::add_function(&mut graph, file_id, "func1", 1, 10).unwrap();
    let func2_id = helpers::add_function(&mut graph, file_id, "func2", 12, 20).unwrap();
    let class_id = helpers::add_class(&mut graph, file_id, "MyClass", 22, 40).unwrap();
    
    // Query for only functions
    let results = graph.query()
        .node_type(NodeType::Function)
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 2);
    assert!(results.contains(&func1_id));
    assert!(results.contains(&func2_id));
    assert!(!results.contains(&file_id));
    assert!(!results.contains(&class_id));
}

#[test]
fn test_query_builder_in_file_filter() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    // Add two files with functions
    let file1_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    let func1_id = helpers::add_function(&mut graph, file1_id, "main", 1, 10).unwrap();
    
    let file2_id = helpers::add_file(&mut graph, "src/lib.rs", "rust").unwrap();
    let func2_id = helpers::add_function(&mut graph, file2_id, "helper", 1, 10).unwrap();
    
    // Query for functions in main.rs
    let results = graph.query()
        .node_type(NodeType::Function)
        .in_file("src/main.rs")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], func1_id);
    
    // Query for functions in lib.rs
    let results = graph.query()
        .node_type(NodeType::Function)
        .in_file("src/lib.rs")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], func2_id);
}

#[test]
fn test_query_builder_file_pattern_filter() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    // Add files with different patterns
    helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    helpers::add_file(&mut graph, "src/lib.rs", "rust").unwrap();
    helpers::add_file(&mut graph, "tests/test_main.rs", "rust").unwrap();
    helpers::add_file(&mut graph, "src/utils.py", "python").unwrap();
    
    // Query for src/*.rs files
    let results = graph.query()
        .node_type(NodeType::CodeFile)
        .file_pattern("src/*.rs")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 2);
    
    // Query for all .py files
    let results = graph.query()
        .node_type(NodeType::CodeFile)
        .file_pattern("**/*.py")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
}

#[test]
fn test_query_builder_property_filter() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    
    // Add functions with different properties
    helpers::add_function_with_metadata(
        &mut graph, file_id, "pub_func", 1, 10, "public", "fn pub_func()", false, false
    ).unwrap();
    
    helpers::add_function_with_metadata(
        &mut graph, file_id, "priv_func", 12, 20, "private", "fn priv_func()", false, false
    ).unwrap();
    
    helpers::add_function_with_metadata(
        &mut graph, file_id, "async_func", 22, 30, "public", "async fn async_func()", true, false
    ).unwrap();
    
    // Query for public functions
    let results = graph.query()
        .node_type(NodeType::Function)
        .property("visibility", "public")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 2);
    
    // Query for async functions
    let results = graph.query()
        .node_type(NodeType::Function)
        .property("is_async", true)
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
}

#[test]
fn test_query_builder_name_contains_filter() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    helpers::add_function(&mut graph, file_id, "get_user", 1, 10).unwrap();
    helpers::add_function(&mut graph, file_id, "set_user", 12, 20).unwrap();
    helpers::add_function(&mut graph, file_id, "delete_user", 22, 30).unwrap();
    helpers::add_function(&mut graph, file_id, "process_data", 32, 40).unwrap();
    
    // Query for functions with "user" in name (case insensitive)
    let results = graph.query()
        .node_type(NodeType::Function)
        .name_contains("user")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 3);
    
    // Query for functions with "data" in name
    let results = graph.query()
        .node_type(NodeType::Function)
        .name_contains("data")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
}

#[test]
fn test_query_builder_custom_predicate() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    helpers::add_function(&mut graph, file_id, "short", 1, 5).unwrap();
    helpers::add_function(&mut graph, file_id, "medium", 10, 25).unwrap();
    helpers::add_function(&mut graph, file_id, "long", 30, 100).unwrap();
    
    // Query for functions longer than 20 lines
    let results = graph.query()
        .node_type(NodeType::Function)
        .custom(|node| {
            if let (Some(start), Some(end)) = (
                node.properties.get_int("line_start"),
                node.properties.get_int("line_end")
            ) {
                (end - start) > 20
            } else {
                false
            }
        })
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
}

#[test]
fn test_query_builder_chaining_multiple_filters() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file1_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    let file2_id = helpers::add_file(&mut graph, "src/lib.rs", "rust").unwrap();
    
    // Add various functions
    helpers::add_function_with_metadata(
        &mut graph, file1_id, "pub_main", 1, 10, "public", "fn pub_main()", false, false
    ).unwrap();
    
    helpers::add_function_with_metadata(
        &mut graph, file1_id, "priv_helper", 12, 20, "private", "fn priv_helper()", false, false
    ).unwrap();
    
    helpers::add_function_with_metadata(
        &mut graph, file2_id, "pub_lib", 1, 10, "public", "fn pub_lib()", false, false
    ).unwrap();
    
    // Query with multiple filters: public functions in main.rs
    let results = graph.query()
        .node_type(NodeType::Function)
        .in_file("src/main.rs")
        .property("visibility", "public")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
    
    // Query with name filter and property filter
    let results = graph.query()
        .node_type(NodeType::Function)
        .name_contains("lib")
        .property("visibility", "public")
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 1);
}

#[test]
fn test_query_builder_limit() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    for i in 0..10 {
        helpers::add_function(&mut graph, file_id, &format!("func{i}"), i * 10, i * 10 + 5).unwrap();
    }
    
    // Query with limit
    let results = graph.query()
        .node_type(NodeType::Function)
        .limit(3)
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 3);
    
    // Query with limit larger than results
    let results = graph.query()
        .node_type(NodeType::Function)
        .limit(100)
        .execute()
        .unwrap();
    
    assert_eq!(results.len(), 10);
}

#[test]
fn test_query_builder_count_and_exists() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    let file_id = helpers::add_file(&mut graph, "src/main.rs", "rust").unwrap();
    helpers::add_function(&mut graph, file_id, "func1", 1, 10).unwrap();
    helpers::add_function(&mut graph, file_id, "func2", 12, 20).unwrap();
    helpers::add_function(&mut graph, file_id, "func3", 22, 30).unwrap();
    
    // Test count
    let count = graph.query()
        .node_type(NodeType::Function)
        .count()
        .unwrap();
    
    assert_eq!(count, 3);
    
    // Test exists (should be true)
    let exists = graph.query()
        .node_type(NodeType::Function)
        .name_contains("func2")
        .exists()
        .unwrap();
    
    assert!(exists);
    
    // Test exists (should be false)
    let exists = graph.query()
        .node_type(NodeType::Function)
        .name_contains("nonexistent")
        .exists()
        .unwrap();
    
    assert!(!exists);
}

#[test]
fn test_query_builder_performance_10k_nodes() {
    let mut graph = CodeGraph::in_memory().unwrap();
    
    // Add 10K functions across multiple files
    let file_id = helpers::add_file(&mut graph, "src/large.rs", "rust").unwrap();
    for i in 0..10000 {
        let name = if i % 3 == 0 {
            format!("public_func_{i}")
        } else {
            format!("func_{i}")
        };
        
        let visibility = if i % 3 == 0 { "public" } else { "private" };
        
        helpers::add_function_with_metadata(
            &mut graph,
            file_id,
            &name,
            i * 10,
            i * 10 + 5,
            visibility,
            &format!("fn {name}()"),
            false,
            false
        ).unwrap();
    }
    
    // Query for public functions with name filter
    let results = graph.query()
        .node_type(NodeType::Function)
        .property("visibility", "public")
        .name_contains("public")
        .execute()
        .unwrap();
    
    // Should find ~3333 functions (every 3rd one)
    assert!(results.len() > 3300 && results.len() < 3400);
    
    // Test count is faster than execute
    let count = graph.query()
        .node_type(NodeType::Function)
        .property("visibility", "public")
        .count()
        .unwrap();
    
    assert!(count > 3300 && count < 3400);
    
    // Test exists short-circuits
    let exists = graph.query()
        .node_type(NodeType::Function)
        .property("visibility", "public")
        .exists()
        .unwrap();
    
    assert!(exists);
}
