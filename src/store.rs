pub mod blacklisted_agent;
pub mod blacklisted_port;
pub mod connectable_port;
pub mod featured_torrent;
pub mod freeleech_token;
pub mod group;
pub mod infohash2id;
pub mod passkey2id;
pub mod peer;
pub mod personal_freeleech;
pub mod torrent;
pub mod user;

use sqlx::{MySql, MySqlPool, QueryBuilder};

use anyhow::{Context, Result};

use crate::{
    config::{self, Config},
    store::{
        blacklisted_agent::BlacklistedAgentStore, blacklisted_port::BlacklistedPortStore,
        connectable_port::ConnectablePortStore, featured_torrent::FeaturedTorrentStore,
        freeleech_token::FreeleechTokenStore, group::GroupStore, infohash2id::InfoHash2IdStore,
        passkey2id::Passkey2IdStore, personal_freeleech::PersonalFreeleechStore,
        torrent::TorrentStore, user::UserStore,
    },
};

use parking_lot::{Mutex, RwLock};
use std::io::{self, Write};

pub struct Stores {
    pub agent_blacklist: RwLock<BlacklistedAgentStore>,
    pub connectable_ports: RwLock<ConnectablePortStore>,
    pub featured_torrents: RwLock<FeaturedTorrentStore>,
    pub freeleech_tokens: RwLock<FreeleechTokenStore>,
    pub groups: RwLock<GroupStore>,
    pub infohash2id: RwLock<InfoHash2IdStore>,
    pub passkey2id: RwLock<Passkey2IdStore>,
    pub personal_freeleeches: RwLock<PersonalFreeleechStore>,
    pub port_blacklist: RwLock<BlacklistedPortStore>,
    pub torrents: Mutex<TorrentStore>,
    pub users: RwLock<UserStore>,
}

impl Stores {
    /// Load all in-memory stores from the database.
    pub async fn new(pool: &MySqlPool, config: &Config) -> Result<Stores> {
        print!("Synchronizing peer counts                              ... ");
        io::stdout().flush().unwrap();
        sync_peer_count_aggregates(&pool, &config).await?;
        println!("[Finished]");

        println!("Loading entities from database into memory...");
        print!("Starting to load  1/11: blacklisted ports              ... ");
        io::stdout().flush().unwrap();
        let port_blacklist = BlacklistedPortStore::default();
        println!("[Finished] Records: {:?}", port_blacklist.len());

        print!("Starting to load  2/11: blacklisted user agents        ... ");
        io::stdout().flush().unwrap();
        let agent_blacklist = BlacklistedAgentStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", agent_blacklist.len());

        print!("Starting to load  3/11: torrents                       ... ");
        io::stdout().flush().unwrap();
        let torrents = TorrentStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", torrents.len());

        print!("Starting to load  4/11: infohash to torrent id mappings... ");
        io::stdout().flush().unwrap();
        let infohash2id = InfoHash2IdStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", infohash2id.len());

        print!("Starting to load  5/11: users                          ... ");
        io::stdout().flush().unwrap();
        let users = UserStore::from_db(&pool, &config).await?;
        println!("[Finished] Records: {:?}", users.len());

        print!("Starting to load  6/11: passkey to user id mappings    ... ");
        io::stdout().flush().unwrap();
        let passkey2id = Passkey2IdStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", passkey2id.len());

        print!("Starting to load  7/11: connectable ports              ... ");
        io::stdout().flush().unwrap();
        let connectable_ports = ConnectablePortStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", connectable_ports.len());

        print!("Starting to load  8/11: freeleech tokens               ... ");
        io::stdout().flush().unwrap();
        let freeleech_tokens = FreeleechTokenStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", freeleech_tokens.len());

        print!("Starting to load  9/11: personal freeleeches           ... ");
        io::stdout().flush().unwrap();
        let personal_freeleeches = PersonalFreeleechStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", personal_freeleeches.len());

        print!("Starting to load 10/11: featured torrents              ... ");
        io::stdout().flush().unwrap();
        let featured_torrents = FeaturedTorrentStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", featured_torrents.len());

        print!("Starting to load 11/11: groups                         ... ");
        io::stdout().flush().unwrap();
        let groups = GroupStore::from_db(&pool).await?;
        println!("[Finished] Records: {:?}", groups.len());

        println!("All entities loaded into memory.");

        Ok(Stores {
            agent_blacklist: RwLock::new(agent_blacklist),
            connectable_ports: RwLock::new(connectable_ports),
            freeleech_tokens: RwLock::new(freeleech_tokens),
            featured_torrents: RwLock::new(featured_torrents),
            groups: RwLock::new(groups),
            infohash2id: RwLock::new(infohash2id),
            passkey2id: RwLock::new(passkey2id),
            personal_freeleeches: RwLock::new(personal_freeleeches),
            port_blacklist: RwLock::new(port_blacklist),
            torrents: Mutex::new(torrents),
            users: RwLock::new(users),
        })
    }
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
