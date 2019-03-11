use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;

use crate::basetypes::{SpanID, TraceID};

pub trait IDGenerator {
    fn new_trace_id(&self) -> TraceID;
    fn new_span_id(&self) -> SpanID;
}

pub fn default_id_generator() -> Arc<dyn IDGenerator + Send + Sync> {
    lazy_static! {
        pub static ref DEFAULT_ID_GENERATOR: Arc<dyn IDGenerator + Send + Sync> =
            Arc::new(DefaultIDGenerator::new());
    }
    Arc::clone(&DEFAULT_ID_GENERATOR)
}

pub struct DefaultIDGenerator {
    source: Mutex<Xoshiro256Plus>,
}

impl DefaultIDGenerator {
    fn new() -> Self {
        DefaultIDGenerator {
            source: Mutex::new(Xoshiro256Plus::seed_from_u64(0)),
        }
    }
}

impl IDGenerator for DefaultIDGenerator {
    fn new_trace_id(&self) -> TraceID {
        let mut trace_id: [u8; 16] = [0; 16];
        let mut source = self.source.lock().unwrap();
        (*source).fill_bytes(&mut trace_id[..]);
        TraceID(trace_id)
    }

    fn new_span_id(&self) -> SpanID {
        let mut span_id: [u8; 8] = [0; 8];
        let mut source = self.source.lock().unwrap();
        (*source).fill_bytes(&mut span_id[..]);
        SpanID(span_id)
    }
}
