use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use autocomplete_rs::protocol::{
    CompletionRequest, DaemonMessage, MAX_BUFFER_LEN, validate_request,
};

fn bench_deserialization(c: &mut Criterion) {
    let bare_json = r#"{"buffer":"git checkout feature-branch","cursor":28}"#;
    let envelope_json = r#"{"type":"complete","buffer":"git checkout feature-branch","cursor":28}"#;
    let shutdown_json = r#"{"type":"shutdown"}"#;
    let malformed_json = r#"{"not_a_valid_field": true}"#;

    let mut group = c.benchmark_group("protocol/deserialize");

    group.bench_function("bare_request", |b| {
        b.iter(|| serde_json::from_str::<CompletionRequest>(bare_json).unwrap());
    });

    group.bench_function("envelope_complete", |b| {
        b.iter(|| serde_json::from_str::<DaemonMessage>(envelope_json).unwrap());
    });

    group.bench_function("envelope_shutdown", |b| {
        b.iter(|| serde_json::from_str::<DaemonMessage>(shutdown_json).unwrap());
    });

    group.bench_function("malformed", |b| {
        b.iter(|| {
            let _: Result<DaemonMessage, _> = serde_json::from_str(malformed_json);
        });
    });

    group.finish();
}

fn bench_validation(c: &mut Criterion) {
    let valid = CompletionRequest {
        buffer: "git commit -m 'message'".into(),
        cursor: 23,
        version: 1,
    };
    let cursor_oob = CompletionRequest {
        buffer: "ls".into(),
        cursor: 99,
        version: 1,
    };
    let max_buffer = CompletionRequest {
        buffer: "x".repeat(MAX_BUFFER_LEN),
        cursor: 0,
        version: 1,
    };

    let mut group = c.benchmark_group("protocol/validate");

    let cases: &[(&str, &CompletionRequest)] = &[
        ("valid", &valid),
        ("cursor_oob", &cursor_oob),
        ("max_buffer", &max_buffer),
    ];
    for &(label, req) in cases {
        group.bench_with_input(BenchmarkId::new("request", label), req, |b, req| {
            b.iter(|| validate_request(req));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_deserialization, bench_validation);
criterion_main!(benches);
