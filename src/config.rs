use std::{env, net::IpAddr, num::NonZeroU64};

use anyhow::{bail, Context, Result};

use crate::rate::RateCollection;

#[derive(Clone)]
pub struct Config {
    /// The interval (in milliseconds) between when history, peers, torrents and
    /// users are flushed to the main mysql database.
    pub flush_interval_milliseconds: u64,
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
    /// The upload_factor is multiplied by 0.01 before being multiplied with
    /// the announced uploaded parameter and saved in the "credited" upload
    /// column. An upload_factor of 200 means global double upload.
    pub upload_factor: u8,
    /// The download factor is multiplied by 0.01 before being multiplied
    /// with the announced downloaded parameter and saved in the "credited"
    /// download column. A download_factor of 0 means global freeleech.
    pub download_factor: u8,
    /// Amount of seconds between scheduled batches where peers are marked as
    /// inactive or erased from memory.
    pub peer_expiry_interval: u64,
    /// Amount of seconds since the last announce before a peer is considered
    /// inactive.
    pub active_peer_ttl: u64,
    /// Amount of seconds since the last announce before a peer is erased from
    /// memory. This value should be long enough that users can suffer
    /// multi-day network outages without announcing, otherwise if their setup
    /// comes back online and the peer has been erased, then their new stats
    /// will be recorded incorrectly.
    pub inactive_peer_ttl: u64,
    /// Site password used by UNIT3D to send api requests to the tracker.
    /// Must be at least 32 characters long and should be properly randomized.
    pub apikey: String,
    /// IP address for the tracker to listen from to receive announces.
    pub listening_ip_address: IpAddr,
    /// Port for the tracker to listen from to receive announces.
    pub listening_port: u16,
    /// Max amount of active peers a user is allowed to have on a torrent.
    /// Prevents abuse from malicious users causing the server to run out of ram,
    /// as well as keeps the peer lists from being filled with too many clients
    /// of a single user.
    pub max_peers_per_torrent_per_user: u16,
    /// Open a connection to the incoming peer announcing and record if their
    /// socket accepts the connection.
    pub is_connectivity_check_enabled: bool,
    /// The minimum number of seconds a socket's connectivity status is cached for
    /// before rechecking the peer's connectivity. Use `-1` for no caching.
    pub connectivity_check_interval: i64,
    /// When enabled, restrict peers to those with open ports. Peers with closed
    /// ports will receive empty peer lists and are not included in other returned
    /// peer lists. Requires `IS_CONNECTIVITY_CHECK_ENABLED` to be `true`.
    pub require_peer_connectivity: bool,
    /// Enable logging of all successful announces to the `announces` table for
    /// debugging. This will generate significant amounts of data. Do not
    /// enable if you do not know what you are doing.
    pub is_announce_logging_enabled: bool,
    /// The header provided by the reverse proxy that includes the bittorrent
    /// client's original ip address. The last address in the comma separated
    /// list will be selected. Leave empty to select the connecting ip address
    /// if not using a reverse proxy.
    pub reverse_proxy_client_ip_header_name: Option<String>,
    /// The max amount of peer lists containing seeds a user is allowed to
    /// receive per time window (in seconds). The rate is calculated using an
    /// exponential decay model. If a user requests peer lists faster than
    /// this, then their peer lists will be empty. Mitigates peer scraping
    /// attacks.
    pub user_receive_seed_list_rate_limits: RateCollection,
    /// The max amount of peer lists containing leeches a user is allowed to
    /// receive per time window (in seconds). The rate is calculated using an
    /// exponential decay model. If a user requests peer lists faster than
    /// this, then their peer lists will be empty. Mitigates peer scraping
    /// attacks.
    pub user_receive_leech_list_rate_limits: RateCollection,
    /// If specified, this will override the immune status on the user's group
    /// to the specified value.
    pub donor_immunity_override: Option<bool>,
    /// If specified, this will override the upload factor of the user's group.
    /// The factor is stored as a percentage.
    pub donor_upload_factor_override: Option<u8>,
    /// If specified, this will override the download factor of the user's
    /// group. The factor is stored as a percentage.
    pub donor_download_factor_override: Option<u8>,
    /// If specified, this will override the immune status on the user's group
    /// to the specified value.
    pub lifetime_donor_immunity_override: Option<bool>,
    /// If specified, this will override the upload factor of the user's group.
    /// The factor is stored as a percentage.
    pub lifetime_donor_upload_factor_override: Option<u8>,
    /// If specified, this will override the download factor of the user's
    /// group. The factor is stored as a percentage.
    pub lifetime_donor_download_factor_override: Option<u8>,
}

impl Config {
    pub fn from_env() -> Result<Config> {
        let flush_interval_milliseconds: NonZeroU64 = env::var("FLUSH_INTERVAL_MILLISECONDS")
            .context("FLUSH_INTERVAL_MILLISECONDS not found in .env file.")?
            .parse()
            .context("FLUSH_INTERVAL_MILLISECONDS must be a number between 1 and 2^64 - 1")?;

        let numwant_default = env::var("NUMWANT_DEFAULT")
            .context("NUMWANT_DEFAULT not found in .env file.")?
            .parse()
            .context("NUMWANT_DEFAULT must be a number between 0 and 2^64 - 1")?;

        let numwant_max = env::var("NUMWANT_MAX")
            .context("NUMWANT_MAX not found in .env file.")?
            .parse()
            .context("NUMWANT_MAX must be a number between 0 and 2^64 - 1")?;

        let announce_min = env::var("ANNOUNCE_MIN")
            .context("ANNOUNCE_MIN not found in .env file.")?
            .parse()
            .context("ANNOUNCE_MIN must be a number between 0 and 2^32 - 1")?;

        let announce_max = env::var("ANNOUNCE_MAX")
            .context("ANNOUNCE_MAX not found in .env file.")?
            .parse()
            .context("ANNOUNCE_MAX must be a number between 0 and 2^32 - 1")?;

        let upload_factor = env::var("UPLOAD_FACTOR")
            .context("UPLOAD_FACTOR not found in .env file.")?
            .parse()
            .context("UPLOAD_FACTOR must be a number between 0 and 2^8 - 1")?;

        let download_factor = env::var("DOWNLOAD_FACTOR")
            .context("DOWNLOAD_FACTOR not found in .env file.")?
            .parse()
            .context("DOWNLOAD_FACTOR must be a number between 0 and 2^8 - 1")?;

        let peer_expiry_interval: NonZeroU64 = env::var("PEER_EXPIRY_INTERVAL")
            .context("PEER_EXPIRY_INTERVAL not found in .env file.")?
            .parse()
            .context("PEER_EXPIRY_INTERVAL must be a number between 1 and 2^64 - 1")?;

        let active_peer_ttl = env::var("ACTIVE_PEER_TTL")
            .context("ACTIVE_PEER_TTL not found in .env file.")?
            .parse()
            .context("ACTIVE_PEER_TTL must be a number between 0 and 2^64 - 1")?;

        let inactive_peer_ttl = env::var("INACTIVE_PEER_TTL")
            .context("INACTIVE_PEER_TTL not found in .env file.")?
            .parse()
            .context("INACTIVE_PEER_TTL must be a number between 0 and 2^64 - 1")?;

        let listening_ip_address = env::var("LISTENING_IP_ADDRESS")
            .context("LISTENING_IP_ADDRESS not found in .env file.")?
            .parse()
            .context("LISTENING_IP_ADDRESS in .env file could not be parsed.")?;

        let listening_port = env::var("LISTENING_PORT")
            .context("LISTENING_PORT not found in .env file.")?
            .parse()
            .context("LISTENING_PORT must be a number between 0 and 2^16 - 1")?;

        let max_peers_per_torrent_per_user = env::var("MAX_PEERS_PER_TORRENT_PER_USER")
            .context("MAX_PEERS_PER_TORRENT_PER_USER not found in .env file.")?
            .parse()
            .context("MAX_PEERS_PER_TORRENT_PER_USER must be a number between 0 and 2^16 - 1")?;

        let is_connectivity_check_enabled = env::var("IS_CONNECTIVITY_CHECK_ENABLED")
            .context("IS_CONNECTIVITY_CHECK_ENABLED not found in .env file.")?
            .parse()
            .context("IS_CONNECTIVITY_CHECK_ENABLED must be either `true` or `false`")?;

        let connectivity_check_interval = env::var("CONNECTIVITY_CHECK_INTERVAL")
            .context("CONNECTIVITY_CHECK_INTERVAL not found in .env file.")?
            .parse()
            .context("CONNECTIVITY_CHECK_INTERVAL must be a number between -(2^63) and 2^63 - 1")?;

        let require_peer_connectivity = env::var("REQUIRE_PEER_CONNECTIVITY")
            .context("REQUIRE_PEER_CONNECTIVITY not found in .env file.")?
            .parse()
            .context("REQUIRE_PEER_CONNECTIVITY must be either `true` or `false`")?;

        let is_announce_logging_enabled = env::var("IS_ANNOUNCE_LOGGING_ENABLED")
            .context("IS_ANNOUNCE_LOGGING_ENABLED not found in .env file.")?
            .parse()
            .context("IS_ANNOUNCE_LOGGING_ENABLED must be either `true` or `false`")?;

        let reverse_proxy_client_ip_header_name =
            env::var("REVERSE_PROXY_CLIENT_IP_HEADER_NAME").ok();

        let user_receive_seed_list_rate_limits = RateCollection::new_from_string(
            &env::var("USER_RECEIVE_SEED_LIST_RATE_LIMITS")
                .context("USER_RECEIVE_SEED_LIST_RATE_LIMITS not found in .env file.")?,
        )
        .context("USER_RECEIVE_SEED_LIST_RATE_LIMITS has incorrect format.")?;

        let user_receive_leech_list_rate_limits = RateCollection::new_from_string(
            &env::var("USER_RECEIVE_LEECH_LIST_RATE_LIMITS")
                .context("USER_RECEIVE_LEECH_LIST_RATE_LIMITS not found in .env file.")?,
        )
        .context("USER_RECEIVE_LEECH_LIST_RATE_LIMITS has incorrect format.")?;

        let donor_immunity_override = env::var("DONOR_IMMUNITY_OVERRIDE")
            .ok()
            .map(|s| s.parse())
            .transpose()
            .context("DONOR_IMMUNITY_OVERRIDE must be either `true` or `false`, if provided")?;

        let donor_upload_factor_override = env::var("DONOR_UPLOAD_FACTOR_OVERRIDE")
            .ok()
            .map(|s| s.parse())
            .transpose()
            .context(
                "DONOR_UPLOAD_FACTOR_OVERRIDE must be a number between 0 and 2^8 - 1, if provided",
            )?;

        let donor_download_factor_override = env::var("DONOR_DOWNLOAD_FACTOR_OVERRIDE")
            .ok()
            .map(|s| s.parse())
            .transpose()
            .context(
                "DONOR_DOWNLOAD_FACTOR_OVERRIDE must be a number between 0 and 2^8 - 1, if provided",
            )?;

        let lifetime_donor_immunity_override = env::var("LIFETIME_DONOR_IMMUNITY_OVERRIDE")
            .ok()
            .map(|s| s.parse())
            .transpose()
            .context(
                "LIFETIME_DONOR_IMMUNITY_OVERRIDE must be either `true` or `false`, if provided",
            )?;

        let lifetime_donor_upload_factor_override = env::var(
                "LIFETIME_DONOR_UPLOAD_FACTOR_OVERRIDE",
            )
            .ok()
            .map(|s| s.parse())
            .transpose()
            .context(
                "LIFETIME_DONOR_UPLOAD_FACTOR_OVERRIDE must be a number between 0 and 2^8 - 1, if provided",
            )?;

        let lifetime_donor_download_factor_override = env::var(
            "LIFETIME_DONOR_DOWNLOAD_FACTOR_OVERRIDE",
        )
        .ok()
        .map(|s| s.parse())
        .transpose()
        .context(
            "LIFETIME_DONOR_DOWNLOAD_FACTOR_OVERRIDE must be a number between 0 and 2^8 - 1, if provided",
        )?;

        let apikey = env::var("APIKEY").context("APIKEY not found in .env file.")?;

        if apikey.len() < 32 {
            bail!("APIKEY must be at least 32 characters long");
        }

        Ok(Config {
            flush_interval_milliseconds: flush_interval_milliseconds.into(),
            numwant_default,
            numwant_max,
            announce_min,
            announce_max,
            upload_factor,
            download_factor,
            peer_expiry_interval: peer_expiry_interval.into(),
            active_peer_ttl,
            inactive_peer_ttl,
            apikey,
            listening_ip_address,
            listening_port,
            max_peers_per_torrent_per_user,
            is_connectivity_check_enabled,
            connectivity_check_interval,
            require_peer_connectivity,
            is_announce_logging_enabled,
            reverse_proxy_client_ip_header_name,
            user_receive_seed_list_rate_limits,
            user_receive_leech_list_rate_limits,
            donor_immunity_override,
            donor_upload_factor_override,
            donor_download_factor_override,
            lifetime_donor_immunity_override,
            lifetime_donor_upload_factor_override,
            lifetime_donor_download_factor_override,
        })
    }
}
