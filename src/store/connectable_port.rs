use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use futures_util::TryStreamExt;
use indexmap::IndexMap;
use sqlx::MySqlPool;
use sqlx::types::chrono::{DateTime, Utc};

use anyhow::{Context, Result};

#[derive(Clone)]
pub struct ConnectablePortStore {
    inner: IndexMap<SocketAddr, ConnectablePort>,
}

#[derive(Clone, Copy, Debug)]
pub struct ConnectablePort {
    pub connectable: bool,
    pub updated_at: DateTime<Utc>,
}

impl ConnectablePortStore {
    pub fn new() -> ConnectablePortStore {
        ConnectablePortStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<ConnectablePortStore> {
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

        let mut peer_map = ConnectablePortStore::new();

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

impl Deref for ConnectablePortStore {
    type Target = IndexMap<SocketAddr, ConnectablePort>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ConnectablePortStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for ConnectablePortStore {
    fn default() -> Self {
        ConnectablePortStore::new()
    }
}
