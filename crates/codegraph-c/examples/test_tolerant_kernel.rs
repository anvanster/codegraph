//! Test tolerant parsing mode against Linux kernel ICE driver
//!
//! This example demonstrates the new hybrid parsing approach:
//! 1. Macro preprocessing to handle kernel-specific macros
//! 2. Tolerant parsing to extract entities even with syntax errors
//! 3. Call graph extraction for function relationships

#![allow(clippy::uninlined_format_args, dead_code)]

use codegraph_c::extractor::{extract_with_options, ExtractionOptions};
use codegraph_c::preprocessor::CPreprocessor;
use codegraph_parser_api::ParserConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Recursively collect all .c files from a directory
fn collect_c_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_c_files(&path));
            } else if path.extension().map(|e| e == "c").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

fn main() {
    let target_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/anvanster/projects/ethernet-linux-ice/src".to_string());

    println!("===========================================");
    println!("Tolerant C Parser - Kernel Code Analysis");
    println!("===========================================");
    println!("Target: {}", target_dir);
    println!();

    let target_path = Path::new(&target_dir);
    if !target_path.exists() {
        eprintln!("ERROR: Target directory does not exist: {}", target_dir);
        std::process::exit(1);
    }

    // Collect all C files
    let c_files: Vec<PathBuf> = collect_c_files(target_path);

    println!("Found {} C files", c_files.len());
    println!();

    // Parse with different modes
    let config = ParserConfig::default();

    // Test 1: Strict mode (original behavior)
    println!("===========================================");
    println!("MODE 1: STRICT (original behavior)");
    println!("===========================================");
    let strict_results = parse_files(&c_files, &config, &ExtractionOptions::default());
    print_results(&strict_results);

    // Test 2: Tolerant mode (no preprocessing)
    println!();
    println!("===========================================");
    println!("MODE 2: TOLERANT (extract partial results)");
    println!("===========================================");
    let tolerant_results = parse_files(&c_files, &config, &ExtractionOptions::tolerant());
    print_results(&tolerant_results);

    // Test 3: Kernel mode (preprocessing + tolerant)
    println!();
    println!("===========================================");
    println!("MODE 3: KERNEL (preprocessing + tolerant)");
    println!("===========================================");
    let kernel_results = parse_files(&c_files, &config, &ExtractionOptions::for_kernel_code());
    print_results(&kernel_results);

    // Show macro analysis
    println!();
    println!("===========================================");
    println!("MACRO ANALYSIS (from first 5 files)");
    println!("===========================================");
    analyze_macros(&c_files);

    // Show call graph sample
    println!();
    println!("===========================================");
    println!("CALL GRAPH SAMPLE");
    println!("===========================================");
    show_call_graph_sample(&c_files, &config);
}

struct ParseResults {
    total_files: usize,
    successful_files: usize,
    partial_files: usize,
    failed_files: usize,
    total_functions: usize,
    total_structs: usize,
    total_calls: usize,
    total_errors: usize,
    parse_time: std::time::Duration,
}

fn parse_files(
    files: &[std::path::PathBuf],
    config: &ParserConfig,
    options: &ExtractionOptions,
) -> ParseResults {
    let start = Instant::now();
    let mut results = ParseResults {
        total_files: files.len(),
        successful_files: 0,
        partial_files: 0,
        failed_files: 0,
        total_functions: 0,
        total_structs: 0,
        total_calls: 0,
        total_errors: 0,
        parse_time: std::time::Duration::ZERO,
    };

    for path in files {
        match fs::read_to_string(path) {
            Ok(source) => match extract_with_options(&source, path, config, options) {
                Ok(extraction) => {
                    results.total_functions += extraction.ir.functions.len();
                    results.total_structs += extraction.ir.classes.len();
                    results.total_errors += extraction.error_count;

                    if extraction.is_partial {
                        results.partial_files += 1;
                    } else {
                        results.successful_files += 1;
                    }
                }
                Err(_) => {
                    results.failed_files += 1;
                }
            },
            Err(_) => {
                results.failed_files += 1;
            }
        }
    }

    results.parse_time = start.elapsed();
    results
}

fn print_results(results: &ParseResults) {
    let success_rate = if results.total_files > 0 {
        (results.successful_files + results.partial_files) as f64 / results.total_files as f64
            * 100.0
    } else {
        0.0
    };

    println!("Files analyzed: {}", results.total_files);
    println!(
        "  - Clean parses: {} ({:.1}%)",
        results.successful_files,
        results.successful_files as f64 / results.total_files as f64 * 100.0
    );
    println!(
        "  - Partial parses: {} ({:.1}%)",
        results.partial_files,
        results.partial_files as f64 / results.total_files as f64 * 100.0
    );
    println!(
        "  - Failed: {} ({:.1}%)",
        results.failed_files,
        results.failed_files as f64 / results.total_files as f64 * 100.0
    );
    println!("Overall success rate: {:.1}%", success_rate);
    println!();
    println!("Entities extracted:");
    println!("  - Functions: {}", results.total_functions);
    println!("  - Structs/Types: {}", results.total_structs);
    println!("  - Syntax errors: {}", results.total_errors);
    println!();
    println!("Parse time: {:.2?}", results.parse_time);
}

fn analyze_macros(files: &[std::path::PathBuf]) {
    let preprocessor = CPreprocessor::new();
    let mut all_macros: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for path in files.iter().take(5) {
        if let Ok(source) = fs::read_to_string(path) {
            for macro_info in preprocessor.analyze_macros(&source) {
                *all_macros.entry(macro_info.name).or_insert(0) += 1;
            }
        }
    }

    // Sort by frequency
    let mut sorted: Vec<_> = all_macros.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    println!("Top 20 detected macros:");
    for (name, count) in sorted.iter().take(20) {
        let kind = preprocessor.classify_macro(name);
        println!("  {:30} {:>3}x  ({:?})", name, count, kind);
    }
}

fn show_call_graph_sample(files: &[std::path::PathBuf], config: &ParserConfig) {
    use codegraph_c::visitor::CVisitor;
    use tree_sitter::Parser;

    // Parse first file with call extraction
    let sample_file = files.iter().find(|p| {
        p.file_name()
            .map(|n| n.to_str().unwrap_or("").contains("main"))
            .unwrap_or(false)
    });

    let sample_file = sample_file.or_else(|| files.first());

    if let Some(path) = sample_file {
        println!("Analyzing: {}", path.display());

        if let Ok(source) = fs::read_to_string(path) {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_c::language()).unwrap();

            if let Some(tree) = parser.parse(&source, None) {
                let mut visitor = CVisitor::new(source.as_bytes(), config.clone());
                visitor.set_extract_calls(true);
                visitor.visit_node(tree.root_node());

                println!();
                println!("Functions defined: {}", visitor.functions.len());
                for func in visitor.functions.iter().take(10) {
                    println!(
                        "  - {} (lines {}-{})",
                        func.name, func.line_start, func.line_end
                    );
                }

                println!();
                println!("Function calls found: {}", visitor.calls.len());
                for call in visitor.calls.iter().take(20) {
                    let caller = call.caller.as_deref().unwrap_or("<global>");
                    println!("  {} -> {} (line {})", caller, call.callee, call.line);
                }

                // Build simple call graph
                if !visitor.calls.is_empty() {
                    println!();
                    println!("Call graph edges:");
                    let mut edges: std::collections::HashMap<String, Vec<String>> =
                        std::collections::HashMap::new();

                    for call in &visitor.calls {
                        if let Some(caller) = &call.caller {
                            edges
                                .entry(caller.clone())
                                .or_default()
                                .push(call.callee.clone());
                        }
                    }

                    for (caller, callees) in edges.iter().take(10) {
                        let unique_callees: std::collections::HashSet<_> = callees.iter().collect();
                        let callees_str: Vec<&str> =
                            unique_callees.iter().take(5).map(|s| s.as_str()).collect();
                        println!("  {} calls: {}", caller, callees_str.join(", "));
                    }
                }
            }
        }
    }
}
