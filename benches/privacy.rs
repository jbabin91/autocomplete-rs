use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use autocomplete_rs::logging::{redact_buffer, redact_sensitive_patterns};

fn bench_redact_buffer(c: &mut Criterion) {
    let short = "ls";
    let medium = "git checkout feature-branch";
    let long = "x".repeat(1000);
    let unicode = "\u{00e9}\u{00e0}\u{00fc}".repeat(50); // 150 chars of accented characters

    let mut group = c.benchmark_group("privacy/redact_buffer");

    let cases: &[(&str, &str)] = &[
        ("short_2", short),
        ("medium_27", medium),
        ("long_1000", &long),
        ("unicode_150", &unicode),
    ];
    for &(label, input) in cases {
        group.bench_with_input(BenchmarkId::new("buffer", label), input, |b, input| {
            b.iter(|| redact_buffer(black_box(input)));
        });
    }

    group.finish();
}

fn bench_redact_sensitive_patterns(c: &mut Criterion) {
    let clean = "ls -la /home/user/projects";
    let password = "curl --data password=hunter2 http://example.com";
    let url_creds = "git clone https://user:s3cret@github.com/org/repo.git";
    let export_secret = "export MY_SECRET_KEY=supersecret123";
    let combined = "curl password=hunter2 https://user:pass@host.com/path && \
                    export API_KEY_SECRET=abc123 token=xyz";

    let mut group = c.benchmark_group("privacy/redact_sensitive");

    let cases: &[(&str, &str)] = &[
        ("clean", clean),
        ("password", password),
        ("url_credentials", url_creds),
        ("export_secret", export_secret),
        ("combined", combined),
    ];
    for &(label, input) in cases {
        group.bench_with_input(BenchmarkId::new("patterns", label), input, |b, input| {
            b.iter(|| redact_sensitive_patterns(black_box(input)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_redact_buffer,
    bench_redact_sensitive_patterns
);
criterion_main!(benches);
