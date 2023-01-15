#[derive(Clone)]
pub struct Config {
    /// The interval (in seconds) between when history, peers, torrents and
    /// users are flushed to the main mysql database.
    pub flush_interval: u64,
    /// The amount of peers that should be sent back if the peer does not
    /// include a numwant.
    pub numwant_default: usize,
    /// The max amount of peers that should be sent back if the peer's numwant
    /// is too high.
    pub numwant_max: usize,
    /// A random amount of seconds between announce_min and announce_max will
    /// be returned to the peer for the next time they should announce.
    pub announce_min: u32,
    /// A random amount of seconds between announce_min and announce_max will
    /// be returned to the peer for the next time they should announce.
    pub announce_max: u32,
    /// The upload_factor is multiplied by 0.01 before being
    /// multiplied with the announced uploaded parameter
    /// and saved in the "credited" upload column.
    /// A upload_factor of 200 means global double upload.
    pub upload_factor: u8,
    /// The download factor is multiplied by 0.01 before being
    /// multiplied with the announced downloaded parameter
    /// and saved in the "credited" download column.
    /// A download_factor of 0 means global freeleech.
    pub download_factor: u8,
    /// Amount of seconds between scheduled batches where peers are marked as
    /// inactive or erased from memory.
    pub peer_expiry_interval: u64,
    /// Amount of seconds since the last announce before a peer is considered
    /// inactive
    pub active_peer_ttl: u64,
    /// Amount of seconds since the last announce before a peer is erased from
    /// memory. This value should be long enough that users can suffer
    /// multi-day network outages without announcing, otherwise if their setup
    /// comes back online and the peer has been erased, then their new stats
    /// will be recorded incorrectly.
    pub inactive_peer_ttl: u64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            flush_interval: 3,            // 3 seconds
            numwant_default: 25,          // 25 peers
            numwant_max: 50,              // 50 peers
            announce_min: 3_600,          // 60 minutes
            announce_max: 5_400,          // 90 minutes
            upload_factor: 100,           // 1x factor
            download_factor: 100,         // 1x factor
            peer_expiry_interval: 1800,   // 30 minutes
            active_peer_ttl: 7200,        // 2 hours
            inactive_peer_ttl: 1_814_400, // 3 weeks
        }
    }
}
