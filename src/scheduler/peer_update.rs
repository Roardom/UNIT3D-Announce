use std::{
    cmp::min,
    net::IpAddr,
    ops::{Deref, DerefMut},
};

use crate::tracker::peer::PeerId;
use chrono::{DateTime, Utc};
use compact_str::CompactString;
use indexmap::IndexMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub IndexMap<Index, PeerUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
    pub user_id: u32,
    pub peer_id: PeerId,
}

pub struct PeerUpdate {
    pub peer_id: PeerId,
    pub ip: std::net::IpAddr,
    pub port: u16,
    pub agent: CompactString,
    pub uploaded: u64,
    pub downloaded: u64,
    pub is_active: bool,
    pub is_seeder: bool,
    pub left: u64,
    pub torrent_id: u32,
    pub user_id: u32,
    pub updated_at: DateTime<Utc>,
    pub connectable: bool,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(IndexMap::new())
    }

    pub fn upsert(
        &mut self,
        peer_id: PeerId,
        ip: IpAddr,
        port: u16,
        agent: CompactString,
        uploaded: u64,
        downloaded: u64,
        is_active: bool,
        is_seeder: bool,
        left: u64,
        torrent_id: u32,
        user_id: u32,
        connectable: bool,
    ) {
        self.insert(
            Index {
                torrent_id,
                user_id,
                peer_id,
            },
            PeerUpdate {
                peer_id,
                ip,
                port,
                agent,
                uploaded,
                downloaded,
                is_active,
                is_seeder,
                left,
                torrent_id,
                user_id,
                updated_at: Utc::now(),
                connectable,
            },
        );
    }

    /// Determine the max amount of peer records that can be inserted at
    /// once
    const fn peer_limit() -> usize {
        /// Max amount of bindings in a mysql query
        const BIND_LIMIT: usize = 65535;

        /// Number of columns being updated in the peer table
        const PEER_COLUMN_COUNT: usize = 13;

        BIND_LIMIT / PEER_COLUMN_COUNT
    }

    /// Take a portion of the peer updates small enough to be inserted into
    /// the database.
    pub fn take_batch(&mut self) -> Queue {
        let len = self.len();

        Queue(self.split_off(len - min(Queue::peer_limit(), len)))
    }

    /// Merge a peer update batch into this peer update batch
    pub fn upsert_batch(&mut self, batch: Queue) {
        for peer_update in batch.values() {
            self.upsert(
                peer_update.peer_id,
                peer_update.ip,
                peer_update.port,
                peer_update.agent.to_owned(),
                peer_update.uploaded,
                peer_update.downloaded,
                peer_update.is_active,
                peer_update.is_seeder,
                peer_update.left,
                peer_update.torrent_id,
                peer_update.user_id,
                peer_update.connectable,
            );
        }
    }

    /// Flushes peer updates to the mysql db
    pub async fn flush_to_db(&self, db: &MySqlPool) -> Result<u64, sqlx::Error> {
        let len = self.len();

        if len == 0 {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                INSERT INTO
                    peers(
                        peer_id,
                        ip,
                        port,
                        agent,
                        uploaded,
                        downloaded,
                        `left`,
                        active,
                        seeder,
                        created_at,
                        updated_at,
                        torrent_id,
                        user_id,
                        connectable
                    )
            "#,
        );

        query_builder
            .push_values(self.values(), |mut bind, peer_update| {
                match peer_update.ip {
                    IpAddr::V4(ip) => bind
                        .push_bind(peer_update.peer_id.to_vec())
                        .push_bind(ip.octets().to_vec())
                        .push_bind(peer_update.port)
                        .push_bind(peer_update.agent.as_str())
                        .push_bind(peer_update.uploaded)
                        .push_bind(peer_update.downloaded)
                        .push_bind(peer_update.left)
                        .push_bind(peer_update.is_active)
                        .push_bind(peer_update.is_seeder)
                        .push_bind(peer_update.updated_at)
                        .push_bind(peer_update.updated_at)
                        .push_bind(peer_update.torrent_id)
                        .push_bind(peer_update.user_id)
                        .push_bind(peer_update.connectable),
                    IpAddr::V6(ip) => bind
                        .push_bind(peer_update.peer_id.to_vec())
                        .push_bind(ip.octets().to_vec())
                        .push_bind(peer_update.port)
                        .push_bind(peer_update.agent.as_str())
                        .push_bind(peer_update.uploaded)
                        .push_bind(peer_update.downloaded)
                        .push_bind(peer_update.left)
                        .push_bind(peer_update.is_active)
                        .push_bind(peer_update.is_seeder)
                        .push_bind(peer_update.updated_at)
                        .push_bind(peer_update.updated_at)
                        .push_bind(peer_update.torrent_id)
                        .push_bind(peer_update.user_id)
                        .push_bind(peer_update.connectable),
                };
            })
            .push(
                r#"
                ON DUPLICATE KEY UPDATE
                    ip = VALUES(ip),
                    port = VALUES(port),
                    agent = VALUES(agent),
                    uploaded = VALUES(uploaded),
                    downloaded = VALUES(downloaded),
                    `left` = VALUES(`left`),
                    active = VALUES(active),
                    seeder = VALUES(seeder),
                    updated_at = VALUES(updated_at),
                    connectable = VALUES(connectable)
            "#,
            );

        query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected())
    }
}

impl Deref for Queue {
    type Target = IndexMap<Index, PeerUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
