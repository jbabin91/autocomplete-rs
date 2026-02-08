use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

use autocomplete_rs::daemon::handler::handle_connection;
use autocomplete_rs::daemon::state::DaemonState;
use autocomplete_rs::engine::StubEngine;
use autocomplete_rs::logging;

fn bench_handle_connection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let state = DaemonState::new(Arc::new(StubEngine), logging::Mode::Production);

    let bare_request = b"{\"buffer\":\"git checkout feature-branch\",\"cursor\":28}\n";
    let envelope_request =
        b"{\"type\":\"complete\",\"buffer\":\"git checkout feature-branch\",\"cursor\":28}\n";

    let mut group = c.benchmark_group("handler/roundtrip");

    group.bench_function("bare_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                let reader: &[u8] = bare_request;
                let writer = tokio::io::sink();
                handle_connection(reader, writer, &state, 1).await.unwrap();
            });
        });
    });

    group.bench_function("envelope_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                let reader: &[u8] = envelope_request;
                let writer = tokio::io::sink();
                handle_connection(reader, writer, &state, 1).await.unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_handle_connection);
criterion_main!(benches);
