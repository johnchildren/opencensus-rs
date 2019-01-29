use std::sync::Arc;

use byteorder::{BigEndian, ByteOrder};
use lazy_static::lazy_static;

use crate::basetypes::{SpanID, TraceID};
use crate::trace::SpanContext;

const DEFAULT_SAMPLING_PROBABILITY: f64 = 1e-4;

lazy_static! {
    pub static ref DEFAULT_SAMPLER: Sampler = probability_sampler(DEFAULT_SAMPLING_PROBABILITY);
}

/// Sampler decides whether a trace should be sampled and exported.
pub type Sampler = Arc<dyn Fn(SamplingParameters<'_>) -> SamplingDecision + Send + Sync>;

/// SamplingParameters contains the values passed to a Sampler.
pub struct SamplingParameters<'a> {
    pub parent_context: Option<&'a SpanContext>,
    pub trace_id: &'a TraceID,
    pub span_id: &'a SpanID,
    pub name: &'a str,
    pub has_remote_parent: bool,
}

/// SamplingParameters contains the values passed to a Sampler.
pub struct SamplingDecision {
    pub sample: bool,
}

/// probability_sampler returns a Sampler that samples a given fraction of traces.
///
/// It also samples spans whose parents are sampled.
pub fn probability_sampler(mut fraction: f64) -> Sampler {
    if fraction.is_sign_negative() {
        fraction = 0.0;
    } else if fraction >= 1.0 {
        return always_sample();
    }

    let trace_id_upper_bound = (fraction * ((1 as u64) << 63) as f64).floor() as u64;
    Arc::new(move |sampling_params: SamplingParameters<'_>| {
        if let Some(parent_context) = sampling_params.parent_context {
            if parent_context.is_sampled() {
                return SamplingDecision { sample: true };
            }
        }
        let x = BigEndian::read_u64(&sampling_params.trace_id.0[0..8]) >> 1;
        SamplingDecision {
            sample: x < trace_id_upper_bound,
        }
    })
}

/// always_sample returns a Sampler that samples every trace.
/// Be careful about using this sampler in a production application with
/// significant traffic: a new trace will be started and exported for every
/// request.
pub fn always_sample() -> Sampler {
    Arc::new(|_sampling_params| SamplingDecision { sample: true })
}

/// never_sample returns a Sampler that samples no traces.
pub fn never_sample() -> Sampler {
    Arc::new(|_sampling_params| SamplingDecision { sample: false })
}
