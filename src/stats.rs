use sqlx::types::chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Stats {
    start_time: DateTime<Utc>,
    requests: u64,
    request_rate: u64,
    response_rate_announce: u64,
    response_rate_scrape: u64,
    response_rate_error: u64,
    scrapes: u64,
    seeders: u64,
    leechers: u64,
    rx_rate: u64,
    tx_rate: u64,
    bytes_received: u64,
    bytes_sent: u64,
}

impl Default for Stats {
    /// Initializes the stats struct.
    fn default() -> Stats {
        Stats {
            start_time: Utc::now(),
            requests: 0,
            request_rate: 0,
            response_rate_announce: 0,
            response_rate_scrape: 0,
            response_rate_error: 0,
            scrapes: 0,
            seeders: 0,
            leechers: 0,
            rx_rate: 0,
            tx_rate: 0,
            bytes_received: 0,
            bytes_sent: 0,
        }
    }
}
