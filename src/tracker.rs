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

use crate::config;
use crate::scheduler::unregistered_info_hash_update::{self, UnregisteredInfoHashUpdate};
use crate::scheduler::{
    Queue, QueueConfig, announce_update,
    history_update::{self, HistoryUpdate},
    peer_update::{self, PeerUpdate},
    torrent_update::{self, TorrentUpdate},
    user_update::{self, UserUpdate},
};
use crate::stats::Stats;

use dotenvy::dotenv;
use parking_lot::{Mutex, RwLock};
use sqlx::Connection;
use sqlx::mysql::MySqlPoolOptions;
use std::io::{self, Write};
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub agent_blacklist: RwLock<blacklisted_agent::Set>,
    pub announce_updates: Mutex<announce_update::Queue>,
    pub config: RwLock<config::Config>,
    pub connectable_ports: RwLock<connectable_port::Map>,
    pub featured_torrents: RwLock<featured_torrent::Set>,
    pub freeleech_tokens: RwLock<freeleech_token::Set>,
    pub groups: RwLock<group::Map>,
    pub history_updates: Mutex<Queue<history_update::Index, HistoryUpdate>>,
    pub infohash2id: RwLock<torrent::infohash2id::Map>,
    pub passkey2id: RwLock<user::passkey2id::Map>,
    pub peer_updates: Mutex<Queue<peer_update::Index, PeerUpdate>>,
    pub personal_freeleeches: RwLock<personal_freeleech::Set>,
    pub pool: MySqlPool,
    pub port_blacklist: RwLock<blacklisted_port::Set>,
    pub stats: Stats,
    pub torrents: Mutex<torrent::Map>,
    pub torrent_updates: Mutex<Queue<torrent_update::Index, TorrentUpdate>>,
    pub unregistered_info_hash_updates:
        Mutex<Queue<unregistered_info_hash_update::Index, UnregisteredInfoHashUpdate>>,
    pub users: RwLock<user::Map>,
    pub user_updates: Mutex<Queue<user_update::Index, UserUpdate>>,
}

impl Tracker {
    /// Creates a database connection pool, and loads all relevant tracker
    /// data into this shared tracker context. This is then passed to all
    /// handlers.
    pub async fn default() -> Result<Arc<Tracker>> {
        print!(".env file: verifying file exists                       ... ");
        io::stdout().flush().unwrap();
        let env_path = dotenv().context(".env file not found.")?;
        println!("[Finished] Path: {:?}", env_path);

        print!("Loading config from env                                ... ");
        io::stdout().flush().unwrap();
        let config = config::Config::from_env()?;
        println!("[Finished]");

        print!("Connecting to database                                 ... ");
        io::stdout().flush().unwrap();
        let pool = connect_to_database().await;
        println!("[Finished]");

        print!("Synchronizing peer counts                              ... ");
        io::stdout().flush().unwrap();
        sync_peer_count_aggregates(&pool, &config).await?;
        println!("[Finished]");

        println!("Loading entities from database into memory...");
        print!("Starting to load  1/11: blacklisted ports              ... ");
        io::stdout().flush().unwrap();
        let port_blacklist = blacklisted_port::Set::default();
        println!("[Finished] Records: {:?}", port_blacklist.len());

        print!("Starting to load  2/11: blacklisted user agents        ... ");
        io::stdout().flush().unwrap();
        let agent_blacklist = blacklisted_agent::Set::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", agent_blacklist.len());

        print!("Starting to load  3/11: torrents                       ... ");
        io::stdout().flush().unwrap();
        let torrents = torrent::Map::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", torrents.len());

        print!("Starting to load  4/11: infohash to torrent id mappings... ");
        io::stdout().flush().unwrap();
        let infohash2id = torrent::infohash2id::Map::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", infohash2id.len());

        print!("Starting to load  5/11: users                          ... ");
        io::stdout().flush().unwrap();
        let users = user::Map::from_db(&pool, &config).await?;
        println!("[Finished] Records: {:?}", users.len());

        print!("Starting to load  6/11: passkey to user id mappings    ... ");
        io::stdout().flush().unwrap();
        let passkey2id = user::passkey2id::Map::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", passkey2id.len());

        print!("Starting to load  7/11: connectable ports              ... ");
        io::stdout().flush().unwrap();
        let connectable_ports = connectable_port::Map::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", connectable_ports.len());

        print!("Starting to load  8/11: freeleech tokens               ... ");
        io::stdout().flush().unwrap();
        let freeleech_tokens = freeleech_token::Set::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", freeleech_tokens.len());

        print!("Starting to load  9/11: personal freeleeches           ... ");
        io::stdout().flush().unwrap();
        let personal_freeleeches = personal_freeleech::Set::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", personal_freeleeches.len());

        print!("Starting to load 10/11: featured torrents              ... ");
        io::stdout().flush().unwrap();
        let featured_torrents = featured_torrent::Set::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", featured_torrents.len());

        print!("Starting to load 11/11: groups                         ... ");
        io::stdout().flush().unwrap();
        let groups = group::Map::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", groups.len());

        println!("All entities loaded into memory.");

        let stats = Stats::default();

        Ok(Arc::new(Tracker {
            agent_blacklist: RwLock::new(agent_blacklist),
            announce_updates: Mutex::new(announce_update::Queue::new()),
            config: RwLock::new(config),
            connectable_ports: RwLock::new(connectable_ports),
            freeleech_tokens: RwLock::new(freeleech_tokens),
            featured_torrents: RwLock::new(featured_torrents),
            groups: RwLock::new(groups),
            history_updates: Mutex::new(Queue::<history_update::Index, HistoryUpdate>::new(
                QueueConfig {
                    max_bindings_per_flush: 65_535,
                    bindings_per_record: 16,
                    // 1 extra binding is used to insert the TTL
                    extra_bindings_per_flush: 1,
                },
            )),
            infohash2id: RwLock::new(infohash2id),
            passkey2id: RwLock::new(passkey2id),
            peer_updates: Mutex::new(Queue::<peer_update::Index, PeerUpdate>::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 15,
                extra_bindings_per_flush: 0,
            })),
            personal_freeleeches: RwLock::new(personal_freeleeches),
            pool,
            port_blacklist: RwLock::new(port_blacklist),
            stats,
            torrents: Mutex::new(torrents),
            torrent_updates: Mutex::new(Queue::<torrent_update::Index, TorrentUpdate>::new(
                QueueConfig {
                    max_bindings_per_flush: 65_535,
                    bindings_per_record: 15,
                    extra_bindings_per_flush: 0,
                },
            )),
            unregistered_info_hash_updates: Mutex::new(Queue::<
                unregistered_info_hash_update::Index,
                UnregisteredInfoHashUpdate,
            >::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 4,
                extra_bindings_per_flush: 0,
            })),
            users: RwLock::new(users),
            user_updates: Mutex::new(Queue::<user_update::Index, UserUpdate>::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 9,
                extra_bindings_per_flush: 0,
            })),
        }))
    }
}

/// Uses the values in the .env file to create a connection pool to the database
async fn connect_to_database() -> sqlx::Pool<sqlx::MySql> {
    // Get pool of database connections.
    MySqlPoolOptions::new()
        .min_connections(0)
        .max_connections(60)
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
