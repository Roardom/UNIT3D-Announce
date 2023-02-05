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
use crate::scheduler::{history_update, peer_deletion, peer_update, torrent_update, user_update};
use crate::stats::Stats;

use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub agent_blacklist: blacklisted_agent::Set,
    pub config: config::Config,
    pub freeleech_tokens: freeleech_token::Set,
    pub history_updates: history_update::Queue,
    pub peer_deletions: peer_deletion::Queue,
    pub peer_updates: peer_update::Queue,
    pub personal_freeleeches: personal_freeleech::Set,
    pub pool: MySqlPool,
    pub port_blacklist: blacklisted_port::Set,
    pub stats: Stats,
    pub torrents: Arc<torrent::Map>,
    pub torrent_updates: torrent_update::Queue,
    pub users: user::Map,
    pub user_updates: user_update::Queue,
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
        let port_blacklist = blacklisted_port::Set::default();

        println!("Loading from database into memory: blacklisted user agents...");
        let agent_blacklist = blacklisted_agent::Set::from_db(&pool).await?;

        println!("Loading from database into memory: config...");
        let config = config::Config::from_env()?;

        println!("Loading from database into memory: torrents...");
        let torrents = Arc::new(torrent::Map::from_db(&pool).await?);

        println!("Loading from database into memory: users...");
        let users = user::Map::from_db(&pool).await?;

        println!("Loading from database into memory: freeleech tokens...");
        let freeleech_tokens = freeleech_token::Set::from_db(&pool).await?;

        println!("Loading from database into memory: personal freeleeches...");
        let personal_freeleeches = personal_freeleech::Set::from_db(&pool).await?;

        let stats = Stats::default();

        Ok(Arc::new(Tracker {
            agent_blacklist,
            config,
            freeleech_tokens,
            history_updates: history_update::Queue::new(),
            peer_deletions: peer_deletion::Queue::new(),
            peer_updates: peer_update::Queue::new(),
            personal_freeleeches,
            pool,
            port_blacklist,
            stats,
            torrents,
            torrent_updates: torrent_update::Queue::new(),
            users,
            user_updates: user_update::Queue::new(),
        }))
    }
}
