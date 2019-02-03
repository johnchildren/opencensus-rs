use std::time::{Duration, Instant};

use crate::export::SpanData;

/// SAMPLE_PERIOD is the minimum time between accepting spans in a single bucket.
pub const SAMPLE_PERIOD: Duration = Duration::from_secs(1);

/// DEFAULT_LATENCIES contains the default latency bucket bounds.
pub const DEFAULT_LATENCIES: [Duration; 8] = [
    Duration::from_micros(10),
    Duration::from_micros(100),
    Duration::from_millis(1),
    Duration::from_millis(10),
    Duration::from_millis(100),
    Duration::from_secs(1),
    Duration::from_secs(10),
    Duration::from_secs(60),
];

/// Bucket is a container for a set of spans for a particular error code or latency range.
#[derive(Debug)]
pub struct Bucket {
    // next time we can accept a span
    next_time: Instant,
    // circular buffer of spans
    buffer: Vec<SpanData>,
    // location next SpanData should be placed in buffer
    next_index: usize,
    // whether the circular buffer has wrapped around
    overflow: bool,
}

impl Bucket {
    pub fn new(buffer_size: usize) -> Self {
        Bucket {
            next_time: Instant::now(),
            buffer: Vec::with_capacity(buffer_size),
            next_index: 0,
            overflow: false,
        }
    }

    pub fn add(&mut self, s: SpanData) {
        if let Some(end_time) = s.end_time {
            if self.buffer.is_empty() {
                return;
            }
            self.next_time = end_time + SAMPLE_PERIOD;
            self.buffer[self.next_index] = s;
            self.next_index += 1;
            if self.next_index == self.buffer.len() {
                self.next_index = 0;
                self.overflow = true;
            }
        }
    }

    fn size(&self) -> usize {
        if self.overflow {
            self.buffer.len()
        } else {
            self.next_index
        }
    }

    fn span(&self, idx: usize) -> SpanData {
        // TODO(john|p=2|#performance): not happy with the clones here
        if self.overflow {
            self.buffer[idx].clone()
        } else if idx < self.buffer.len() - self.next_index {
            self.buffer[self.next_index + idx].clone()
        } else {
            self.buffer[self.next_index + idx - self.buffer.len()].clone()
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        let current_size = self.size();
        if current_size < new_size {
            self.buffer = (0..new_size).map(|i| self.span(i)).collect();
            self.next_index = current_size;
            self.overflow = false;
            return;
        }
        self.buffer = (0..new_size)
            .map(|i| self.span(i + current_size - new_size))
            .collect();
        self.next_index = 0;
        self.overflow = true;
    }
}

pub fn latency_bucket(latency: Duration) -> usize {
    let mut i = 0;
    while i < DEFAULT_LATENCIES.len() && latency >= DEFAULT_LATENCIES[i] {
        i += 1;
    }
    i
}

pub fn latency_bucket_bounds(idx: usize) -> (Duration, Duration) {
    if idx == 0 {
        (Duration::new(0, 0), DEFAULT_LATENCIES[idx])
    } else if idx == DEFAULT_LATENCIES.len() {
        (
            DEFAULT_LATENCIES[idx - 1],
            Duration::from_secs(u64::max_value()),
        )
    } else {
        (DEFAULT_LATENCIES[idx - 1], DEFAULT_LATENCIES[idx])
    }
}
