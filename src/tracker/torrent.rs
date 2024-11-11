use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use diesel::deserialize::Queryable;
use diesel::dsl::sql;
use diesel::sql_types::{Bool, Integer, TinyInt, Unsigned};
use diesel::Selectable;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::tracker::peer;

use super::Db;

use anyhow::{Context, Result};

pub mod infohash;
pub use infohash::InfoHash;

pub mod infohash2id;

pub mod status;
pub use status::Status;

use crate::tracker::Tracker;

pub struct Map(IndexMap<u32, Torrent>);

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &Db) -> Result<Map> {
        use crate::schema::torrents;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let peers = peer::Map::from_db(db).await?;

        // First, group the peers by their torrent id.

        struct GroupedPeer {
            peers: peer::Map,
        }

        let mut grouped_peers: IndexMap<u32, GroupedPeer> = IndexMap::new();

        peers.iter().for_each(|(index, peer)| {
            grouped_peers
                .entry(peer.torrent_id)
                .and_modify(|torrent| {
                    torrent.peers.insert(*index, *peer);
                })
                .or_insert_with(|| {
                    let mut peers = peer::Map::new();
                    peers.insert(*index, *peer);

                    GroupedPeer { peers }
                });
        });

        // TODO: deleted_at column still needs added to unit3d. Until then, no
        // torrents are considered deleted.
        let torrents_data = torrents::table
            .select((
                torrents::id,
                torrents::status,
                sql::<Unsigned<Integer>>("GREATEST(0, seeders)"),
                sql::<Unsigned<Integer>>("GREATEST(0, leechers)"),
                sql::<Unsigned<Integer>>("GREATEST(0, times_completed)"),
                sql::<Unsigned<TinyInt>>("100 - LEAST(torrents.free, 100)"),
                sql::<Unsigned<TinyInt>>("IF(doubleup, 200, 100)"),
                sql::<Bool>("0"),
            ))
            .filter(torrents::deleted_at.is_null())
            .load::<DBImportTorrent>(&mut db.get().await?)
            .await
            .context("Failed loading torrents.")?;

        let mut torrent_map = Map::new();

        torrents_data.iter().for_each(|torrent| {
            // Default values if torrent doesn't exist
            let mut peers = peer::Map::new();

            // Overwrite default values if peers exists
            if let Some(peer_group) = grouped_peers.get(&torrent.id) {
                peers.extend(peer_group.peers.iter());
            }

            // Insert torrent with its peers
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
                    peers,
                },
            );
        });

        Ok(torrent_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(torrent): Json<APIInsertTorrent>,
    ) -> StatusCode {
        if let Ok(info_hash) = InfoHash::from_str(&torrent.info_hash) {
            println!("Inserting torrent with id {}.", torrent.id);
            let old_torrent = tracker.torrents.lock().swap_remove(&torrent.id);
            let peers = old_torrent.unwrap_or_default().peers;

            tracker.torrents.lock().insert(
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

            tracker.infohash2id.write().insert(info_hash, torrent.id);

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(torrent): Json<APIRemoveTorrent>,
    ) -> StatusCode {
        let mut torrent_guard = tracker.torrents.lock();

        if let Some(torrent) = torrent_guard.get_mut(&torrent.id) {
            println!("Removing torrent with id {}.", torrent.id);
            torrent.is_deleted = true;

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn show(
        State(tracker): State<Arc<Tracker>>,
        Path(id): Path<u32>,
    ) -> Result<Json<Torrent>, StatusCode> {
        tracker
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

#[derive(Queryable, Clone, Default)]
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
