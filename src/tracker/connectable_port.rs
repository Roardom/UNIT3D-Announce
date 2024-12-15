use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

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
        let peers: Vec<(SocketAddr, ConnectablePort)> = sqlx::query!(
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
        .map(|row| {
            (
                SocketAddr::from((
                    IpAddr::from_str(
                        &row.ip_address
                            .expect("INET6_NTOA failed to decode peer ip."),
                    )
                    .expect("Peer ip failed to decode."),
                    row.port,
                )),
                ConnectablePort {
                    connectable: row.connectable,
                    updated_at: row
                        .updated_at
                        .expect("Peer with a null updated_at found in database."),
                },
            )
        })
        .fetch_all(db)
        .await
        .context("Failed loading peers.")?;

        let mut peer_map = Map::new();

        for (index, peer) in peers {
            peer_map.insert(index, peer);
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
