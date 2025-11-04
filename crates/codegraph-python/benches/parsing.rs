// Benchmarks will be implemented in Phase 8 (Polish)
use codegraph::CodeGraph;
use codegraph_python::{Parser, ParserConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;

fn bench_parse_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_file");

    group.bench_function("simple.py", |b| {
        b.iter(|| {
            let mut graph = CodeGraph::in_memory().unwrap();
            let parser = Parser::new();
            let path = Path::new("tests/fixtures/simple.py");
            parser.parse_file(black_box(path), &mut graph).unwrap();
        });
    });

    group.finish();
}

fn bench_parse_project(c: &mut Criterion) {
    let mut group = c.benchmark_group("project");

    // Sequential parsing
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut graph = CodeGraph::in_memory().unwrap();
            let config = ParserConfig {
                parallel: false,
                ..Default::default()
            };
            let parser = Parser::with_config(config);
            let path = Path::new("tests/fixtures/test_project");
            parser.parse_directory(black_box(path), &mut graph).unwrap();
        });
    });

    // Parallel parsing with 2 threads
    group.bench_function("parallel_2", |b| {
        b.iter(|| {
            let mut graph = CodeGraph::in_memory().unwrap();
            let config = ParserConfig {
                parallel: true,
                num_threads: Some(2),
                ..Default::default()
            };
            let parser = Parser::with_config(config);
            let path = Path::new("tests/fixtures/test_project");
            parser.parse_directory(black_box(path), &mut graph).unwrap();
        });
    });

    // Parallel parsing with 4 threads
    group.bench_function("parallel_4", |b| {
        b.iter(|| {
            let mut graph = CodeGraph::in_memory().unwrap();
            let config = ParserConfig {
                parallel: true,
                num_threads: Some(4),
                ..Default::default()
            };
            let parser = Parser::with_config(config);
            let path = Path::new("tests/fixtures/test_project");
            parser.parse_directory(black_box(path), &mut graph).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_parse_single_file, bench_parse_project);
criterion_main!(benches);
