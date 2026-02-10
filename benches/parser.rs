use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use autocomplete_rs::engine::CompletionEngine;
use autocomplete_rs::parser::ParserEngine;
use autocomplete_rs::parser::tokenizer::tokenize;
use autocomplete_rs::protocol::{CompletionRequest, PROTOCOL_VERSION};

fn bench_tokenize(c: &mut Criterion) {
    let long_pipe = "cat file.txt | grep -i pattern | sort -u | head -n 20 | tee output.log";
    let quoted = r#"echo "hello world" | grep 'foo bar' && echo done"#;

    let inputs: &[(&str, &str, usize)] = &[
        ("simple", "ls -la", 6),
        ("medium", "git checkout feature-branch", 28),
        ("pipeline", long_pipe, long_pipe.len()),
        ("quoted", quoted, quoted.len()),
    ];

    let mut group = c.benchmark_group("parser/tokenize");
    for &(label, buffer, cursor) in inputs {
        group.bench_with_input(BenchmarkId::new("tokenize", label), &(), |b, _| {
            b.iter(|| tokenize(black_box(buffer), black_box(cursor)))
        });
    }
    group.finish();
}

fn bench_complete(c: &mut Criterion) {
    let engine: Arc<dyn CompletionEngine> = Arc::new(ParserEngine::new());

    let long_pipe = "cat file.txt | grep -i pattern | sort -u | head -n 20 | tee output.log";
    let quoted = r#"echo "hello world" | grep 'foo bar' && echo done"#;

    let inputs: &[(&str, &str, usize)] = &[
        ("simple", "ls -la", 6),
        ("medium", "git checkout feature-branch", 28),
        ("pipeline", long_pipe, long_pipe.len()),
        ("quoted", quoted, quoted.len()),
    ];

    let mut group = c.benchmark_group("parser/complete");
    for &(label, buffer, cursor) in inputs {
        let req = CompletionRequest {
            buffer: buffer.to_string(),
            cursor,
            version: PROTOCOL_VERSION,
        };
        group.bench_with_input(BenchmarkId::new("parser", label), &req, |b, req| {
            b.iter(|| engine.complete(black_box(req)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_tokenize, bench_complete);
criterion_main!(benches);
