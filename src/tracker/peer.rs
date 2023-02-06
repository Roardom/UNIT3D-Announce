use std::ops::Deref;
use std::str::FromStr;

use dashmap::DashMap;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use crate::Error;

pub mod peer_id;
pub use peer_id::PeerId;
pub mod user_agent;
pub use user_agent::UserAgent;

pub struct Map(DashMap<Index, Peer>);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Index {
    pub user_id: u32,
    pub peer_id: PeerId,
}

#[derive(Clone, Copy, Debug)]
pub struct Peer {
    pub ip_address: std::net::IpAddr,
    pub user_id: u32,
    pub torrent_id: u32,
    pub port: u16,
    pub is_seeder: bool,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
    pub uploaded: u64,
    pub downloaded: u64,
}

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        // TODO: is_active still isn't handled by unit3d
        let peers: Vec<(Index, Peer)> = sqlx::query!(
            r#"
                SELECT
                    INET6_NTOA(peers.ip) as `ip_address: String`,
                    peers.user_id as `user_id: u32`,
                    peers.torrent_id as `torrent_id: u32`,
                    peers.port as `port: u16`,
                    peers.seeder as `is_seeder: bool`,
                    1 as `is_active: bool`,
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
        .map_err(|_| Error("Failed loading peers."))?;

        let peer_map = Map::new();

        for (index, peer) in peers {
            peer_map.insert(index, peer);
        }

        Ok(peer_map)
    }
}

impl Deref for Map {
    type Target = DashMap<Index, Peer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Map {
    fn default() -> Self {
        Map::new()
    }
}
