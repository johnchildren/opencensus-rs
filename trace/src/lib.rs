/*!
Package trace contains support for OpenCensus distributed tracing.

The following assumes a basic familiarity with OpenCensus concepts.
See http://opencensus.io


### Exporting Traces

To export collected tracing data, register at least one exporter. You can use
one of the provided exporters or write your own.

```ignore
use opencensus_trace::register_exporter;

register_exporter(exporter)
```

By default, traces will be sampled relatively rarely. To change the sampling
frequency for your entire program, call set_global_default_sampler. Use a ProbabilitySampler
to sample a subset of traces, or use AlwaysSample to collect a trace on every run:

```rust
use opencensus_trace::sampling;
use opencensus_trace::{set_global_default_sampler, Config};

set_global_default_sampler(&sampling::always_sample());
```

Be careful about using always_sample in a production application with
significant traffic: a new trace will be started and exported for every request.

### Adding Spans to a Trace

A trace consists of a tree of spans. In Rust, the current span is carried in an
io_context::Context.

It is common to want to capture all the activity of a function call in a span. For
this to work, the function must take an io_context::Context as a parameter. Add
these two lines to the top of the function:

```rust
use opencensus_trace::start_span;

let parent = io_context::Context::background();

let (ctx, span) = start_span(&parent.freeze(), "example.com/Run", &[]);
```

start_span will create a new top-level span if the context
doesn't contain another span, otherwise it will create a child span.
*/
#![warn(missing_docs, rust_2018_idioms, clippy::all)]

mod basetypes;
mod config;
mod export;
mod id_generator;
/// Trace propagation
pub mod propagation;
/// Trace sampling
pub mod sampling;
mod spanbucket;
mod spanstore;
mod status_codes;
mod trace;
mod tracestate;

pub use crate::basetypes::{SpanID, TraceID};
pub use crate::config::{set_global_default_sampler, set_global_id_generator, Config};
pub use crate::export::{register_exporter, unregister_exporter};
pub use crate::trace::{
    start_span, start_span_with_remote_parent, Span, SpanContext, TraceOptions,
};
