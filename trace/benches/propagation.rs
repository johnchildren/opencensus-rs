use criterion::{criterion_group, criterion_main, Criterion};

use opencensus_trace::propagation::{from_binary, to_binary};
use opencensus_trace::{SpanContext, SpanID, TraceID, TraceOptions};

fn benchmark_to_binary(c: &mut Criterion) {
    let trace_id = TraceID([
        0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e,
        0x4f,
    ]);
    let span_id = SpanID([0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);
    let trace_options = TraceOptions(0);
    let span_context = SpanContext {
        trace_id,
        span_id,
        trace_options,
        trace_state: None,
    };

    c.bench_function("to_binary", move |b| {
        b.iter(|| {
            to_binary(&span_context);
        })
    });
}

fn benchmark_from_binary(c: &mut Criterion) {
    let bin = vec![
        0, 0, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 1, 97, 98, 99, 100,
        101, 102, 103, 104, 2, 1,
    ];

    c.bench_function("from_binary", move |b| {
        b.iter(|| {
            from_binary(&bin);
        })
    });
}

criterion_group!(benches, benchmark_to_binary, benchmark_from_binary);

criterion_main!(benches);
