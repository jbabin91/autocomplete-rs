use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use autocomplete_rs::engine::{CompletionEngine, StubEngine};
use autocomplete_rs::protocol::{CompletionRequest, PROTOCOL_VERSION};

fn bench_complete(c: &mut Criterion) {
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);

    let long = "x".repeat(1000);
    let inputs: &[(&str, &str, usize)] = &[
        ("short", "ls", 2),
        ("medium", "git checkout feature-branch", 28),
        ("long", &long, 1000),
    ];

    let mut group = c.benchmark_group("engine/complete");
    for &(label, buffer, cursor) in inputs {
        let req = CompletionRequest {
            buffer: buffer.to_string(),
            cursor,
            version: PROTOCOL_VERSION,
        };
        group.bench_with_input(BenchmarkId::new("stub", label), &req, |b, req| {
            b.iter(|| engine.complete(black_box(req)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_complete);
criterion_main!(benches);
