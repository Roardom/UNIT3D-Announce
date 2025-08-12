use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use futures_util::TryStreamExt;
use indexmap::IndexMap;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Map(IndexMap<SocketAddr, ConnectablePort>);

#[derive(Clone, Copy, Debug)]
pub struct ConnectablePort {
    pub connectable: bool,
    pub updated_at: DateTime<Utc>,
}

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        let mut peers = sqlx::query!(
            r#"
                SELECT
                    INET6_NTOA(peers.ip) as `ip_address: String`,
                    peers.port as `port: u16`,
                    COALESCE(MAX(peers.connectable), 0) as `connectable!: bool`,
                    MAX(peers.updated_at) as `updated_at: DateTime<Utc>`
                FROM
                    peers
                GROUP BY
                    peers.ip, peers.port
            "#
        )
        .fetch(db);

        let mut peer_map = Map::new();

        while let Some(peer) = peers.try_next().await.context("Failed loading peers.")? {
            peer_map.insert(
                SocketAddr::from((
                    IpAddr::from_str(
                        &peer
                            .ip_address
                            .context("INET6_NTOA failed to decode peer ip.")?,
                    )
                    .context("Peer ip failed to decode.")?,
                    peer.port,
                )),
                ConnectablePort {
                    connectable: peer.connectable,
                    updated_at: peer
                        .updated_at
                        .context("Peer with a null updated_at found in database.")?,
                },
            );
        }

        Ok(peer_map)
    }
}

impl Deref for Map {
    type Target = IndexMap<SocketAddr, ConnectablePort>;

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
