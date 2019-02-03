use criterion::{criterion_group, criterion_main, Criterion};

use opencensus_trace::start_span;

fn benchmark_start_span(c: &mut Criterion) {
    let ctx = io_context::Context::background().freeze();
    c.bench_function("start_span", move |b| {
        b.iter(|| {
            let (_, span) = start_span(&ctx, "/foo", &[]);
            span.end();
        })
    });
}

criterion_group!(benches, benchmark_start_span);

criterion_main!(benches);
