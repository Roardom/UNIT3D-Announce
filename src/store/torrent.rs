use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use futures_util::TryStreamExt;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use sqlx::types::chrono::{DateTime, Utc};
use tracing::info;

use anyhow::{Context, Result};

pub mod infohash;
pub use infohash::InfoHash;

pub mod infohash2id;

pub mod status;
pub use status::Status;

use crate::state::AppState;
use crate::store::peer::{self, Index, Peer};
use peer::peer_id::PeerId;

pub struct Map(IndexMap<u32, Torrent>);

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        // Load one torrent per info hash. If multiple are found, prefer
        // undeleted torrents. If multiple are still found, prefer approved
        // torrents. If multiple are still found, prefer the oldest.
        let mut torrents = sqlx::query_as!(
            DBImportTorrent,
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.status as `status: Status`,
                    torrents.seeders as `seeders: u32`,
                    torrents.leechers as `leechers: u32`,
                    torrents.times_completed as `times_completed: u32`,
                    100 - LEAST(torrents.free, 100) as `download_factor: u8`,
                    IF(torrents.doubleup, 200, 100) as `upload_factor: u8`,
                    torrents.deleted_at IS NOT NULL as `is_deleted: bool`
                FROM
                    torrents
                JOIN (
                    SELECT
                        COALESCE(
                            MIN(CASE WHEN deleted_at IS NULL AND status = 1 THEN id END),
                            MIN(CASE WHEN deleted_at IS NULL AND status != 1 THEN id END),
                            MIN(CASE WHEN deleted_at IS NOT NULL THEN id END)
                        ) AS id
                    FROM
                        torrents
                    GROUP BY
                        info_hash
                ) AS distinct_torrents
                    ON distinct_torrents.id = torrents.id
            "#
        )
        .fetch(db);

        let mut torrent_map = Map::new();

        while let Some(torrent) = torrents
            .try_next()
            .await
            .context("Failed loading torrents.")?
        {
            torrent_map.insert(
                torrent.id,
                Torrent {
                    id: torrent.id,
                    status: torrent.status,
                    seeders: torrent.seeders,
                    leechers: torrent.leechers,
                    times_completed: torrent.times_completed,
                    download_factor: torrent.download_factor,
                    upload_factor: torrent.upload_factor,
                    is_deleted: torrent.is_deleted,
                    peers: peer::Map::new(),
                },
            );
        }

        // Load peers into each torrent
        let mut peers = sqlx::query!(
            r#"
                SELECT
                    INET6_NTOA(peers.ip) as `ip_address: IpAddr`,
                    peers.user_id as `user_id: u32`,
                    peers.torrent_id as `torrent_id: u32`,
                    peers.port as `port: u16`,
                    peers.seeder as `is_seeder: bool`,
                    peers.active as `is_active: bool`,
                    peers.visible as `is_visible: bool`,
                    peers.connectable as `is_connectable: bool`,
                    peers.updated_at as `updated_at: DateTime<Utc>`,
                    peers.uploaded as `uploaded: u64`,
                    peers.downloaded as `downloaded: u64`,
                    peers.peer_id as `peer_id: PeerId`
                FROM
                    peers
            "#
        )
        .fetch(db);

        while let Some(peer) = peers.try_next().await.expect("Failed loading peers.") {
            torrent_map.entry(peer.torrent_id).and_modify(|torrent| {
                torrent.peers.insert(
                    Index {
                        user_id: peer.user_id,
                        peer_id: peer.peer_id,
                    },
                    Peer {
                        ip_address: peer
                            .ip_address
                            .expect("INET6_NTOA failed to decode peer ip."),
                        port: peer.port,
                        is_seeder: peer.is_seeder,
                        is_active: peer.is_active,
                        is_visible: peer.is_visible,
                        is_connectable: peer.is_connectable,
                        has_sent_completed: false,
                        updated_at: peer
                            .updated_at
                            .expect("Peer with a null updated_at found in database."),
                        uploaded: peer.uploaded,
                        downloaded: peer.downloaded,
                    },
                );
            });
        }

        Ok(torrent_map)
    }

    pub async fn upsert(
        State(state): State<Arc<AppState>>,
        Json(torrent): Json<APIInsertTorrent>,
    ) -> StatusCode {
        if let Ok(info_hash) = InfoHash::from_str(&torrent.info_hash) {
            info!("Inserting torrent with id {}.", torrent.id);
            let old_torrent = state.stores.torrents.lock().swap_remove(&torrent.id);
            let peers = old_torrent.unwrap_or_default().peers;

            state.stores.torrents.lock().insert(
                torrent.id,
                Torrent {
                    id: torrent.id,
                    status: torrent.status,
                    is_deleted: torrent.is_deleted,
                    seeders: torrent.seeders,
                    leechers: torrent.leechers,
                    times_completed: torrent.times_completed,
                    download_factor: torrent.download_factor,
                    upload_factor: torrent.upload_factor,
                    peers,
                },
            );

            state
                .stores
                .infohash2id
                .write()
                .insert(info_hash, torrent.id);

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn destroy(
        State(state): State<Arc<AppState>>,
        Json(torrent): Json<APIRemoveTorrent>,
    ) -> StatusCode {
        let mut torrent_guard = state.stores.torrents.lock();

        if let Some(torrent) = torrent_guard.get_mut(&torrent.id) {
            info!("Removing torrent with id {}.", torrent.id);
            torrent.is_deleted = true;

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn show(
        State(state): State<Arc<AppState>>,
        Path(id): Path<u32>,
    ) -> Result<Json<Torrent>, StatusCode> {
        state
            .stores
            .torrents
            .lock()
            .get(&id)
            .map(|torrent| Json(torrent.clone()))
            .ok_or(StatusCode::NOT_FOUND)
    }
}

impl Deref for Map {
    type Target = IndexMap<u32, Torrent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Default)]
pub struct DBImportTorrent {
    pub id: u32,
    pub status: Status,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
    pub is_deleted: bool,
}

#[derive(Clone, Default, Serialize)]
pub struct Torrent {
    pub id: u32,
    pub status: Status,
    pub is_deleted: bool,
    pub peers: peer::Map,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize)]
pub struct APIInsertTorrent {
    pub id: u32,
    pub status: Status,
    pub info_hash: String,
    pub is_deleted: bool,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize)]
pub struct APIRemoveTorrent {
    pub id: u32,
}
