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

use sqlx::{MySql, MySqlPool, QueryBuilder};

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::config;
use crate::scheduler::announce_update::AnnounceUpdate;
use crate::scheduler::unregistered_info_hash_update::{self, UnregisteredInfoHashUpdate};
use crate::scheduler::{
    announce_update,
    history_update::{self, HistoryUpdate},
    peer_update::{self, PeerUpdate},
    torrent_update::{self, TorrentUpdate},
    user_update::{self, UserUpdate},
    Queue, QueueConfig,
};
use crate::stats::Stats;

use dotenvy::dotenv;
use parking_lot::{Mutex, RwLock};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Connection;
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub agent_blacklist: RwLock<blacklisted_agent::Set>,
    pub announce_update_tx: mpsc::UnboundedSender<AnnounceUpdate>,
    pub config: RwLock<config::Config>,
    pub connectable_ports: RwLock<connectable_port::Map>,
    pub featured_torrents: RwLock<featured_torrent::Set>,
    pub freeleech_tokens: RwLock<freeleech_token::Set>,
    pub groups: RwLock<group::Map>,
    pub history_update_tx: mpsc::UnboundedSender<HistoryUpdate>,
    pub infohash2id: RwLock<torrent::infohash2id::Map>,
    pub passkey2id: RwLock<user::passkey2id::Map>,
    pub peer_update_tx: mpsc::UnboundedSender<PeerUpdate>,
    pub personal_freeleeches: RwLock<personal_freeleech::Set>,
    pub pool: MySqlPool,
    pub port_blacklist: RwLock<blacklisted_port::Set>,
    pub stats: Stats,
    pub torrents: Mutex<torrent::Map>,
    pub torrent_update_tx: mpsc::UnboundedSender<TorrentUpdate>,
    pub unregistered_info_hash_update_tx: mpsc::UnboundedSender<UnregisteredInfoHashUpdate>,
    pub users: RwLock<user::Map>,
    pub user_update_tx: mpsc::UnboundedSender<UserUpdate>,
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

        println!("Synchronizing peer counts...");
        sync_peer_count_aggregates(&pool, &config).await?;
        println!("\x1B[1F\x1B[2KSynchronized peer counts");

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
        let users = user::Map::from_db(&pool, &config).await?;
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

        let (announce_update_tx, announce_update_rx) = mpsc::unbounded_channel::<AnnounceUpdate>();
        let (history_update_tx, history_update_rx) = mpsc::unbounded_channel::<HistoryUpdate>();
        let (peer_update_tx, peer_update_rx) = mpsc::unbounded_channel::<PeerUpdate>();
        let (torrent_update_tx, torrent_update_rx) = mpsc::unbounded_channel::<TorrentUpdate>();
        let (unregistered_info_hash_update_tx, unregistered_info_hash_update_rx) =
            mpsc::unbounded_channel::<UnregisteredInfoHashUpdate>();
        let (user_update_tx, user_update_rx) = mpsc::unbounded_channel::<UserUpdate>();

        let announce_updates = announce_update::Queue::new();
        let history_updates = Queue::<history_update::Index, HistoryUpdate>::new(
            QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 16,
                // 1 extra binding is used to insert the TTL
                extra_bindings_per_flush: 1,
            },
            history_update_rx,
        );

        let peer_updates = Queue::<peer_update::Index, PeerUpdate>::new(
            QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 15,
                extra_bindings_per_flush: 0,
            },
            peer_update_rx,
        );

        let torrent_updates = Queue::<torrent_update::Index, TorrentUpdate>::new(
            QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 15,
                extra_bindings_per_flush: 0,
            },
            torrent_update_rx,
        );

        let unregistered_info_hash_updates =
            Queue::<unregistered_info_hash_update::Index, UnregisteredInfoHashUpdate>::new(
                QueueConfig {
                    max_bindings_per_flush: 65_535,
                    bindings_per_record: 4,
                    extra_bindings_per_flush: 0,
                },
                unregistered_info_hash_update_rx,
            );

        let user_updates = Queue::<user_update::Index, UserUpdate>::new(
            QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 9,
                extra_bindings_per_flush: 0,
            },
            user_update_rx,
        );

        Ok(Arc::new(Tracker {
            agent_blacklist: RwLock::new(agent_blacklist),
            announce_update_tx,
            config: RwLock::new(config),
            connectable_ports: RwLock::new(connectable_ports),
            freeleech_tokens: RwLock::new(freeleech_tokens),
            featured_torrents: RwLock::new(featured_torrents),
            groups: RwLock::new(groups),
            history_update_tx,
            infohash2id: RwLock::new(infohash2id),
            passkey2id: RwLock::new(passkey2id),
            peer_update_tx,
            personal_freeleeches: RwLock::new(personal_freeleeches),
            pool,
            port_blacklist: RwLock::new(port_blacklist),
            stats,
            torrents: Mutex::new(torrents),
            torrent_update_tx,
            unregistered_info_hash_update_tx,
            users: RwLock::new(users),
            user_update_tx,
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

async fn sync_peer_count_aggregates(db: &MySqlPool, config: &config::Config) -> anyhow::Result<()> {
    let mut query: QueryBuilder<MySql> = QueryBuilder::new(
        r#"
        UPDATE torrents
            LEFT JOIN (
                SELECT
                    torrent_id,
                    SUM(peers.left = 0) AS updated_seeders,
                    SUM(peers.left > 0) AS updated_leechers
                FROM peers
                WHERE peers.active
                    AND peers.visible
        "#,
    );

    if config.require_peer_connectivity {
        query.push(" AND peers.connectable ");
    }

    query
        .push(
            r#"
                    GROUP BY torrent_id
                ) AS seeders_leechers
                    ON torrents.id = seeders_leechers.torrent_id
            SET
                torrents.seeders = COALESCE(seeders_leechers.updated_seeders, 0),
                torrents.leechers = COALESCE(seeders_leechers.updated_leechers, 0)
            WHERE
                torrents.deleted_at IS NULL
            "#,
        )
        .build()
        .execute(db)
        .await
        .context("Failed synchronizing peer count aggregates to the database.")?;

    Ok(())
}
