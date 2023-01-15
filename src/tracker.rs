pub mod blacklisted_agent;
pub mod blacklisted_port;
pub mod freeleech_token;
pub mod peer;
pub mod personal_freeleech;
pub mod torrent;
pub mod user;

pub use peer::Peer;
pub use torrent::Torrent;
pub use user::User;

use sqlx::MySqlPool;

use crate::config;
use crate::error::Error;
use crate::scheduler::{
    HistoryUpdateBuffer, PeerDeletionBuffer, PeerUpdateBuffer, TorrentUpdateBuffer,
};
use crate::stats::Stats;
use crate::tracker::personal_freeleech::PersonalFreeleechSet;
use crate::tracker::{
    blacklisted_agent::AgentSet, blacklisted_port::PortSet, freeleech_token::FreeleechTokenSet,
    torrent::TorrentMap, user::UserMap,
};

use dotenvy::dotenv;
use regex::Regex;
use sqlx::mysql::MySqlPoolOptions;
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub agent_blacklist: AgentSet,
    pub agent_blacklist_regex: Regex,
    pub config: config::Config,
    pub freeleech_tokens: FreeleechTokenSet,
    pub history_updates: HistoryUpdateBuffer,
    pub peer_deletions: PeerDeletionBuffer,
    pub peer_updates: PeerUpdateBuffer,
    pub personal_freeleeches: PersonalFreeleechSet,
    pub pool: MySqlPool,
    pub port_blacklist: PortSet,
    pub stats: Stats,
    pub torrents: Arc<TorrentMap>,
    pub torrent_updates: TorrentUpdateBuffer,
    pub users: UserMap,
}

impl Tracker {
    /// Creates a database connection pool, and loads all relevant tracker
    /// data into this shared tracker context. This is then passed to all
    /// handlers.
    pub async fn default() -> Result<Arc<Tracker>, Error> {
        println!(".env file: verifying file exists...");
        dotenv().map_err(|_| Error(".env file not found. Aborting."))?;

        println!(".env file: verifying file contents...");
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| Error("DATABASE_URL not found in .env file. Aborting."))?;

        println!("Connecting to database...");
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await
            .map_err(|_| {
                Error(
                "Could not connect to the database located at DATABASE_URL in .env file. Aborting.",
            )
            })?;

        println!("Loading from database into memory: blacklisted ports...");
        let port_blacklist = PortSet::default();

        println!("Loading from database into memory: blacklisted user agents...");
        let agent_blacklist = AgentSet::from_db(&pool).await?;

        println!("Loading from database into memory: config...");
        let config = config::Config::default();

        println!("Loading from database into memory: torrents...");
        let torrents = Arc::new(TorrentMap::from_db(&pool).await?);

        println!("Loading from database into memory: users...");
        let users = UserMap::from_db(&pool).await?;

        println!("Loading from database into memory: freeleech tokens...");
        let freeleech_tokens = FreeleechTokenSet::from_db(&pool).await?;

        println!("Loading from database into memory: personal freeleeches...");
        let personal_freeleeches = PersonalFreeleechSet::from_db(&pool).await?;

        println!("Compiling user agent blacklist regex...");
        let agent_blacklist_regex =
            Regex::new(r"Mozilla|Browser|Chrome|Safari|AppleWebKit|Opera|Links|Lynx|Bot|Unknown")
                .expect("Invalid regex expression.");

        let stats = Stats::default();

        Ok(Arc::new(Tracker {
            agent_blacklist,
            agent_blacklist_regex,
            config,
            freeleech_tokens,
            history_updates: HistoryUpdateBuffer::new(),
            peer_deletions: PeerDeletionBuffer::new(),
            peer_updates: PeerUpdateBuffer::new(),
            personal_freeleeches,
            pool,
            port_blacklist,
            stats,
            torrents,
            torrent_updates: TorrentUpdateBuffer::new(),
            users,
            // user_updates: DashMap::new(),
        }))
    }
}
