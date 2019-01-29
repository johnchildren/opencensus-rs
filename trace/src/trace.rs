use std::collections::HashMap;
use std::fmt;
use std::iter::IntoIterator;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use io_context::Context;

use crate::basetypes::{AttributeValue, Link, SpanID, Status, TraceID};
use crate::config;
use crate::export::SpanData;
use crate::sampling::{Sampler, SamplingParameters};
use crate::tracestate::Tracestate;

/// Span represents a span of a trace.  It has an associated SpanContext, and
/// stores data accumulated while the span is active.
///
/// Ideally users should interact with Spans by calling the functions in this
/// package that take a Context parameter.
#[derive(Debug, Clone)]
pub struct Span {
    /// data contains information recorded about the span.
    ///
    /// It will be some if we are exporting the span or recording events for it.
    /// Otherwise, data is none, and the Span is simply a carrier for the
    /// SpanContext, so that the trace ID is propagated.
    data: Option<Arc<RwLock<SpanData>>>,
    span_context: SpanContext,
    //store: Option<Arc<SpanStore<'a>>>,
    //end_once: Once,
}

pub fn start_span(ctx: &Arc<Context>, name: &str, o: &[StartOption]) -> (Context, Arc<Span>) {
    let mut opts = StartOptions::default();
    let parent = from_context(ctx).map(|p| &p.span_context);
    for op in o {
        op(&mut opts);
    }
    let span = start_span_internal(name, parent, false, &opts);

    (new_context(&ctx, Arc::clone(&span)), span)
}

pub fn start_span_with_remote_parent(
    ctx: &Arc<Context>,
    name: &str,
    parent: &SpanContext,
    o: &[StartOption],
) -> (Context, Arc<Span>) {
    let mut opts = StartOptions::default();
    for op in o.into_iter() {
        op(&mut opts);
    }

    let span = start_span_internal(name, Some(parent), false, &opts);

    (new_context(&ctx, Arc::clone(&span)), span)
}

fn start_span_internal(
    name: &str,
    parent: Option<&SpanContext>,
    remote_parent: bool,
    o: &StartOptions,
) -> Arc<Span> {
    let mut span_context = parent
        .map(SpanContext::clone)
        .unwrap_or_else(SpanContext::default);

    let cfg = config::load_config();

    let id_generator = Arc::clone(&cfg.id_generator);
    if parent.is_none() {
        span_context.trace_id = id_generator.new_trace_id();
    }
    span_context.span_id = id_generator.new_span_id();
    let mut sampler = cfg.default_sampler;

    if parent.is_none() || remote_parent || o.sampler.is_some() {
        if let Some(s) = &o.sampler {
            sampler = Arc::clone(s);
        }
        span_context.set_is_sampled(
            sampler(SamplingParameters {
                parent_context: parent,
                trace_id: &span_context.trace_id,
                span_id: &span_context.span_id,
                name,
                has_remote_parent: remote_parent,
            })
            .sample,
        );
    }

    //TODO(john|p=2|#feature): Enable local span store configuration.
    if !span_context.is_sampled() {
        return Arc::new(Span {
            data: None,
            span_context,
            //end_once: Once::new(),
        });
    }

    let data = SpanData {
        span_context: span_context.clone(),
        parent_span_id: parent.map(|p| p.span_id),
        span_kind: o.span_kind,
        name: name.to_string(),
        start_time: Instant::now(),
        end_time: None,
        attributes: HashMap::new(),
        annotations: Vec::new(),
        message_events: Vec::new(),
        status: None,
        links: Vec::new(),
        has_remote_parent: remote_parent,
    };

    Arc::new(Span {
        data: Some(Arc::new(RwLock::new(data))),
        span_context,
        //end_once: Once::new(),
    })
}

impl Span {
    pub fn end(&self) {
        if !self.is_recording_events() {
            return;
        }
    }

    pub fn is_recording_events(&self) -> bool {
        self.data.is_some()
    }

    fn make_span_data(&self) -> Option<RwLock<SpanData>> {
        if let Some(data) = &self.data {
            let data = data.read().unwrap();
            Some(RwLock::new((*data).clone()))
        } else {
            None
        }
    }

    pub fn span_context(&self) -> &SpanContext {
        &self.span_context
    }

    pub fn set_name(&mut self, name: &str) {
        if let Some(data) = &self.data {
            let mut data = data.write().unwrap();
            (*data).name = name.to_string();
        }
    }

    pub fn set_status(&mut self, status: &Status) {
        if let Some(data) = &self.data {
            let mut data = data.write().unwrap();
            (*data).status = Some(status.clone());
        }
    }

    pub fn add_attributes(&mut self, attrs: impl IntoIterator<Item = (String, AttributeValue)>) {
        if let Some(data) = &self.data {
            let mut data = data.write().unwrap();
            (*data).attributes = attrs.into_iter().collect();
        }
    }

    pub fn add_link(&mut self, l: Link) {
        if let Some(data) = &self.data {
            let mut data = data.write().unwrap();
            (*data).links.push(l);
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(data) = &self.data {
            let data = data.read().unwrap();
            write!(f, "span {} {}", self.span_context.span_id, data.name)?;
        } else {
            write!(f, "span {}", self.span_context.span_id)?;
        }
        Ok(())
    }
}

const SPAN_ID_KEY: &str = "OPENCENSUS_TRACE_SPAN_ID_KEY";

pub fn from_context(ctx: &Context) -> Option<&Arc<Span>> {
    ctx.get_value(SPAN_ID_KEY)
}

pub fn new_context(parent: &Arc<Context>, span: Arc<Span>) -> Context {
    let mut ctx = Context::create_child(parent);
    ctx.add_value(SPAN_ID_KEY, span);
    ctx
}

/// SpanContext contains the state that must propagate across process boundaries.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct SpanContext {
    pub trace_id: TraceID,
    pub span_id: SpanID,
    pub trace_options: TraceOptions,
    pub trace_state: Option<Tracestate>,
}

impl SpanContext {
    /// is_sampled returns true if the span will be exported.
    pub fn is_sampled(&self) -> bool {
        self.trace_options.is_sampled()
    }

    /// set_is_sampled sets the TraceOptions bit that determines whether the
    /// span will be exported.
    fn set_is_sampled(&mut self, sampled: bool) {
        if sampled {
            self.trace_options.0 |= 1
        } else {
            self.trace_options.0 &= !1
        }
    }
}

/// TraceOptions contains options associated with a trace span.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct TraceOptions(pub u32); // ???

impl TraceOptions {
    /// Whether the trace should be sampled.
    pub fn is_sampled(self) -> bool {
        self.0 & 1 == 1
    }
}

/// All available span kinds. Span kind must be either one of these values.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpanKind {
    Unspecified = 1,
    Server,
    Client,
}

impl Default for SpanKind {
    fn default() -> SpanKind {
        SpanKind::Unspecified
    }
}

/// StartOptions contains options concerning how a span is started.
#[derive(Clone, Default)]
pub struct StartOptions {
    /// Sampler to consult for this Span. If provided, it is always consulted.
    ///
    /// If not provided, then the behavior differs based on whether
    /// the parent of this Span is remote, local, or there is no parent.
    /// In the case of a remote parent or no parent, the
    /// default sampler (see Config) will be consulted. Otherwise,
    /// when there is a non-remote parent, no new sampling decision will be made:
    /// we will preserve the sampling of the parent.
    pub sampler: Option<Sampler>,

    /// SpanKind represents the kind of a span. Defaults to Unspecified.
    pub span_kind: SpanKind,
}

/// StartOption applies changes to StartOptions.
type StartOption = Box<dyn Fn(&mut StartOptions)>;

/// with_span_kind makes new spans to be created with the given kind.
pub fn with_span_kind(span_kind: SpanKind) -> StartOption {
    Box::new(move |o: &mut StartOptions| o.span_kind = span_kind)
}

/// with_sampler makes new spans to be created with a custom sampler.
pub fn with_sampler(sampler: Sampler) -> StartOption {
    Box::new(move |o: &mut StartOptions| o.sampler = Some(Arc::clone(&sampler)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::export::Exporter;
    use crate::tracestate::{Key, Value};

    const TID: TraceID = TraceID([1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 4, 8, 16, 32, 64, 128]);
    const SID: SpanID = SpanID([1, 2, 4, 8, 16, 32, 64, 128]);

    struct TestExporter {
        exported: Vec<SpanData>,
    }

    impl Exporter for TestExporter {
        fn export_span(&mut self, s: &SpanData) {
            self.exported.push(s.clone())
        }
    }

    #[test]
    fn id_string_represenation() {
        assert_eq!(format!("{}", TID), "01020304050607080102040810204080");
        assert_eq!(format!("{}", SID), "0102040810204080");
    }

    #[test]
    fn context_roundtrip() {
        let span_context = SpanContext {
            trace_id: TID,
            span_id: SID,
            trace_options: TraceOptions(0),
            trace_state: None,
        };
        let want = Arc::new(Span {
            data: None,
            span_context: span_context.clone(),
            //store: None,
            //end_once: Once::new(),
        });
        let ctx = new_context(&Context::background().freeze(), want);
        let got = from_context(&ctx);

        // can't compare mutexes
        assert_eq!(got.unwrap().span_context, span_context);
    }

    #[test]
    fn start_span_doesnt_record() {
        let (ctx, _) = start_span(&Context::background().freeze(), "start_span", &[]);
        assert!(!from_context(&ctx).unwrap().is_recording_events())
    }

    #[test]
    fn sampling_sets_trace_options_correctly() {
        use crate::sampling::{always_sample, never_sample};

        enum Parent {
            Remote,
            Local,
            None,
        }

        struct TestCase {
            pub parent: Parent,
            pub parent_trace_options: TraceOptions,
            pub sampler: Option<Sampler>,
            pub want_trace_options: TraceOptions,
        }

        let test_cases = &[
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(0),
                sampler: None,
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(1),
                sampler: None,
                want_trace_options: TraceOptions(1),
            },
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(0),
                sampler: Some(never_sample()),
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(1),
                sampler: Some(never_sample()),
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(0),
                sampler: Some(always_sample()),
                want_trace_options: TraceOptions(1),
            },
            TestCase {
                parent: Parent::Remote,
                parent_trace_options: TraceOptions(1),
                sampler: Some(always_sample()),
                want_trace_options: TraceOptions(1),
            },
            TestCase {
                parent: Parent::Local,
                parent_trace_options: TraceOptions(0),
                sampler: Some(never_sample()),
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::Local,
                parent_trace_options: TraceOptions(1),
                sampler: Some(never_sample()),
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::Local,
                parent_trace_options: TraceOptions(0),
                sampler: Some(always_sample()),
                want_trace_options: TraceOptions(1),
            },
            TestCase {
                parent: Parent::Local,
                parent_trace_options: TraceOptions(1),
                sampler: Some(always_sample()),
                want_trace_options: TraceOptions(1),
            },
            TestCase {
                parent: Parent::None,
                parent_trace_options: TraceOptions(0),
                sampler: Some(never_sample()),
                want_trace_options: TraceOptions(0),
            },
            TestCase {
                parent: Parent::None,
                parent_trace_options: TraceOptions(0),
                sampler: Some(always_sample()),
                want_trace_options: TraceOptions(1),
            },
        ];

        for test in test_cases {
            let (ctx, _) = match test.parent {
                Parent::Remote => {
                    let sc = SpanContext {
                        trace_id: TID,
                        span_id: SID,
                        trace_options: test.parent_trace_options,
                        trace_state: None,
                    };
                    match &test.sampler {
                        Some(sampler) => start_span_with_remote_parent(
                            &Context::background().freeze(),
                            "foo",
                            &sc,
                            &[with_sampler(Arc::clone(sampler))],
                        ),
                        None => start_span_with_remote_parent(
                            &Context::background().freeze(),
                            "foo",
                            &sc,
                            &[],
                        ),
                    }
                }
                Parent::Local => {
                    let sampler = if test.parent_trace_options == TraceOptions(1) {
                        crate::sampling::always_sample()
                    } else {
                        crate::sampling::never_sample()
                    };
                    let (ctx2, _) = start_span(
                        &Context::background().freeze(),
                        "foo",
                        &[with_sampler(sampler)],
                    );
                    match &test.sampler {
                        Some(sampler) => {
                            start_span(&ctx2.freeze(), "foo", &[with_sampler(Arc::clone(sampler))])
                        }
                        None => start_span(&ctx2.freeze(), "foo", &[]),
                    }
                }
                Parent::None => match &test.sampler {
                    Some(sampler) => start_span(
                        &Context::background().freeze(),
                        "foo",
                        &[with_sampler(Arc::clone(sampler))],
                    ),
                    None => start_span(&Context::background().freeze(), "foo", &[]),
                },
            };
            match from_context(&ctx) {
                None => panic!("no span in context!"),
                Some(span) => {
                    let sc = &span.span_context;
                    assert!(sc.span_id != SpanID([0; 8]));
                    assert_eq!(sc.trace_options, test.want_trace_options);
                }
            }
        }
    }

    #[test]
    fn sampler_has_no_effect_on_local_children() {}

    #[test]
    fn probability_sampler_samples_approximately() {
        use crate::sampling::probability_sampler;
        let mut exported: u64 = 0;
        for _ in 0..1000 {
            let (_, span) = start_span(
                &Context::background().freeze(),
                "foo",
                &[with_sampler(probability_sampler(0.3))],
            );
            if span.span_context.is_sampled() {
                exported += 1;
            }
        }
        // potentially flakey, but unavoidable.
        if exported < 200 || exported > 400 {
            panic!(
                "number of spans out of expected bounds, want approx 30% got {}",
                (exported as f64) * 0.1
            );
        }
    }

    #[test]
    fn start_with_remote_parent_works() {
        fn check_child(p: &SpanContext, c: &Span) {
            assert_eq!(c.span_context.trace_id, p.trace_id);
            assert!(c.span_context.span_id != p.span_id);
            assert_eq!(c.span_context.trace_options, p.trace_options);
            assert_eq!(c.span_context.trace_state, p.trace_state);
        }

        let sc = SpanContext {
            trace_id: TID,
            span_id: SID,
            trace_options: TraceOptions(0),
            trace_state: None,
        };

        let (ctx, _) = start_span_with_remote_parent(
            &Context::background().freeze(),
            "start_span_with_remote_parent",
            &sc,
            &[],
        );
        check_child(&sc, from_context(&ctx).unwrap());

        let (ctx, _) = start_span_with_remote_parent(
            &Context::background().freeze(),
            "start_span_with_remote_parent",
            &sc,
            &[],
        );
        check_child(&sc, from_context(&ctx).unwrap());

        let trace_state: Tracestate = Tracestate::try_new(
            None,
            &[(Key::try_new("foo").unwrap(), Value::try_new("bar").unwrap())],
        )
        .unwrap();
        let sc = SpanContext {
            trace_id: TID,
            span_id: SID,
            trace_options: TraceOptions(0),
            trace_state: Some(trace_state),
        };

        let (ctx, _) = start_span_with_remote_parent(
            &Context::background().freeze(),
            "start_span_with_remote_parent",
            &sc,
            &[],
        );
        check_child(&sc, from_context(&ctx).unwrap());

        let (ctx, _) = start_span_with_remote_parent(
            &Context::background().freeze(),
            "start_span_with_remote_parent",
            &sc,
            &[],
        );
        check_child(&sc, from_context(&ctx).unwrap());

        let ctx = ctx.freeze();
        let (ctx2, _) = start_span(&ctx, "StartSpan", &[]);
        let parent = from_context(&ctx).unwrap().span_context();
        check_child(parent, from_context(&ctx2).unwrap());
    }

    #[test]
    fn span_kind() {
        use crate::export::{register_exporter, unregister_exporter};

        fn start_span_helper(o: &[StartOption]) -> Arc<Span> {
            let (_, span) = start_span_with_remote_parent(
                &Context::background().freeze(),
                "span0",
                &SpanContext {
                    trace_id: TID,
                    span_id: SID,
                    trace_options: TraceOptions(1),
                    trace_state: None,
                },
                o,
            );
            span
        }

        fn end_span(span: Arc<Span>) {
            assert!(span.is_recording_events());
            assert!(span.span_context.is_sampled());

            let te: Arc<dyn Exporter + Send + Sync> = Arc::new(TestExporter {
                exported: Vec::new(),
            });

            register_exporter(Arc::clone(&te));
            span.end();
            unregister_exporter(&te);
        }

        struct TestCase {
            name: &'static str,
            start_options: Vec<StartOption>,
            want: SpanData,
        }

        let test_cases = &[
            TestCase {
                name: "default StartOptions",
                start_options: vec![with_span_kind(SpanKind::Unspecified)],
                want: SpanData {
                    span_context: SpanContext {
                        trace_id: TID,
                        span_id: SpanID([0; 8]),
                        trace_options: TraceOptions(1),
                        trace_state: None,
                    },
                    parent_span_id: Some(SID),
                    name: "span0".to_string(),
                    span_kind: SpanKind::Unspecified,
                    has_remote_parent: true,

                    start_time: Instant::now(),
                    end_time: None,
                    attributes: HashMap::new(),
                    annotations: Vec::new(),
                    message_events: Vec::new(),
                    status: None,
                    links: Vec::new(),
                },
            },
            TestCase {
                name: "client span",
                start_options: vec![with_span_kind(SpanKind::Client)],
                want: SpanData {
                    span_context: SpanContext {
                        trace_id: TID,
                        span_id: SpanID([0; 8]),
                        trace_options: TraceOptions(1),
                        trace_state: None,
                    },
                    parent_span_id: Some(SID),
                    name: "span0".to_string(),
                    span_kind: SpanKind::Client,
                    has_remote_parent: true,

                    start_time: Instant::now(),
                    end_time: None,
                    attributes: HashMap::new(),
                    annotations: Vec::new(),
                    message_events: Vec::new(),
                    status: None,
                    links: Vec::new(),
                },
            },
            TestCase {
                name: "server span",
                start_options: vec![with_span_kind(SpanKind::Server)],
                want: SpanData {
                    span_context: SpanContext {
                        trace_id: TID,
                        span_id: SpanID([0; 8]),
                        trace_options: TraceOptions(1),
                        trace_state: None,
                    },
                    parent_span_id: Some(SID),
                    name: "span0".to_string(),
                    span_kind: SpanKind::Server,
                    has_remote_parent: true,

                    start_time: Instant::now(),
                    end_time: None,
                    attributes: HashMap::new(),
                    annotations: Vec::new(),
                    message_events: Vec::new(),
                    status: None,
                    links: Vec::new(),
                },
            },
        ];

        for test in test_cases {
            let mut span = start_span_helper(&test.start_options);
            let got = end_span(span);
        }
    }
}