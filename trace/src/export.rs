use std::sync::{Arc, RwLock};
use std::time;

use lazy_static::lazy_static;

use crate::basetypes::{Annotation, Attributes, Link, MessageEvent, SpanID, Status};
use crate::trace::{SpanContext, SpanKind};

/// Exporter is a trait for structs that receive sampled trace spans.
///
/// The export_span method should be safe for concurrent use and should return
/// quickly; if an Exporter takes a significant amount of time to process a
/// SpanData, that work should be done on another thread or in a future.
pub trait Exporter {
    fn export_span(&self, s: &SpanData);
}

type Exporters = RwLock<Vec<Arc<dyn Exporter + Send + Sync>>>;

lazy_static! {
    pub static ref EXPORTERS: Exporters = { RwLock::new(Vec::new()) };
}

/// register_exporter adds to the list of Exporters that will receive sampled
/// trace spans.
///
/// Binaries can register exporters, libraries shouldn't register exporters.
pub fn register_exporter(e: Arc<dyn Exporter + Send + Sync>) {
    let mut exporters = EXPORTERS.write().unwrap();
    //TODO(john|p=3|#techdebt): there must be a better way to do this?
    for exporter in &*exporters {
        if Arc::ptr_eq(exporter, &e) {
            return;
        }
    }
    exporters.push(e);
}

/// unregister_exporter removes from the list of Exporters the Exporter that was
/// registered with the given Arc.
pub fn unregister_exporter(e: &Arc<dyn Exporter + Send + Sync>) {
    let mut exporters = EXPORTERS.write().unwrap();
    //TODO(john|p=3|#techdebt): there must be a better way to do this?
    *exporters = (*exporters)
        .iter()
        .filter(|exporter| Arc::ptr_eq(exporter, e))
        .cloned()
        .collect();
}

/// SpanData contains all the information collected by a Span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanData {
    pub span_context: SpanContext,
    pub parent_span_id: Option<SpanID>,
    pub span_kind: SpanKind,
    pub name: String,
    pub start_time: time::Instant,
    /// The wall clock time of EndTime will be adjusted to always be offset
    /// from StartTime by the duration of the span.
    pub end_time: Option<time::Instant>,
    /// The values of Attributes each have type string, bool, or int64.
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub message_events: Vec<MessageEvent>,
    pub status: Option<Status>,
    pub links: Vec<Link>,
    pub has_remote_parent: bool,
}
