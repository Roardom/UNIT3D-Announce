use std::{net::IpAddr, sync::Arc};

use crate::{model::peer_id::PeerId, state::AppState};
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
    pub ipv4: Option<IpAddr>,
    pub ipv4_port: Option<u16>,
    pub ipv4_connectable: Option<bool>,
    pub ipv6: Option<IpAddr>,
    pub ipv6_port: Option<u16>,
    pub ipv6_connectable: Option<bool>,
}

impl Mergeable for PeerUpdate {
    fn merge(&mut self, new: &Self) {
        // Merge dual-stack endpoints independently
        if new.ipv4.is_some() {
            self.ipv4 = new.ipv4;
            self.ipv4_port = new.ipv4_port;
            self.ipv4_connectable = new.ipv4_connectable;
        }
        if new.ipv6.is_some() {
            self.ipv6 = new.ipv6;
            self.ipv6_port = new.ipv6_port;
            self.ipv6_connectable = new.ipv6_connectable;
        }

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

fn ip_to_bytes(ip: &IpAddr) -> Vec<u8> {
    match ip {
        IpAddr::V4(ip) => ip.octets().to_vec(),
        IpAddr::V6(ip) => ip.octets().to_vec(),
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
                        connectable,
                        ipv4,
                        ipv4_port,
                        ipv4_connectable,
                        ipv6,
                        ipv6_port,
                        ipv6_connectable
                    )
            "#,
        );

        query_builder
            // Trailing space required before the push values function
            // Leading space required after the push values function
            .push_values(self.iter(), |mut bind, (index, peer_update)| {
                bind.push_bind(index.peer_id.to_vec())
                    .push_bind(ip_to_bytes(&peer_update.ip))
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
                    .push_bind(peer_update.connectable)
                    .push_bind(peer_update.ipv4.as_ref().map(ip_to_bytes))
                    .push_bind(peer_update.ipv4_port)
                    .push_bind(peer_update.ipv4_connectable)
                    .push_bind(peer_update.ipv6.as_ref().map(ip_to_bytes))
                    .push_bind(peer_update.ipv6_port)
                    .push_bind(peer_update.ipv6_connectable);
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
                    connectable = VALUES(connectable),
                    ipv4 = COALESCE(VALUES(ipv4), ipv4),
                    ipv4_port = CASE WHEN VALUES(ipv4) IS NOT NULL THEN VALUES(ipv4_port) ELSE ipv4_port END,
                    ipv4_connectable = CASE WHEN VALUES(ipv4) IS NOT NULL THEN VALUES(ipv4_connectable) ELSE ipv4_connectable END,
                    ipv6 = COALESCE(VALUES(ipv6), ipv6),
                    ipv6_port = CASE WHEN VALUES(ipv6) IS NOT NULL THEN VALUES(ipv6_port) ELSE ipv6_port END,
                    ipv6_connectable = CASE WHEN VALUES(ipv6) IS NOT NULL THEN VALUES(ipv6_connectable) ELSE ipv6_connectable END
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
