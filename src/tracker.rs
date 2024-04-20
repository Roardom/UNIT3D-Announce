pub mod blacklisted_agent;
pub mod blacklisted_port;
pub mod connectable_port;
pub mod featured_torrent;
pub mod freeleech_token;
pub mod group;
pub mod peer;
pub mod personal_freeleech;
pub mod torrent;
pub mod user;

pub use peer::Peer;

use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::config;
use crate::scheduler::{history_update, peer_update, torrent_update, user_update};
use crate::stats::Stats;

use dotenvy::dotenv;
use parking_lot::{Mutex, RwLock};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Connection;
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub agent_blacklist: RwLock<blacklisted_agent::Set>,
    pub config: config::Config,
    pub connectable_ports: RwLock<connectable_port::Map>,
    pub featured_torrents: RwLock<featured_torrent::Set>,
    pub freeleech_tokens: RwLock<freeleech_token::Set>,
    pub groups: RwLock<group::Map>,
    pub history_updates: Mutex<history_update::Queue>,
    pub infohash2id: RwLock<torrent::infohash2id::Map>,
    pub passkey2id: RwLock<user::passkey2id::Map>,
    pub peer_updates: Mutex<peer_update::Queue>,
    pub personal_freeleeches: RwLock<personal_freeleech::Set>,
    pub pool: MySqlPool,
    pub port_blacklist: RwLock<blacklisted_port::Set>,
    pub stats: Stats,
    pub torrents: Mutex<torrent::Map>,
    pub torrent_updates: Mutex<torrent_update::Queue>,
    pub users: RwLock<user::Map>,
    pub user_updates: Mutex<user_update::Queue>,
}

impl Tracker {
    /// Creates a database connection pool, and loads all relevant tracker
    /// data into this shared tracker context. This is then passed to all
    /// handlers.
    pub async fn default() -> Result<Arc<Tracker>> {
        println!(".env file: verifying file exists...");
        dotenv().context(".env file not found.")?;
        println!("\x1B[1F\x1B[2KFound .env file");

        println!("Loading from database into memory: config...");
        let config = config::Config::from_env()?;
        println!("\x1B[1F\x1B[2KLoaded config parameters");

        println!("Connecting to database...");
        let pool = connect_to_database().await;
        println!("\x1B[1F\x1B[2KConnected to database");

        println!("Loading from database into memory: blacklisted ports...");
        let port_blacklist = blacklisted_port::Set::default();
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} blacklisted ports",
            port_blacklist.len()
        );

        println!("Loading from database into memory: blacklisted user agents...");
        let agent_blacklist = blacklisted_agent::Set::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} blacklisted agents",
            agent_blacklist.len()
        );

        println!("Loading from database into memory: torrents...");
        let torrents = torrent::Map::from_db(&pool).await?;
        println!("\x1B[1F\x1B[2KLoaded {:?} torrents", torrents.len());

        println!("Loading from database into memory: infohash to torrent id mapping...");
        let infohash2id = torrent::infohash2id::Map::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} torrent infohash to id mappings",
            infohash2id.len()
        );

        println!("Loading from database into memory: users...");
        let users = user::Map::from_db(&pool).await?;
        println!("\x1B[1F\x1B[2KLoaded {:?} users", users.len());

        println!("Loading from database into memory: passkey to user id mapping...");
        let passkey2id = user::passkey2id::Map::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} user passkey to id mappings",
            passkey2id.len()
        );

        println!("Loading from database into memory: connectable ports...");
        let connectable_ports = connectable_port::Map::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} connectable ports",
            connectable_ports.len()
        );

        println!("Loading from database into memory: freeleech tokens...");
        let freeleech_tokens = freeleech_token::Set::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} freeleech tokens",
            freeleech_tokens.len()
        );

        println!("Loading from database into memory: personal freeleeches...");
        let personal_freeleeches = personal_freeleech::Set::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} personal freeleeches",
            personal_freeleeches.len()
        );

        println!("Loading from database into memory: featured_torrents...");
        let featured_torrents = featured_torrent::Set::from_db(&pool).await?;
        println!(
            "\x1B[1F\x1B[2KLoaded {:?} featured torrents",
            featured_torrents.len()
        );

        println!("Loading from database into memory: groups...");
        let groups = group::Map::from_db(&pool).await?;
        println!("\x1B[1F\x1B[2KLoaded {:?} groups", groups.len());

        let stats = Stats::default();

        Ok(Arc::new(Tracker {
            agent_blacklist: RwLock::new(agent_blacklist),
            config,
            connectable_ports: RwLock::new(connectable_ports),
            freeleech_tokens: RwLock::new(freeleech_tokens),
            featured_torrents: RwLock::new(featured_torrents),
            groups: RwLock::new(groups),
            history_updates: Mutex::new(history_update::Queue::new()),
            infohash2id: RwLock::new(infohash2id),
            passkey2id: RwLock::new(passkey2id),
            peer_updates: Mutex::new(peer_update::Queue::new()),
            personal_freeleeches: RwLock::new(personal_freeleeches),
            pool,
            port_blacklist: RwLock::new(port_blacklist),
            stats,
            torrents: Mutex::new(torrents),
            torrent_updates: Mutex::new(torrent_update::Queue::new()),
            users: RwLock::new(users),
            user_updates: Mutex::new(user_update::Queue::new()),
        }))
    }
}

/// Uses the values in the .env file to create a connection pool to the database
async fn connect_to_database() -> sqlx::Pool<sqlx::MySql> {
    // Get pool of database connections.
    MySqlPoolOptions::new()
        .min_connections(0)
        .max_connections(10)
        .max_lifetime(Duration::from_secs(30 * 60))
        .idle_timeout(Duration::from_secs(10 * 60))
        .acquire_timeout(Duration::from_secs(30))
        .before_acquire(|conn, _meta| Box::pin(async move {
            // MySQL will never shrink its buffers and MySQL will eventually crash
            // from running out of memory if we don't do this.
            conn.shrink_buffers();
            Ok(true)
        }))
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL not found in .env file. Aborting."))
        .await
        .expect("Could not connect to the database using the DATABASE_URL value in .env file. Aborting.")
}
