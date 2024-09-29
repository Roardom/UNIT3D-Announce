use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

/// Used to efficiently calculate rates of a recurring event.
///
/// Each time the rate is ticked, its new rate is calculated using the
/// following formula:
///
/// ```
/// new_rate = old_rate * e^(-1 * (current_time - last_event_time) / window) + 1
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Rate {
    /// The effective current rate of requests in the given time window based
    /// on exponential decay.
    count: f64,
    /// The maximum rate of requests in the given time window that are allowed.
    max_count: f64,
    /// The duration of the time window for rate limiting in seconds.
    window: f64,
    /// The timestamp of the last request processed.
    updated_at: f64,
}

impl Rate {
    /// Initializes a new rate.
    pub fn new(window: Duration, max_events_per_window: f64) -> Self {
        Self {
            count: 0f64,
            max_count: max_events_per_window,
            window: window.as_secs_f64(),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Negative unix epoch.")
                .as_secs_f64(),
        }
    }

    /// Updates the current rate.
    pub fn tick(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Negative unix epoch.")
            .as_secs_f64();
        let elapsed = now - self.updated_at;

        self.count = self.count * f64::exp(-1f64 * elapsed / self.window) + 1f64;

        self.updated_at = now;
    }

    // Current rate is under the max rate.
    pub fn is_under_limit(&self) -> bool {
        self.count <= self.max_count
    }

    // Current rate is above the max rate.
    pub fn is_over_limit(&self) -> bool {
        !self.is_under_limit()
    }

    /// Computes the current rate per second.
    pub fn per_second(&self) -> f64 {
        self.count / self.window
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RateCollection {
    rates: Vec<Rate>,
}

impl RateCollection {
    /// Create a new RateCollection.
    pub fn new(rates: &[Rate]) -> Self {
        Self {
            rates: rates.into(),
        }
    }

    /// Create a rate limit collection from a string of the form
    /// `window1=max_amount1;window2=max_amount2`.
    pub fn new_from_string(s: &str) -> anyhow::Result<Self> {
        Ok(Self {
            rates: s
                .split(';')
                .map(|pair| {
                    let mut split = pair.splitn(2, '=');
                    let window: f64 = split
                        .next()
                        .context("Failed to parse rate window.")?
                        .trim()
                        .parse()?;
                    let max_events_per_window: f64 = split
                        .next()
                        .context("Failed to parse rate max events per window.")?
                        .trim()
                        .parse()?;

                    Ok(Rate::new(
                        Duration::from_secs_f64(window),
                        max_events_per_window,
                    ))
                })
                .collect::<Result<Vec<Rate>, anyhow::Error>>()
                .context("Failed to parse rate limit string.")?
                .into(),
        })
    }

    /// Updates the current rate.
    pub fn tick(&mut self) {
        for rate in &mut self.rates {
            rate.tick();
        }
    }

    // All rates are under the max rate.
    pub fn is_under_limit(&self) -> bool {
        self.rates.iter().all(|rate| rate.is_under_limit())
    }

    // At least one rate is over the max rate.
    pub fn is_over_limit(&self) -> bool {
        !self.is_under_limit()
    }
}
