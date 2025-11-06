//! Benchmarks for the Melbi evaluator.
//!
//! This file contains benchmarks to measure evaluator performance.
//! Run with: `cargo bench` in the core/ directory.
//!
//! Benchmark groups:
//! 1. eval_only: Measures pure evaluation performance (expressions are pre-parsed/analyzed)
//! 2. full_pipeline: Measures parse + analyze + eval together (for comparison)
//! 3. cel_comparison: Comparison with CEL (Common Expression Language) interpreter

use bumpalo::Bump;
use cel_interpreter::{Context, Program};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use melbi_core::{analyzer, evaluator, parser, types::manager::TypeManager};
use pprof::criterion::{Output, PProfProfiler};

/// Generate an arithmetic expression like "1 + 1 + 1 + ... + 1" with `n` additions.
fn generate_arithmetic_chain(n: usize) -> String {
    if n == 0 {
        return "1".to_string();
    }

    let mut expr = String::from("1");
    for _ in 0..n {
        expr.push_str(" + 1");
    }
    expr
}

/// Benchmark: Pure evaluation performance (pre-parsed and pre-analyzed).
///
/// This measures only the evaluator's performance, isolating it from parsing and analysis.
fn bench_eval_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_only");

    // Sizes chosen to stay under default stack depth limit (1000)
    for size in [100, 200, 400, 800] {
        // Set throughput to measure operations per second
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            // Setup: Parse and analyze the expression once
            let arena = Bump::new();
            let type_manager = TypeManager::new(&arena);
            let source = generate_arithmetic_chain(size);
            let parsed = parser::parse(&arena, &source).expect("Parse failed");
            let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[])
                .expect("Analysis failed");

            // Benchmark: Only the evaluation step
            b.iter(|| {
                let result = evaluator::eval(
                    black_box(&arena),
                    black_box(type_manager),
                    black_box(&typed),
                    black_box(&[]),
                    black_box(&[]),
                );
                // Extract the integer value to avoid lifetime issues
                let value = result.expect("Eval failed").as_int().expect("Expected int");
                black_box(value)
            });
        });
    }

    group.finish();
}

/// Benchmark: Full pipeline (parse + analyze + eval).
///
/// This measures the complete pipeline to understand where time is spent.
/// Compare with eval_only to see what percentage of time is spent in the evaluator.
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    // Sizes chosen to stay under default stack depth limit (1000)
    for size in [100, 200, 400, 800] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let source = generate_arithmetic_chain(size);

            // Benchmark: Parse + Analyze + Eval
            b.iter(|| {
                let arena = Bump::new();
                let type_manager = TypeManager::new(&arena);

                let parsed =
                    parser::parse(black_box(&arena), black_box(&source)).expect("Parse failed");

                let typed = analyzer::analyze(
                    black_box(type_manager),
                    black_box(&arena),
                    black_box(&parsed),
                    black_box(&[]),
                    black_box(&[]),
                )
                .expect("Analysis failed");

                let result = evaluator::eval(
                    black_box(&arena),
                    black_box(type_manager),
                    black_box(&typed),
                    black_box(&[]),
                    black_box(&[]),
                );

                // Extract the integer value to avoid lifetime issues
                let value = result.expect("Eval failed").as_int().expect("Expected int");
                black_box(value)
            });
        });
    }

    group.finish();
}

/// Benchmark: CEL (Common Expression Language) interpreter for comparison.
///
/// Benchmarks the same arithmetic chains using cel-interpreter to provide
/// a performance baseline comparison with another Rust expression evaluator.
fn bench_cel_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("cel_comparison");

    for size in [100, 200, 400, 800] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let source = generate_arithmetic_chain(size);

            // Setup: Compile the CEL program once
            let program = Program::compile(&source).expect("CEL compilation failed");
            let context = Context::default();

            // Benchmark: Only the evaluation step (similar to eval_only)
            b.iter(|| {
                let result = program.execute(black_box(&context));
                let value = result.expect("CEL eval failed");
                black_box(value)
            });
        });
    }

    group.finish();
}

/// Benchmark: CEL full pipeline (compile + execute).
///
/// Measures CEL's compile + execute to compare with Melbi's full_pipeline.
fn bench_cel_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("cel_full_pipeline");

    for size in [100, 200, 400, 800] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let source = generate_arithmetic_chain(size);

            // Benchmark: Compile + Execute
            b.iter(|| {
                let program = Program::compile(black_box(&source)).expect("CEL compilation failed");
                let context = Context::default();
                let result = program.execute(black_box(&context));
                let value = result.expect("CEL eval failed");
                black_box(value)
            });
        });
    }

    group.finish();
}

// Configure Criterion with profiling support
criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_eval_only, bench_full_pipeline, bench_cel_comparison, bench_cel_full_pipeline
}
criterion_main!(benches);
