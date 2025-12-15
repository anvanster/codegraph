//! Test tolerant parsing mode against Linux kernel ICE driver
//!
//! This example demonstrates the parsing approaches:
//! 1. Strict mode - original behavior, fails on syntax errors
//! 2. Tolerant mode - extract entities even with syntax errors
//! 3. Kernel mode - preprocessing + tolerant
//! 4. Pipeline mode - new layered pipeline with platform detection

#![allow(clippy::uninlined_format_args, dead_code)]

use codegraph_c::extractor::{extract_with_options, ExtractionOptions};
use codegraph_c::pipeline::{Pipeline, PipelineConfig};
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

    // Test 4: Pipeline mode (new layered pipeline)
    println!();
    println!("===========================================");
    println!("MODE 4: PIPELINE (layered processing)");
    println!("===========================================");
    let pipeline_results = parse_files_with_pipeline(&c_files, &config);
    print_pipeline_results(&pipeline_results);

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

struct PipelineParseResults {
    total_files: usize,
    successful_files: usize,
    partial_files: usize,
    failed_files: usize,
    total_functions: usize,
    total_structs: usize,
    total_errors: usize,
    parse_time: std::time::Duration,
    // Pipeline-specific stats
    stubs_injected: usize,
    conditionals_stripped: usize,
    gcc_neutralized: usize,
    attributes_stripped: usize,
    platform_confidence: f32,
    // Macro neutralization stats
    likely_unlikely_stripped: usize,
    build_bug_on_stripped: usize,
    warn_on_stripped: usize,
    define_macros_stubbed: usize,
    rcu_simplified: usize,
    typeof_replaced: usize,
    // Iterator macro stats
    list_for_each_expanded: usize,
    for_each_expanded: usize,
    container_of_expanded: usize,
}

fn parse_files_with_pipeline(
    files: &[std::path::PathBuf],
    config: &ParserConfig,
) -> PipelineParseResults {
    let start = Instant::now();
    let pipeline = Pipeline::new();
    let pipeline_config = PipelineConfig::for_kernel_code();
    let options = ExtractionOptions::tolerant();

    let mut results = PipelineParseResults {
        total_files: files.len(),
        successful_files: 0,
        partial_files: 0,
        failed_files: 0,
        total_functions: 0,
        total_structs: 0,
        total_errors: 0,
        parse_time: std::time::Duration::ZERO,
        stubs_injected: 0,
        conditionals_stripped: 0,
        gcc_neutralized: 0,
        attributes_stripped: 0,
        platform_confidence: 0.0,
        likely_unlikely_stripped: 0,
        build_bug_on_stripped: 0,
        warn_on_stripped: 0,
        define_macros_stubbed: 0,
        rcu_simplified: 0,
        typeof_replaced: 0,
        list_for_each_expanded: 0,
        for_each_expanded: 0,
        container_of_expanded: 0,
    };

    let mut confidence_sum = 0.0;
    let mut confidence_count = 0;

    for path in files {
        match fs::read_to_string(path) {
            Ok(source) => {
                // First pass through pipeline
                let pipeline_result = pipeline.process(&source, &pipeline_config);

                // Accumulate pipeline stats
                results.stubs_injected += pipeline_result.stats.stubs_injected;
                results.conditionals_stripped += pipeline_result.stats.conditionals_stripped;
                results.gcc_neutralized += pipeline_result.stats.gcc_neutralized;
                results.attributes_stripped += pipeline_result.stats.attributes_stripped;
                confidence_sum += pipeline_result.platform.confidence;
                confidence_count += 1;

                // Accumulate macro stats
                let ms = &pipeline_result.stats.macro_stats;
                results.likely_unlikely_stripped += ms.likely_unlikely_stripped;
                results.build_bug_on_stripped += ms.build_bug_on_stripped;
                results.warn_on_stripped += ms.warn_on_stripped;
                results.define_macros_stubbed += ms.define_macros_stubbed;
                results.rcu_simplified += ms.rcu_simplified;
                results.typeof_replaced += ms.typeof_replaced;
                results.list_for_each_expanded += ms.list_for_each_expanded;
                results.for_each_expanded += ms.for_each_expanded;
                results.container_of_expanded += ms.container_of_expanded;

                // Now extract from processed source
                match extract_with_options(&pipeline_result.source, path, config, &options) {
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
                }
            }
            Err(_) => {
                results.failed_files += 1;
            }
        }
    }

    results.parse_time = start.elapsed();
    results.platform_confidence = if confidence_count > 0 {
        confidence_sum / confidence_count as f32
    } else {
        0.0
    };
    results
}

fn print_pipeline_results(results: &PipelineParseResults) {
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
    println!("Pipeline statistics:");
    println!(
        "  - Platform confidence: {:.1}%",
        results.platform_confidence * 100.0
    );
    println!("  - Header stubs injected: {}", results.stubs_injected);
    println!(
        "  - Conditionals stripped: {}",
        results.conditionals_stripped
    );
    println!(
        "  - GCC extensions neutralized: {}",
        results.gcc_neutralized
    );
    println!("  - Attributes stripped: {}", results.attributes_stripped);
    println!();
    println!("Macro neutralization:");
    println!(
        "  - likely/unlikely stripped: {}",
        results.likely_unlikely_stripped
    );
    println!(
        "  - BUILD_BUG_ON stripped: {}",
        results.build_bug_on_stripped
    );
    println!("  - WARN_ON/BUG_ON stripped: {}", results.warn_on_stripped);
    println!(
        "  - DEFINE_* macros stubbed: {}",
        results.define_macros_stubbed
    );
    println!("  - RCU macros simplified: {}", results.rcu_simplified);
    println!("  - typeof() replaced: {}", results.typeof_replaced);
    println!();
    println!("Iterator macro handling:");
    println!(
        "  - list_for_each_* expanded: {}",
        results.list_for_each_expanded
    );
    println!("  - for_each_* expanded: {}", results.for_each_expanded);
    println!(
        "  - container_of expanded: {}",
        results.container_of_expanded
    );
    println!();
    println!("Parse time: {:.2?}", results.parse_time);
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
