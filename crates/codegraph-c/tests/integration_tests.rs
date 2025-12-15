//! Integration tests for the C parser

use codegraph::CodeGraph;
use codegraph_c::{CParser, CodeParser};
use std::path::Path;

#[test]
fn test_parse_hello_world() {
    let source = r#"
#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    return 0;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("hello.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 1);
    assert_eq!(file_info.imports.len(), 1);
}

#[test]
fn test_parse_multiple_functions() {
    let source = r#"
int add(int a, int b) {
    return a + b;
}

int subtract(int a, int b) {
    return a - b;
}

int multiply(int a, int b) {
    return a * b;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("math.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 3);
}

#[test]
fn test_parse_struct_with_typedef() {
    let source = r#"
struct Node {
    int data;
    struct Node *next;
};

typedef struct Node Node;

Node* create_node(int data) {
    return NULL;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("node.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.classes.len(), 1); // struct Node
    assert_eq!(file_info.functions.len(), 1); // create_node
}

#[test]
fn test_parse_enum() {
    let source = r#"
enum Status {
    OK = 0,
    ERROR = 1,
    PENDING = 2
};

int get_status() {
    return OK;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("status.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.classes.len(), 1); // enum Status
    assert_eq!(file_info.functions.len(), 1); // get_status
}

#[test]
fn test_parse_union() {
    let source = r#"
union Data {
    int i;
    float f;
    char str[20];
};

void process_data(union Data *d) {
    d->i = 10;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("data.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.classes.len(), 1); // union Data
    assert_eq!(file_info.functions.len(), 1); // process_data
}

#[test]
fn test_parse_static_functions() {
    let source = r#"
static int helper(int x) {
    return x * 2;
}

int public_func(int x) {
    return helper(x) + 1;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("module.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 2);
}

#[test]
fn test_parse_multiple_includes() {
    let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "myheader.h"

void test() {}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("test.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.imports.len(), 4);
}

#[test]
fn test_parse_pointer_parameters() {
    let source = r#"
void swap(int *a, int *b) {
    int temp = *a;
    *a = *b;
    *b = temp;
}

void process_array(int arr[], int size) {
    // Process array
}

void process_strings(char **strings, int count) {
    // Process strings
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("pointers.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 3);
}

#[test]
fn test_parse_variadic_function() {
    let source = r#"
#include <stdarg.h>

int sum(int count, ...) {
    va_list args;
    va_start(args, count);

    int total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(args, int);
    }

    va_end(args);
    return total;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("varargs.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 1);
}

#[test]
fn test_parse_complex_function() {
    let source = r#"
int complex_function(int x, int y) {
    int result = 0;

    if (x > 0) {
        for (int i = 0; i < x; i++) {
            if (i % 2 == 0) {
                result += i;
            } else {
                result -= i;
            }
        }
    } else if (x < 0) {
        while (y > 0) {
            result += y;
            y--;
        }
    } else {
        switch (y) {
            case 1:
                result = 100;
                break;
            case 2:
                result = 200;
                break;
            default:
                result = 0;
                break;
        }
    }

    return result;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("complex.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 1);
}

#[test]
fn test_parse_header_file() {
    let parser = CParser::new();

    assert!(parser.can_parse(Path::new("header.h")));
    assert!(parser.can_parse(Path::new("source.c")));
    assert!(!parser.can_parse(Path::new("source.cpp")));
    assert!(!parser.can_parse(Path::new("source.rs")));
}

#[test]
fn test_parse_nested_structs() {
    let source = r#"
struct Address {
    char street[100];
    char city[50];
    int zip;
};

struct Person {
    char name[100];
    int age;
    struct Address address;
};
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("person.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.classes.len(), 2);
}

#[test]
fn test_parse_syntax_error() {
    let source = r#"
int broken( {
    // Missing closing brace
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("broken.c"), &mut graph);

    assert!(result.is_err());
}

#[test]
fn test_parse_empty_file() {
    let source = "";

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("empty.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 0);
    assert_eq!(file_info.classes.len(), 0);
}

#[test]
fn test_parse_comments_only() {
    let source = r#"
// This is a comment
/* This is a
   multi-line comment */
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("comments.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 0);
}

#[test]
fn test_parser_metrics() {
    let source = r#"
int test() { return 0; }
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let mut parser = CParser::new();

    // Reset metrics
    parser.reset_metrics();
    let metrics = parser.metrics();
    assert_eq!(metrics.files_attempted, 0);

    // Parse a file
    let _ = parser.parse_source(source, Path::new("test.c"), &mut graph);

    // Note: parse_source doesn't update metrics, only parse_file does
    // This is by design to avoid double counting
}

#[test]
fn test_parser_language() {
    let parser = CParser::new();
    assert_eq!(parser.language(), "c");
}

#[test]
fn test_parser_file_extensions() {
    let parser = CParser::new();
    let extensions = parser.file_extensions();
    assert!(extensions.contains(&".c"));
    assert!(extensions.contains(&".h"));
}

#[test]
fn test_parse_function_pointer_param() {
    let source = r#"
void register_callback(void (*callback)(int)) {
    // Store callback
}

void process(int (*compare)(const void*, const void*)) {
    // Use comparator
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("callbacks.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 2);
}

#[test]
fn test_parse_const_params() {
    let source = r#"
void print_string(const char *str) {
    // Print string
}

int compare(const int *a, const int *b) {
    return *a - *b;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("const.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 2);
}

#[test]
fn test_parse_goto_complexity() {
    let source = r#"
void cleanup(int *resources, int count) {
    for (int i = 0; i < count; i++) {
        if (resources[i] < 0) {
            goto error;
        }
    }
    return;

error:
    // Cleanup code
    return;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("goto.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 1);
}

#[test]
fn test_parse_ternary_operator() {
    let source = r#"
int max(int a, int b) {
    return a > b ? a : b;
}

int abs_val(int x) {
    return x >= 0 ? x : -x;
}
"#;

    let mut graph = CodeGraph::in_memory().unwrap();
    let parser = CParser::new();
    let result = parser.parse_source(source, Path::new("ternary.c"), &mut graph);

    assert!(result.is_ok());
    let file_info = result.unwrap();
    assert_eq!(file_info.functions.len(), 2);
}
