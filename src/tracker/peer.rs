use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use ahash::RandomState;
use chrono::serde::ts_seconds;
use scc::HashMap;
use serde::{Serialize, Serializer};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub mod peer_id;
pub use peer_id::PeerId;

#[derive(Clone, Serialize)]
pub struct Map(HashMap<Index, Peer, RandomState>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
    pub user_id: u32,
    pub peer_id: PeerId,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Peer {
    pub ip_address: std::net::IpAddr,
    pub user_id: u32,
    pub torrent_id: u32,
    pub port: u16,
    pub is_seeder: bool,
    pub is_active: bool,
    #[serde(with = "ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub uploaded: u64,
    pub downloaded: u64,
}

impl Map {
    pub fn new() -> Map {
        Map(HashMap::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        let peers: Vec<(Index, Peer)> = sqlx::query!(
            r#"
                SELECT
                    INET6_NTOA(peers.ip) as `ip_address: String`,
                    peers.user_id as `user_id: u32`,
                    peers.torrent_id as `torrent_id: u32`,
                    peers.port as `port: u16`,
                    peers.seeder as `is_seeder: bool`,
                    peers.active as `is_active: bool`,
                    peers.updated_at as `updated_at: DateTime<Utc>`,
                    peers.uploaded as `uploaded: u64`,
                    peers.downloaded as `downloaded: u64`,
                    peers.peer_id as `peer_id: PeerId`
                FROM
                    peers
            "#
        )
        .map(|row| {
            (
                Index {
                    user_id: row.user_id,
                    peer_id: row.peer_id,
                },
                Peer {
                    ip_address: std::net::IpAddr::from_str(
                        &row.ip_address
                            .expect("INET6_NTOA failed to decode peer ip."),
                    )
                    .expect("Peer ip failed to decode."),
                    user_id: row.user_id,
                    torrent_id: row.torrent_id,
                    port: row.port,
                    is_seeder: row.is_seeder,
                    is_active: row.is_active,
                    updated_at: row
                        .updated_at
                        .expect("Peer with a null updated_at found in database."),
                    uploaded: row.uploaded,
                    downloaded: row.downloaded,
                },
            )
        })
        .fetch_all(db)
        .await
        .context("Failed loading peers.")?;

        let peer_map = Map::new();

        for (index, peer) in peers {
            peer_map.entry(index).or_insert(peer);
        }

        Ok(peer_map)
    }
}

impl Deref for Map {
    type Target = HashMap<Index, Peer, RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Map {
    fn default() -> Self {
        Map::new()
    }
}

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.user_id, self.peer_id)
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
