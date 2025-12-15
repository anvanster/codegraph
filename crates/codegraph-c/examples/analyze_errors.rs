//! Analyze what's causing syntax errors in kernel code parsing

#![allow(
    clippy::uninlined_format_args,
    clippy::type_complexity,
    clippy::into_iter_on_ref,
    clippy::needless_borrow
)]

use codegraph_c::preprocessor::CPreprocessor;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

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

/// Extract context around an error node
fn get_error_context(source: &str, node: Node, context_lines: usize) -> String {
    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    let lines: Vec<&str> = source.lines().collect();
    let start = start_line.saturating_sub(context_lines);
    let end = (end_line + context_lines + 1).min(lines.len());

    let mut result = String::new();
    for i in start..end {
        let marker = if i >= start_line && i <= end_line {
            ">>>"
        } else {
            "   "
        };
        result.push_str(&format!(
            "{} {:4}: {}\n",
            marker,
            i + 1,
            lines.get(i).unwrap_or(&"")
        ));
    }
    result
}

/// Classify error based on surrounding context
fn classify_error(source: &str, node: Node) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(start_line).unwrap_or(&"");

    // Check for common patterns
    if line.contains("DEFINE_MUTEX") || line.contains("DEFINE_SPINLOCK") {
        return "DEFINE_* macro declaration".to_string();
    }
    if line.contains("LIST_HEAD") || line.contains("HLIST_HEAD") {
        return "LIST_HEAD declaration".to_string();
    }
    if line.contains("MODULE_") {
        return "MODULE_* macro".to_string();
    }
    if line.contains("__attribute__") {
        return "__attribute__ syntax".to_string();
    }
    if line.contains("typeof") || line.contains("__typeof__") {
        return "typeof expression".to_string();
    }
    if line.contains("container_of") {
        return "container_of macro".to_string();
    }
    if line.contains("likely(") || line.contains("unlikely(") {
        return "likely/unlikely macro".to_string();
    }
    if line.contains("BUILD_BUG_ON") || line.contains("WARN_ON") || line.contains("BUG_ON") {
        return "assertion macro".to_string();
    }
    if line.contains("ARRAY_SIZE") {
        return "ARRAY_SIZE macro".to_string();
    }
    if line.contains("offsetof") || line.contains("sizeof") {
        return "size/offset expression".to_string();
    }
    if text.contains("##") || text.contains("#@") {
        return "token pasting/stringification".to_string();
    }
    if line.trim().starts_with("#if")
        || line.trim().starts_with("#else")
        || line.trim().starts_with("#endif")
    {
        return "preprocessor conditional".to_string();
    }

    // Check for unknown type patterns
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() >= 2 {
        let first = words[0];
        if first
            .chars()
            .next()
            .map(|c| c.is_lowercase())
            .unwrap_or(false)
            && ![
                "static", "extern", "const", "volatile", "inline", "void", "int", "char", "long",
                "short", "unsigned", "signed", "struct", "union", "enum", "if", "else", "for",
                "while", "do", "switch", "case", "return", "break", "continue", "goto", "typedef",
            ]
            .contains(&first)
        {
            return format!("unknown type: {}", first);
        }
    }

    // Truncate long error text
    let short_text = if text.len() > 50 {
        format!("{}...", &text[..50])
    } else {
        text.to_string()
    };

    format!("other: {}", short_text.replace('\n', " "))
}

fn analyze_errors(node: Node, source: &str, errors: &mut Vec<(String, String, usize)>) {
    if node.is_error() {
        let classification = classify_error(source, node);
        let context = get_error_context(source, node, 1);
        let line = node.start_position().row + 1;
        errors.push((classification, context, line));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        analyze_errors(child, source, errors);
    }
}

fn analyze_with_preprocessing(
    c_files: &[PathBuf],
    parser: &mut Parser,
    preprocessor: &CPreprocessor,
    use_preprocessing: bool,
) -> (
    HashMap<String, usize>,
    HashMap<String, Vec<(String, String, usize)>>,
    usize,
) {
    let mut error_classifications: HashMap<String, usize> = HashMap::new();
    let mut sample_errors: HashMap<String, Vec<(String, String, usize)>> = HashMap::new();
    let mut total_errors = 0;

    for path in c_files {
        if let Ok(source) = fs::read_to_string(path) {
            let code = if use_preprocessing {
                preprocessor.preprocess(&source)
            } else {
                source.clone()
            };

            if let Some(tree) = parser.parse(&code, None) {
                let mut errors = Vec::new();
                analyze_errors(tree.root_node(), &code, &mut errors);

                for (classification, context, line) in errors {
                    total_errors += 1;
                    *error_classifications
                        .entry(classification.clone())
                        .or_insert(0) += 1;

                    let samples = sample_errors.entry(classification).or_default();
                    if samples.len() < 2 {
                        samples.push((path.display().to_string(), context, line));
                    }
                }
            }
        }
    }

    (error_classifications, sample_errors, total_errors)
}

fn main() {
    let target_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/anvanster/projects/ethernet-linux-ice/src".to_string());

    println!("===========================================");
    println!("Error Pattern Analysis");
    println!("===========================================");
    println!("Target: {}", target_dir);
    println!();

    let target_path = Path::new(&target_dir);
    let c_files = collect_c_files(target_path);

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_c::language()).unwrap();
    let preprocessor = CPreprocessor::new();

    // Analyze WITHOUT preprocessing
    println!("===========================================");
    println!("MODE 1: RAW (no preprocessing)");
    println!("===========================================");
    let (raw_classifications, raw_samples, raw_total) =
        analyze_with_preprocessing(&c_files, &mut parser, &preprocessor, false);
    print_analysis(&raw_classifications, &raw_samples, raw_total);

    // Analyze WITH preprocessing
    println!();
    println!("===========================================");
    println!("MODE 2: PREPROCESSED");
    println!("===========================================");
    let (prep_classifications, _prep_samples, prep_total) =
        analyze_with_preprocessing(&c_files, &mut parser, &preprocessor, true);
    print_analysis_summary(&prep_classifications, prep_total);

    // Print comparison
    println!();
    println!("===========================================");
    println!("COMPARISON");
    println!("===========================================");
    let reduction = raw_total as f64 - prep_total as f64;
    let pct = (reduction / raw_total as f64) * 100.0;
    println!("Raw errors:          {}", raw_total);
    println!("Preprocessed errors: {}", prep_total);
    println!("Reduction:           {:.0} ({:.1}%)", reduction, pct);
}

fn print_analysis(
    error_classifications: &HashMap<String, usize>,
    sample_errors: &HashMap<String, Vec<(String, String, usize)>>,
    total_errors: usize,
) {
    println!("Total errors found: {}", total_errors);
    println!();

    // Sort by frequency
    let mut sorted: Vec<_> = error_classifications.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    println!("===========================================");
    println!("ERROR CLASSIFICATION (by frequency)");
    println!("===========================================");

    for (classification, count) in &sorted {
        let percentage = **count as f64 / total_errors as f64 * 100.0;
        println!("{:>6} ({:5.1}%)  {}", count, percentage, classification);
    }

    println!();
    println!("===========================================");
    println!("SAMPLE ERRORS (top 5 categories)");
    println!("===========================================");

    for (classification, _) in sorted.iter().take(5) {
        println!();
        println!("--- {} ---", classification);
        if let Some(samples) = sample_errors.get(*classification) {
            for (file, context, line) in samples.iter().take(1) {
                let filename = Path::new(file)
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("");
                println!("File: {}:{}", filename, line);
                println!("{}", context);
            }
        }
    }

    println!();
    println!("===========================================");
    println!("MITIGATION STRATEGIES");
    println!("===========================================");

    // Generate recommendations
    let mut mitigations: Vec<(&str, &str, usize)> = Vec::new();

    for (classification, count) in &sorted {
        let mitigation = match classification.as_str() {
            s if s.starts_with("unknown type:") => {
                ("Add type to preprocessor type_macros", "preprocessor.rs")
            }
            "DEFINE_* macro declaration" => (
                "Expand DEFINE_MUTEX/SPINLOCK to variable declarations",
                "preprocessor.rs",
            ),
            "LIST_HEAD declaration" => (
                "Expand LIST_HEAD to struct initialization",
                "preprocessor.rs",
            ),
            "MODULE_* macro" => (
                "Strip MODULE_* lines (already partially handled)",
                "preprocessor.rs",
            ),
            "__attribute__ syntax" => ("Improve __attribute__ stripping regex", "preprocessor.rs"),
            "typeof expression" => (
                "Replace typeof() with void* or generic type",
                "preprocessor.rs",
            ),
            "container_of macro" => ("Expand container_of to cast expression", "preprocessor.rs"),
            "likely/unlikely macro" => ("Strip likely()/unlikely() wrappers", "preprocessor.rs"),
            "assertion macro" => (
                "Strip or expand BUILD_BUG_ON/WARN_ON macros",
                "preprocessor.rs",
            ),
            _ => ("Manual review needed", "visitor.rs"),
        };
        mitigations.push((classification.as_str(), mitigation.0, **count));
    }

    for (category, mitigation, count) in mitigations.iter().take(10) {
        println!("{:>6} errors: {} -> {}", count, category, mitigation);
    }
}

fn print_analysis_summary(error_classifications: &HashMap<String, usize>, total_errors: usize) {
    println!("Total errors found: {}", total_errors);
    println!();

    // Sort by frequency
    let mut sorted: Vec<_> = error_classifications.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    println!("ERROR CLASSIFICATION (by frequency)");
    println!("-------------------------------------------");

    for (classification, count) in sorted.iter().take(15) {
        let percentage = **count as f64 / total_errors as f64 * 100.0;
        println!("{:>6} ({:5.1}%)  {}", count, percentage, classification);
    }
}
