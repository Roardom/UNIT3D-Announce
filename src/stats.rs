use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Json,
};

use crate::tracker::Tracker;

pub struct Stats {
    created_at: AtomicF64,
    last_request_at: AtomicF64,
    last_announce_response_at: AtomicF64,
    requests_per_1s: AtomicF64,
    requests_per_10s: AtomicF64,
    requests_per_60s: AtomicF64,
    requests_per_900s: AtomicF64,
    requests_per_7200s: AtomicF64,
    announce_responses_per_1s: AtomicF64,
    announce_responses_per_10s: AtomicF64,
    announce_responses_per_60s: AtomicF64,
    announce_responses_per_900s: AtomicF64,
    announce_responses_per_7200s: AtomicF64,
}

impl Default for Stats {
    /// Initializes the stats struct.
    fn default() -> Stats {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time must go forwards.")
            .as_secs_f64();

        Stats {
            created_at: AtomicF64::new(now),
            last_request_at: AtomicF64::new(now),
            last_announce_response_at: AtomicF64::new(now),
            requests_per_1s: Default::default(),
            requests_per_10s: Default::default(),
            requests_per_60s: Default::default(),
            requests_per_900s: Default::default(),
            requests_per_7200s: Default::default(),
            announce_responses_per_1s: Default::default(),
            announce_responses_per_10s: Default::default(),
            announce_responses_per_60s: Default::default(),
            announce_responses_per_900s: Default::default(),
            announce_responses_per_7200s: Default::default(),
        }
    }
}

impl Stats {
    pub fn increment_request(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH);

        if let Ok(now) = now {
            let elapsed = now.as_secs_f64() - self.last_request_at.load(Ordering::Relaxed);
            self.requests_per_1s
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |rate| {
                    rate * f64::exp(-1f64 * elapsed / 1f64) + 1f64
                });
            self.requests_per_10s
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |rate| {
                    rate * f64::exp(-1f64 * elapsed / 10f64) + 1f64
                });
            self.requests_per_60s
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |rate| {
                    rate * f64::exp(-1f64 * elapsed / 60f64) + 1f64
                });
            self.requests_per_900s
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |rate| {
                    rate * f64::exp(-1f64 * elapsed / 900f64) + 1f64
                });
            self.requests_per_7200s
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |rate| {
                    rate * f64::exp(-1f64 * elapsed / 7200f64) + 1f64
                });
            self.last_request_at
                .store(now.as_secs_f64(), Ordering::Relaxed);
        }
    }

    pub fn increment_announce_response(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH);

        if let Ok(now) = now {
            let elapsed =
                now.as_secs_f64() - self.last_announce_response_at.load(Ordering::Relaxed);
            self.announce_responses_per_1s.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |rate| rate * f64::exp(-1f64 * elapsed / 1f64) + 1f64,
            );
            self.announce_responses_per_10s.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |rate| rate * f64::exp(-1f64 * elapsed / 10f64) + 1f64,
            );
            self.announce_responses_per_60s.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |rate| rate * f64::exp(-1f64 * elapsed / 60f64) + 1f64,
            );
            self.announce_responses_per_900s.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |rate| rate * f64::exp(-1f64 * elapsed / 900f64) + 1f64,
            );
            self.announce_responses_per_7200s.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |rate| rate * f64::exp(-1f64 * elapsed / 7200f64) + 1f64,
            );
            self.last_announce_response_at
                .store(now.as_secs_f64(), Ordering::Relaxed);
        }
    }
}

pub async fn show(State(tracker): State<Arc<Tracker>>) -> Json<APIGetStats> {
    Json(APIGetStats {
        created_at: tracker.stats.created_at.load(Ordering::Relaxed),
        last_request_at: tracker.stats.last_request_at.load(Ordering::Relaxed),
        last_announce_response_at: tracker
            .stats
            .last_announce_response_at
            .load(Ordering::Relaxed),
        requests_per_1s: tracker.stats.requests_per_1s.load(Ordering::Relaxed),
        requests_per_10s: tracker.stats.requests_per_10s.load(Ordering::Relaxed) / 10f64,
        requests_per_60s: tracker.stats.requests_per_60s.load(Ordering::Relaxed) / 60f64,
        requests_per_900s: tracker.stats.requests_per_900s.load(Ordering::Relaxed) / 900f64,
        requests_per_7200s: tracker.stats.requests_per_7200s.load(Ordering::Relaxed) / 7200f64,
        announce_responses_per_1s: tracker
            .stats
            .announce_responses_per_1s
            .load(Ordering::Relaxed),
        announce_responses_per_10s: tracker
            .stats
            .announce_responses_per_10s
            .load(Ordering::Relaxed)
            / 10f64,
        announce_responses_per_60s: tracker
            .stats
            .announce_responses_per_60s
            .load(Ordering::Relaxed)
            / 60f64,
        announce_responses_per_900s: tracker
            .stats
            .announce_responses_per_900s
            .load(Ordering::Relaxed)
            / 900f64,
        announce_responses_per_7200s: tracker
            .stats
            .announce_responses_per_7200s
            .load(Ordering::Relaxed)
            / 7200f64,
    })
}

#[derive(serde::Serialize)]
pub struct APIGetStats {
    created_at: f64,
    last_request_at: f64,
    last_announce_response_at: f64,
    requests_per_1s: f64,
    requests_per_10s: f64,
    requests_per_60s: f64,
    requests_per_900s: f64,
    requests_per_7200s: f64,
    announce_responses_per_1s: f64,
    announce_responses_per_10s: f64,
    announce_responses_per_60s: f64,
    announce_responses_per_900s: f64,
    announce_responses_per_7200s: f64,
}

pub async fn record_request(
    State(state): State<Arc<Tracker>>,
    request: Request,
    next: Next,
) -> Response {
    state.stats.increment_request();

    next.run(request).await
}

struct AtomicF64 {
    content: AtomicU64,
}

impl Default for AtomicF64 {
    fn default() -> Self {
        Self {
            content: AtomicU64::new(0f64.to_bits()),
        }
    }
}

impl AtomicF64 {
    fn new(value: f64) -> Self {
        Self {
            content: AtomicU64::new(value.to_bits()),
        }
    }

    fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.content.load(order))
    }

    fn store(&self, value: f64, order: Ordering) {
        self.content.store(value.to_bits(), order)
    }

    fn fetch_update<F>(&self, set_order: Ordering, fetch_order: Ordering, mut f: F)
    where
        F: FnMut(f64) -> f64,
    {
        let _ = self.content.fetch_update(set_order, fetch_order, |num| {
            Some(f(f64::from_bits(num)).to_bits())
        });
    }
}
