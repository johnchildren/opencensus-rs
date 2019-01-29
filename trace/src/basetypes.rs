use std::collections::HashMap;
use std::fmt;
use std::time;

use crate::status_codes::StatusCode;

/// TraceID is a 16-byte identifier for a set of spans.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct TraceID(pub [u8; 16]);

/// SpanID is an 8-byte identifier for a single span.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct SpanID(pub [u8; 8]);

impl fmt::Display for TraceID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vals = self.0;
        for val in vals.iter() {
            write!(f, "{:02x}", val)?;
        }
        Ok(())
    }
}

impl fmt::Display for SpanID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vals = self.0;
        for val in vals.iter() {
            write!(f, "{:02x}", val)?;
        }
        Ok(())
    }
}

/// Annotation represents a text annotation with a set of attributes and a timestamp.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Annotation {
    pub time: time::Instant,
    pub message: String,
    pub attributes: Attributes,
}

/// Attributes represents a key-value pairs on a span, link or annotation.
//TODO(john|p=4|#techdebt): consider using a newtype if not too clunky.
pub type Attributes = HashMap<String, AttributeValue>;

/// AttributeValues are the values of attributes on a span, link or annotation.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AttributeValue {
    BoolAttribute(bool),
    Int64Attribute(i64),
    StringAttribute(String),
}

/// LinkType specifies the relationship between the span that had the link
/// added, and the linked span.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType {
    /// The relationship of the two spans is unknown.
    Unspecified = 0,
    /// The current span is a child of the linked span.
    Child,
    /// The current span is a child of the linked span.
    Parent,
}

/// Link represents a reference from one span to another span.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Link {
    pub trace_id: TraceID,
    pub span_id: SpanID,
    pub _type: LinkType,
    pub attributes: Attributes,
}

/// The current span is a child of the linked span.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum MessageEventType {
    /// Unknown event type.
    Unspecified = 0,
    /// Indicates a sent RPC message.
    Sent,
    /// Indicates a received RPC message.
    Recv,
}

/// MessageEvent represents an event describing a message sent or received on the network.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MessageEvent {
    pub time: time::Instant,
    pub event_type: MessageEventType,
    pub message_id: i64,
    pub uncompressed_byte_size: i64,
    pub compressed_byte_size: i64,
}

/// Status is the status of a Span.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Status {
    pub code: StatusCode,
    pub message: String,
}
