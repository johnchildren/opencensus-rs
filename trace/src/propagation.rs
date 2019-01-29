use crate::basetypes::{SpanID, TraceID};
use crate::trace::{SpanContext, TraceOptions};

/// BinaryFormat format:
///
/// Binary value: <version_id><version_format>
/// version_id: 1 byte representing the version id.
///
/// For version_id = 0:
///
/// version_format: <field><field>
/// field_format: <field_id><field_format>
///
/// Fields:
///
/// TraceId: (field_id = 0, len = 16, default = "0000000000000000") - 16-byte array representing the trace_id.
/// SpanId: (field_id = 1, len = 8, default = "00000000") - 8-byte array representing the span_id.
/// TraceOptions: (field_id = 2, len = 1, default = "0") - 1-byte array representing the trace_options.
///
/// Fields MUST be encoded using the field id order (smaller to higher).
///
/// Valid value example:
///
/// {0, 0, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 1, 97,
/// 98, 99, 100, 101, 102, 103, 104, 2, 1}
///
/// version_id = 0;
/// trace_id = {64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79}
/// span_id = {97, 98, 99, 100, 101, 102, 103, 104};
/// trace_options = {1};

/// to_binary returns the binary format representation of a SpanContext.
pub fn to_binary(sc: &SpanContext) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(29, 0);
    buf[2..18].copy_from_slice(&sc.trace_id.0);
    buf[18] = 1;
    buf[19..27].copy_from_slice(&sc.span_id.0);
    buf[27] = 2;
    buf[28] = sc.trace_options.0 as u8;
    buf
}

/// from_binary returns the SpanContext represented by b.
///
/// If b has an unsupported version ID or contains no TraceID, FromBinary
/// returns with None.
pub fn from_binary(buf: &[u8]) -> Option<SpanContext> {
    let mut b = buf;
    if b.is_empty() || b[0] != 0 {
        return None;
    }

    b = &b[1..];
    let trace_id;
    if b.len() >= 17 && b[0] == 0 {
        let mut a: [u8; 16] = Default::default();
        a.copy_from_slice(&b[1..17]);
        trace_id = TraceID(a);
    } else {
        return None;
    }

    b = &b[17..];
    let span_id;
    if b.len() >= 9 && b[0] == 1 {
        let mut a: [u8; 8] = Default::default();
        a.copy_from_slice(&b[1..9]);
        span_id = SpanID(a);
    } else {
        return None;
    }

    b = &b[9..];
    let trace_options;
    if b.len() >= 2 && b[0] == 2 {
        trace_options = TraceOptions(u32::from(b[1]));
    } else {
        return None;
    }

    Some(SpanContext {
        trace_id,
        span_id,
        trace_options,
        trace_state: None,
    })
}

// TODO(john|p=2|#feature|#http): Support Http format, hyper feature flag?
/*
/// HTTPFormat implementations propagate span contexts
/// in HTTP requests.
///
/// SpanContextFromRequest extracts a span context from incoming
/// requests.
///
/// SpanContextToRequest modifies the given request to include the given
/// span context.
trait HTTPFormat<Request> {
    pub fn span_context_from_request(req: &Request) -> (sc: SpanContext, ok: bool)
    pub fn span_context_to_request(sc: SpanContext, req: &mut Request)
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_binary() {
        let trace_id = TraceID([
            0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d,
            0x4e, 0x4f,
        ]);
        let span_id = SpanID([0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);
        let mut b: Vec<u8> = vec![
            0, 0, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 1, 97, 98, 99,
            100, 101, 102, 103, 104, 2, 1,
        ];

        let b2 = to_binary(&SpanContext {
            trace_id,
            span_id,
            trace_options: TraceOptions(1),
            trace_state: None,
        });

        assert_eq!(*b2, *b);

        match from_binary(&mut b.clone()) {
            None => panic!("decode failed"),
            Some(span_context) => {
                assert_eq!(span_context.trace_id, trace_id);
                assert_eq!(span_context.span_id, span_id);
            }
        }

        b[0] = 1;
        if from_binary(&mut b).is_some() {
            panic!("decoded bytes containing unsupported version");
        }

        b = vec![0, 1, 97, 98, 99, 100, 101, 102, 103, 104, 2, 1];
        if from_binary(&mut b).is_some() {
            panic!("decoded bytes without a TraceID");
        }

        // No such thing as an empty struct in Rust so can't replicate Go tests
    }

    #[test]
    fn test_from_binary() {
        let valid_data = [
            0, 0, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 1, 97, 98, 99,
            100, 101, 102, 103, 104, 2, 1,
        ];

        #[derive(Clone)]
        struct TestCase<'a> {
            data: &'a [u8],
            want_trace_id: Option<TraceID>,
            want_span_id: Option<SpanID>,
            want_opts: Option<TraceOptions>,
            want_ok: bool,
        }

        let mut test_cases = [
            TestCase {
                data: &[0, 0, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77],
                want_trace_id: None,
                want_span_id: None,
                want_opts: None,
                want_ok: false,
            },
            TestCase {
                data: &[0, 1, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77],
                want_trace_id: None,
                want_span_id: None,
                want_opts: None,
                want_ok: false,
            },
            TestCase {
                data: &valid_data,
                want_trace_id: Some(TraceID([
                    64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
                ])),
                want_span_id: Some(SpanID([97, 98, 99, 100, 101, 102, 103, 104])),
                want_opts: Some(TraceOptions(1)),
                want_ok: true,
            },
        ];

        for test_case in test_cases.iter_mut() {
            let mut data = test_case.data.to_vec();
            match from_binary(&mut data) {
                None => assert!(!test_case.want_ok, "unexpected error while decoding"),
                Some(span_context) => {
                    if let Some(trace_id) = test_case.want_trace_id {
                        assert_eq!(span_context.trace_id, trace_id);
                    }
                    if let Some(span_id) = test_case.want_span_id {
                        assert_eq!(span_context.span_id, span_id);
                    }
                    if let Some(trace_opts) = test_case.want_opts {
                        assert_eq!(span_context.trace_options, trace_opts);
                    }
                }
            }
        }
    }
}
