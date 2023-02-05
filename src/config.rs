use crate::error::Error;
use std::env;

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
    /// Site password used by UNIT3D to send api requests to the tracker.
    pub apikey: String,
}

impl Config {
    pub fn from_env() -> Result<Config, Error> {
        let flush_interval: u64 = env::var("FLUSH_INTERVAL")
            .map_err(|_| Error("FLUSH_INTERVAL not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("FLUSH_INTERVAL must be a number between 0 and 2^64 - 1"))?;

        let numwant_default = env::var("NUMWANT_DEFAULT")
            .map_err(|_| Error("NUMWANT_DEFAULT not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("NUMWANT_DEFAULT must be a number between 0 and 2^64 - 1"))?;

        let numwant_max = env::var("NUMWANT_MAX")
            .map_err(|_| Error("NUMWANT_MAX not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("NUMWANT_MAX must be a number between 0 and 2^64 - 1"))?;

        let announce_min = env::var("ANNOUNCE_MIN")
            .map_err(|_| Error("ANNOUNCE_MIN not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("ANNOUNCE_MIN must be a number between 0 and 2^32 - 1"))?;

        let announce_max = env::var("ANNOUNCE_MAX")
            .map_err(|_| Error("ANNOUNCE_MAX not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("ANNOUNCE_MAX must be a number between 0 and 2^32 - 1"))?;

        let upload_factor = env::var("UPLOAD_FACTOR")
            .map_err(|_| Error("UPLOAD_FACTOR not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("UPLOAD_FACTOR must be a number between 0 and 255"))?;

        let download_factor = env::var("DOWNLOAD_FACTOR")
            .map_err(|_| Error("DOWNLOAD_FACTOR not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("DOWNLOAD_FACTOR must be a number between 0 and 255"))?;

        let peer_expiry_interval = env::var("PEER_EXPIRY_INTERVAL")
            .map_err(|_| Error("PEER_EXPIRY_INTERVAL not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("PEER_EXPIRY_INTERVAL must be a number between 0 and 2^64 - 1"))?;

        let active_peer_ttl = env::var("ACTIVE_PEER_TTL")
            .map_err(|_| Error("ACTIVE_PEER_TTL not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("ACTIVE_PEER_TTL must be a number between 0 and 2^64 - 1"))?;

        let inactive_peer_ttl = env::var("INACTIVE_PEER_TTL")
            .map_err(|_| Error("INACTIVE_PEER_TTL not found in .env file. Aborting."))?
            .parse()
            .map_err(|_| Error("INACTIVE_PEER_TTL must be a number between 0 and 2^64 - 1"))?;

        let apikey =
            env::var("APIKEY").map_err(|_| Error("APIKEY not found in .env file. Aborting."))?;

        if apikey.len() < 32 {
            return Err(Error(
                "APIKEY must be at least 32 characters long. Aborting.",
            ));
        }

        Ok(Config {
            flush_interval,
            numwant_default,
            numwant_max,
            announce_min,
            announce_max,
            upload_factor,
            download_factor,
            peer_expiry_interval,
            active_peer_ttl,
            inactive_peer_ttl,
            apikey,
        })
    }
}
