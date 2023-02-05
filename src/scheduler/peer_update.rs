use std::{net::IpAddr, ops::Deref};

use crate::tracker::peer::{PeerId, UserAgent};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub DashMap<Index, PeerUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
    pub user_id: u32,
    pub peer_id: PeerId,
}

#[derive(Clone, Copy)]
pub struct PeerUpdate {
    pub peer_id: PeerId,
    pub ip: std::net::IpAddr,
    pub port: u16,
    pub agent: UserAgent,
    pub uploaded: u64,
    pub downloaded: u64,
    pub is_seeder: bool,
    pub left: u64,
    pub torrent_id: u32,
    pub user_id: u32,
    pub updated_at: DateTime<Utc>,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(DashMap::new())
    }

    pub fn upsert(
        &self,
        peer_id: PeerId,
        ip: IpAddr,
        port: u16,
        agent: UserAgent,
        uploaded: u64,
        downloaded: u64,
        is_seeder: bool,
        left: u64,
        torrent_id: u32,
        user_id: u32,
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
                is_seeder,
                left,
                torrent_id,
                user_id,
                updated_at: Utc::now(),
            },
        );
    }

    /// Flushes peer updates to the mysql db
    pub async fn flush_to_db(&self, db: &MySqlPool) {
        if self.len() == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_PEER_COLUMNS: usize = 13;
        const PEER_LIMIT: usize = BIND_LIMIT / NUM_PEER_COLUMNS;

        let mut peer_updates: Vec<_> = vec![];

        for _ in 0..std::cmp::min(PEER_LIMIT, self.len()) {
            let peer_update = *self.iter().next().unwrap();
            self.remove(&Index {
                torrent_id: peer_update.torrent_id,
                user_id: peer_update.user_id,
                peer_id: peer_update.peer_id,
            });
            peer_updates.push(peer_update);
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
                        seeder,
                        created_at,
                        updated_at,
                        torrent_id,
                        user_id
                    )
            "#,
        );

        query_builder
            .push_values(peer_updates.clone(), |mut bind, peer_update| {
                bind.push_bind(peer_update.peer_id.to_vec())
                    .push("INET6_ATON(")
                    .push_bind_unseparated(peer_update.ip.to_string())
                    .push_unseparated(")")
                    .push_bind(peer_update.port)
                    .push_bind(peer_update.agent.to_vec())
                    .push_bind(peer_update.uploaded)
                    .push_bind(peer_update.downloaded)
                    .push_bind(peer_update.left)
                    .push_bind(peer_update.is_seeder)
                    .push_bind(peer_update.updated_at)
                    .push_bind(peer_update.updated_at)
                    .push_bind(peer_update.torrent_id)
                    .push_bind(peer_update.user_id);
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
                    seeder = VALUES(seeder),
                    updated_at = VALUES(updated_at)
            "#,
            );

        let result = query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected());

        match result {
            Ok(_) => (),
            Err(e) => {
                println!("Peer update failed: {}", e);
                peer_updates.into_iter().for_each(|peer_update| {
                    self.upsert(
                        peer_update.peer_id,
                        peer_update.ip,
                        peer_update.port,
                        peer_update.agent,
                        peer_update.uploaded,
                        peer_update.downloaded,
                        peer_update.is_seeder,
                        peer_update.left,
                        peer_update.torrent_id,
                        peer_update.user_id,
                    );
                });
            }
        }
    }
}

impl Deref for Queue {
    type Target = DashMap<Index, PeerUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
