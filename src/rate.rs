use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use tokio::time::Duration;

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Negative unix epoch.")
        .as_secs_f64()
}

/// Used to efficiently calculate rates of a recurring event.
///
/// The rate is stored as an exponentially decaying count:
///
/// ```text
/// current_count = stored_count * e^(-1 * (current_time - updated_at) / window)
/// ```
///
/// Decay is applied whenever the rate is read, so a rate that has gone over
/// its limit recovers even if no further events are recorded. `tick` records a
/// new event, but only while the rate is under its limit: events that are
/// already being limited are not counted, otherwise heavy clients would keep
/// the counter pinned forever.
#[derive(Clone, Copy, Debug, Deserialize)]
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
        Self::new_at(window, max_events_per_window, now())
    }

    fn new_at(window: Duration, max_events_per_window: f64, updated_at: f64) -> Self {
        Self {
            count: 0f64,
            max_count: max_events_per_window,
            window: window.as_secs_f64(),
            updated_at,
        }
    }

    fn decayed_count_at(&self, now: f64) -> f64 {
        let elapsed = (now - self.updated_at).max(0f64);

        self.count * (-elapsed / self.window).exp()
    }

    fn is_under_limit_at(&self, now: f64) -> bool {
        self.decayed_count_at(now) <= self.max_count
    }

    /// Returns the number of seconds until this rate drops back under its
    /// limit. Returns `0.0` when the rate is currently under the limit.
    fn retry_after_at(&self, now: f64) -> f64 {
        let count = self.decayed_count_at(now);

        if count <= self.max_count {
            0f64
        } else if self.max_count <= 0f64 {
            f64::INFINITY
        } else {
            self.window * (count / self.max_count).ln()
        }
    }

    /// Returns the number of seconds until this rate drops back under its
    /// limit. Returns `0.0` when the rate is currently under the limit.
    pub fn retry_after(&self) -> f64 {
        self.retry_after_at(now())
    }

    /// The duration of the time window for this rate in seconds.
    pub fn window_secs(&self) -> f64 {
        self.window
    }

    fn tick_at(&mut self, now: f64) {
        let count = self.decayed_count_at(now);

        self.count = if count <= self.max_count {
            count + 1f64
        } else {
            count
        };

        self.updated_at = now;
    }
}

impl Serialize for Rate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let now = now();
        let count = self.decayed_count_at(now);

        let mut state = serializer.serialize_struct("Rate", 6)?;
        state.serialize_field("count", &count)?;
        state.serialize_field("max_count", &self.max_count)?;
        state.serialize_field("window", &self.window)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("retry_after", &self.retry_after_at(now))?;
        state.serialize_field("is_limited", &(count > self.max_count))?;
        state.end()
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
    ///
    /// An empty string creates an empty collection, which disables the limit.
    pub fn new_from_string(s: &str) -> anyhow::Result<Self> {
        let rates = s
            .split(';')
            .filter(|&s| !s.is_empty())
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

                if !window.is_finite() || window <= 0f64 {
                    anyhow::bail!("Rate window must be a positive number.");
                }

                if !max_events_per_window.is_finite() || max_events_per_window <= 0f64 {
                    anyhow::bail!("Rate max events per window must be a positive number.");
                }

                Ok(Rate::new(
                    Duration::from_secs_f64(window),
                    max_events_per_window,
                ))
            })
            .collect::<Result<Vec<Rate>, anyhow::Error>>()
            .context("Failed to parse rate limit string.")?;

        Ok(Self::new(&rates))
    }

    /// Updates the current rate.
    pub fn tick(&mut self) {
        self.tick_at(now());
    }

    fn tick_at(&mut self, now: f64) {
        for rate in &mut self.rates {
            rate.tick_at(now);
        }
    }

    fn is_under_limit_at(&self, now: f64) -> bool {
        self.rates.iter().all(|rate| rate.is_under_limit_at(now))
    }

    // All rates are under the max rate.
    pub fn is_under_limit(&self) -> bool {
        self.is_under_limit_at(now())
    }

    /// Returns the rate with the longest recovery time among the rates that
    /// are currently over their limit.
    pub fn most_restrictive_over_limit(&self) -> Option<Rate> {
        self.most_restrictive_over_limit_at(now())
    }

    fn most_restrictive_over_limit_at(&self, now: f64) -> Option<Rate> {
        self.rates
            .iter()
            .filter(|rate| !rate.is_under_limit_at(now))
            .max_by(|a, b| a.retry_after_at(now).total_cmp(&b.retry_after_at(now)))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(window: f64, max_count: f64) -> Rate {
        Rate::new_at(Duration::from_secs_f64(window), max_count, 0f64)
    }

    #[test]
    fn decay_is_applied_when_reading() {
        let mut rate = rate(60f64, 10f64);

        for _ in 0..11 {
            rate.tick_at(0f64);
        }

        assert!(!rate.is_under_limit_at(0f64));
        assert!(rate.retry_after_at(0f64) > 0f64);
        assert!(rate.is_under_limit_at(60f64));
        assert_eq!(rate.retry_after_at(60f64), 0f64);
    }

    #[test]
    fn events_are_not_counted_while_limited() {
        let mut rate = rate(60f64, 10f64);

        for _ in 0..11 {
            rate.tick_at(0f64);
        }

        let count_before = rate.decayed_count_at(0f64);

        rate.tick_at(0f64);

        assert_eq!(rate.decayed_count_at(0f64), count_before);
    }

    #[test]
    fn retry_after_matches_decay() {
        let mut rate = rate(60f64, 10f64);

        for _ in 0..20 {
            rate.tick_at(0f64);
        }

        let retry_after = rate.retry_after_at(0f64);

        assert!((retry_after - 60f64 * (11f64 / 10f64).ln()).abs() < 1e-9);
        assert!(rate.is_under_limit_at(retry_after));
    }

    #[test]
    fn rejects_non_positive_limits() {
        assert!(RateCollection::new_from_string("60=0").is_err());
        assert!(RateCollection::new_from_string("0=10").is_err());
        assert!(RateCollection::new_from_string("-60=10").is_err());
    }

    #[test]
    fn empty_collection_is_unlimited() {
        let collection = RateCollection::new(&[]);

        assert!(collection.is_under_limit_at(0f64));
        assert!(collection.most_restrictive_over_limit_at(0f64).is_none());
    }

    #[test]
    fn most_restrictive_over_limit_is_reported() {
        let mut collection = RateCollection::new(&[
            Rate::new_at(Duration::from_secs(60), 1f64, 0f64),
            Rate::new_at(Duration::from_secs(3600), 1f64, 0f64),
        ]);

        collection.tick_at(0f64);
        collection.tick_at(0f64);

        assert_eq!(
            collection
                .most_restrictive_over_limit_at(0f64)
                .map(|rate| rate.window_secs()),
            Some(3600f64)
        );
    }
}
