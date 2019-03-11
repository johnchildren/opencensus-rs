use criterion::{criterion_group, criterion_main, Criterion};

use opencensus_trace::{
    always_sample, never_sample, set_global_default_sampler, start_span, AttributeValue, SpanID,
    TraceID,
};

fn benchmark_start_span_always_sample(c: &mut Criterion) {
    let ctx = io_context::Context::background().freeze();
    let attr_sets = vec![
        vec![],
        vec![
            ("key1", AttributeValue::BoolAttribute(false)),
            (
                "key2",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            ("key2", AttributeValue::Int64Attribute(123)),
        ],
        vec![
            ("key1", AttributeValue::BoolAttribute(false)),
            ("key2", AttributeValue::BoolAttribute(true)),
            (
                "key3",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            (
                "key4",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            ("key5", AttributeValue::Int64Attribute(123)),
            ("key6", AttributeValue::Int64Attribute(456)),
        ],
    ];

    set_global_default_sampler(&always_sample());
    c.bench_function_over_inputs(
        "start_span/always_sample",
        move |b, attrs| {
            b.iter(|| {
                let (_, mut span) = start_span(&ctx, "/foo", &[]);
                span.add_attributes(attrs.clone());
                span.end();
            })
        },
        attr_sets,
    );
}

fn benchmark_start_span_never_sample(c: &mut Criterion) {
    let ctx = io_context::Context::background().freeze();
    let attr_sets = vec![
        vec![],
        vec![
            ("key1", AttributeValue::BoolAttribute(false)),
            (
                "key2",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            ("key2", AttributeValue::Int64Attribute(123)),
        ],
        vec![
            ("key1", AttributeValue::BoolAttribute(false)),
            ("key2", AttributeValue::BoolAttribute(true)),
            (
                "key3",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            (
                "key4",
                AttributeValue::StringAttribute(String::from("hello")),
            ),
            ("key5", AttributeValue::Int64Attribute(123)),
            ("key6", AttributeValue::Int64Attribute(456)),
        ],
    ];

    set_global_default_sampler(&never_sample());
    c.bench_function_over_inputs(
        "start_span/never_sample",
        move |b, attrs| {
            b.iter(|| {
                let (_, mut span) = start_span(&ctx, "/foo", &[]);
                span.add_attributes(attrs.clone());
                span.end();
            })
        },
        attr_sets,
    );
}

criterion_group!(
    start_span_benches,
    benchmark_start_span_always_sample,
    benchmark_start_span_never_sample
);

fn benchmark_trace_id_display(c: &mut Criterion) {
    let span = TraceID([
        0x0D, 0x0E, 0x0A, 0x0D, 0x0B, 0x0E, 0x0E, 0x0F, 0x0F, 0x0E, 0x0E, 0x0B, 0x0D, 0x0A, 0x0E,
        0x0D,
    ]);
    let want = "0d0e0a0d0b0e0e0f0f0e0e0b0d0a0e0d";
    c.bench_function("TraceID::fmt", move |b| {
        b.iter(|| {
            assert_eq!(format!("{}", span), want);
        })
    });
}

fn benchmark_span_id_display(c: &mut Criterion) {
    let span = SpanID([0x0D, 0x0E, 0x0A, 0x0D, 0x0B, 0x0E, 0x0E, 0x0F]);
    let want = "0d0e0a0d0b0e0e0f";
    c.bench_function("SpanID::fmt", move |b| {
        b.iter(|| {
            assert_eq!(format!("{}", span), want);
        })
    });
}

criterion_group!(
    display_benches,
    benchmark_trace_id_display,
    benchmark_span_id_display
);

criterion_main!(start_span_benches, display_benches);
