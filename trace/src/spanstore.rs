use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time;

use lazy_static::lazy_static;

use crate::export::SpanData;
use crate::spanbucket::{latency_bucket, Bucket, DEFAULT_LATENCIES};
use crate::status_codes::StatusCode;
use crate::trace::Span;

const MAX_BUCKET_SIZE: usize = 100_000;
const DEFAULT_BUCKET_SIZE: usize = 10;

lazy_static! {
    static ref SPAN_STORES: RwLock<HashMap<String, Arc<SpanStore<'static>>>> =
        RwLock::new(HashMap::new());
}

/// SpanStore keeps track of spans stored for a particular span name.
///
/// It contains all active spans; a sample of spans for failed requests,
/// categorized by error code; and a sample of spans for successful requests,
/// bucketed by latency.
#[derive(Debug)]
pub struct SpanStore<'a>(Mutex<SpanStoreContents<'a>>);

// TODO(john|p=2|#techdebt): this doesn't seem idiomatic.
#[derive(Debug)]
struct SpanStoreContents<'a> {
    //active: BTreeSet<Span>,
    errors: HashMap<StatusCode, Bucket<'a>>,
    latency: Vec<Bucket<'a>>,
    max_spans_per_error_bucket: usize,
}

impl<'a> SpanStore<'a> {
    pub fn new(name: &str, latency_bucket_size: usize, error_bucket_size: usize) -> Self {
        let latency = (0..=(DEFAULT_LATENCIES.len()))
            .map(|_| Bucket::new(latency_bucket_size))
            .collect();
        let contents = SpanStoreContents {
            //active: BTreeSet::new(),
            errors: HashMap::new(),
            latency,
            max_spans_per_error_bucket: error_bucket_size,
        };
        SpanStore(Mutex::new(contents))
    }

    fn resize(&mut self, latency_bucket_size: usize, error_bucket_size: usize) {
        let mut contents = self.0.lock().unwrap();
        for i in 0..contents.latency.len() {
            contents.latency[i].resize(latency_bucket_size);
        }
        for errors in contents.errors.values_mut() {
            errors.resize(error_bucket_size);
        }
        contents.max_spans_per_error_bucket = error_bucket_size;
    }

    fn add(&mut self, span: Span) {
        let contents = self.0.lock().unwrap();
        // contents.active.insert(Span)
    }

    fn finished(&mut self, span: &Span, sd: &'a SpanData) {
        let end_time = sd.end_time.unwrap_or_else(time::Instant::now);
        let latency = end_time.duration_since(sd.start_time);
        let code = sd
            .status
            .clone()
            .map(|s| s.code)
            .unwrap_or_else(|| StatusCode::Unknown);

        let mut contents = self.0.lock().unwrap();
        // contents.active.remove(span);
        if code == StatusCode::OK {
            contents.latency[latency_bucket(latency)].add(&sd);
        } else if let Some(bucket) = contents.errors.get_mut(&code) {
            bucket.add(&sd);
        } else {
            let mut bucket = Bucket::new(contents.max_spans_per_error_bucket);
            bucket.add(&sd);
            contents.errors.insert(code, bucket);
        }
    }
}

pub fn span_store_for_name(name: &str) -> Option<Arc<SpanStore<'static>>> {
    let stores = SPAN_STORES.read().unwrap();
    let opt = stores.get(name);
    opt.map(Arc::clone)
}

pub fn span_store_for_name_create_if_new(name: &str) -> Arc<SpanStore<'static>> {
    match span_store_for_name(name) {
        Some(store) => store,
        None => {
            let mut stores = SPAN_STORES.write().unwrap();
            let store = Arc::new(SpanStore::new(
                name,
                DEFAULT_BUCKET_SIZE,
                DEFAULT_BUCKET_SIZE,
            ));
            stores.insert(name.to_string(), Arc::clone(&store));
            store
        }
    }
}
/*
pub fn span_store_set_size(name: &str, latency_bucket_size: usize, error_bucket_size: usize) {
    let mut stores = SPAN_STORES.write().unwrap();
    match stores.get_mut(name) {
        Some(store) => store.resize(latency_bucket_size, error_bucket_size),
        None => {
            let store = SpanStore::new(name, latency_bucket_size, error_bucket_size);
            stores.insert(name.to_string(), Arc::new(store));
        }
    }
}
*/
