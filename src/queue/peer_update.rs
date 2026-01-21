use std::{net::IpAddr, sync::Arc};

use crate::{state::AppState, store::peer::PeerId};
use chrono::{DateTime, Utc};
use sqlx::{MySql, QueryBuilder};

use super::{Flushable, Mergeable};

// Fields must be in same order as database primary key
#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Index {
    pub user_id: u32,
    pub torrent_id: u32,
    pub peer_id: PeerId,
}

#[derive(Clone)]
pub struct PeerUpdate {
    pub ip: std::net::IpAddr,
    pub port: u16,
    pub agent: String,
    pub uploaded: u64,
    pub downloaded: u64,
    pub is_active: bool,
    pub is_seeder: bool,
    pub is_visible: bool,
    pub left: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub connectable: bool,
}

impl Mergeable for PeerUpdate {
    fn merge(&mut self, new: &Self) {
        if new.updated_at > self.updated_at {
            self.ip = new.ip;
            self.port = new.port;
            self.agent = new.agent.clone();
            self.uploaded = new.uploaded;
            self.downloaded = new.downloaded;
            self.is_active = new.is_active;
            self.is_seeder = new.is_seeder;
            self.is_visible = new.is_visible;
            self.left = new.left;
            self.updated_at = new.updated_at;
            self.connectable = new.connectable;
        }

        self.created_at = std::cmp::min(self.created_at, new.created_at);
    }
}

impl Flushable<PeerUpdate> for super::Batch<Index, PeerUpdate> {
    async fn flush_to_db(&self, state: &Arc<AppState>) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
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
                        visible,
                        created_at,
                        updated_at,
                        torrent_id,
                        user_id,
                        connectable
                    )
            "#,
        );

        query_builder
            // Trailing space required before the push values function
            // Leading space required after the push values function
            .push_values(self.iter(), |mut bind, (index, peer_update)| {
                bind.push_bind(index.peer_id.to_vec())
                    .push_bind(match peer_update.ip {
                        IpAddr::V4(ip) => ip.octets().to_vec(),
                        IpAddr::V6(ip) => ip.octets().to_vec(),
                    })
                    .push_bind(peer_update.port)
                    .push_bind(peer_update.agent.as_str())
                    .push_bind(peer_update.uploaded)
                    .push_bind(peer_update.downloaded)
                    .push_bind(peer_update.left)
                    .push_bind(peer_update.is_active)
                    .push_bind(peer_update.is_seeder)
                    .push_bind(peer_update.is_visible)
                    .push_bind(peer_update.created_at)
                    .push_bind(peer_update.updated_at)
                    .push_bind(index.torrent_id)
                    .push_bind(index.user_id)
                    .push_bind(peer_update.connectable);
            })
            // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
            // However, Mariadb doesn't yet support aliases
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
                    visible = VALUES(visible),
                    updated_at = VALUES(updated_at),
                    connectable = VALUES(connectable)
            "#,
            );

        query_builder
            .build()
            .persistent(false)
            .execute(&state.pool)
            .await
            .map(|result| result.rows_affected())
    }
}
