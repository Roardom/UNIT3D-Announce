use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

use ahash::RandomState;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use scc::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::tracker::peer;

use anyhow::{Context, Result};

pub mod infohash;
pub use infohash::InfoHash;

pub mod infohash2id;
pub use infohash2id::InfoHash2Id;

pub mod status;
pub use status::Status;

use crate::tracker::Tracker;

pub struct Map(HashMap<u32, Torrent, RandomState>);

impl Map {
    pub fn new() -> Map {
        Map(HashMap::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        let peers = peer::Map::from_db(db).await?;

        // First, group the peers by their torrent id.

        struct GroupedPeer {
            peers: peer::Map,
            num_seeders: u32,
            num_leechers: u32,
        }

        let grouped_peers: HashMap<u32, GroupedPeer> = HashMap::new();

        peers.scan(|index, peer| {
            grouped_peers
                .entry(peer.torrent_id)
                .and_modify(|torrent| {
                    torrent.peers.entry(*index).or_insert(*peer);
                    torrent.num_seeders += (peer.is_active && peer.is_seeder) as u32;
                    torrent.num_leechers += (peer.is_active && !peer.is_seeder) as u32;
                })
                .or_insert_with(|| {
                    let peers = peer::Map::new();
                    peers.entry(*index).or_insert(*peer);

                    GroupedPeer {
                        peers,
                        num_seeders: (peer.is_active && peer.is_seeder) as u32,
                        num_leechers: (peer.is_active && !peer.is_seeder) as u32,
                    }
                });
        });

        // TODO: deleted_at column still needs added to unit3d. Until then, no
        // torrents are considered deleted.
        let torrents: Vec<DBImportTorrent> = sqlx::query_as!(
            DBImportTorrent,
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.status as `status: Status`,
                    torrents.times_completed as `times_completed: u32`,
                    LEAST(100 - torrents.free, IF(featured_torrents.torrent_id IS NULL, 100, 0)) as `download_factor: u8`,
                    IF(featured_torrents.torrent_id IS NULL, 100, 200) as `upload_factor: u8`,
                    0 as `is_deleted: bool`
                FROM
                    torrents
                LEFT JOIN
                    featured_torrents
                ON
                    torrents.id = featured_torrents.torrent_id
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading torrents.")?;

        let torrent_map = Map::new();

        torrents.iter().for_each(|torrent| {
            // Default values if torrent doesn't exist
            let peers = peer::Map::new();
            let mut seeders = 0;
            let mut leechers = 0;

            // Overwrite default values if peers exists
            if let Some(peer_group) = grouped_peers.get(&torrent.id) {
                let peer_group = peer_group.get();

                peer_group.peers.scan(|index, peer| {
                    peers.entry(*index).or_insert(*peer);
                });
                seeders = peer_group.num_seeders;
                leechers = peer_group.num_leechers;
            }

            // Insert torrent with its peers
            torrent_map.entry(torrent.id).or_insert(Torrent {
                id: torrent.id,
                status: torrent.status,
                seeders,
                leechers,
                times_completed: torrent.times_completed,
                download_factor: torrent.download_factor,
                upload_factor: torrent.upload_factor,
                is_deleted: torrent.is_deleted,
                peers,
            });
        });

        Ok(torrent_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(insert_torrent): Json<APIInsertTorrent>,
    ) -> StatusCode {
        if let Ok(info_hash) = InfoHash::from_str(&insert_torrent.info_hash) {
            println!("Inserting torrent with id {}.", insert_torrent.id);
            let old_torrent = tracker.torrents.remove(&insert_torrent.id);
            let peers = old_torrent.unwrap_or_default().1.peers;

            tracker
                .torrents
                .entry(insert_torrent.id)
                .and_modify(|torrent| {
                    torrent.id = insert_torrent.id;
                    torrent.status = insert_torrent.status;
                    torrent.is_deleted = insert_torrent.is_deleted;
                    torrent.seeders = insert_torrent.seeders;
                    torrent.leechers = insert_torrent.leechers;
                    torrent.times_completed = insert_torrent.times_completed;
                    torrent.download_factor = insert_torrent.download_factor;
                    torrent.upload_factor = insert_torrent.upload_factor;
                    torrent.peers = peers.clone();
                })
                .or_insert(Torrent {
                    id: insert_torrent.id,
                    status: insert_torrent.status,
                    is_deleted: insert_torrent.is_deleted,
                    seeders: insert_torrent.seeders,
                    leechers: insert_torrent.leechers,
                    times_completed: insert_torrent.times_completed,
                    download_factor: insert_torrent.download_factor,
                    upload_factor: insert_torrent.upload_factor,
                    peers,
                });

            // Safe since the value being modified implements Copy.
            unsafe {
                tracker
                    .infohash2id
                    .entry(info_hash)
                    .and_modify(|id| {
                        *id = insert_torrent.id;
                    })
                    .or_insert(insert_torrent.id);
            }

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(torrent): Json<APIRemoveTorrent>,
    ) -> StatusCode {
        tracker.torrents.update(&torrent.id, |_index, torrent| {
            println!("Removing torrent with id {}.", torrent.id);
            torrent.is_deleted = true;

            return StatusCode::OK;
        });

        StatusCode::BAD_REQUEST
    }

    pub async fn show(
        State(tracker): State<Arc<Tracker>>,
        Path(id): Path<u32>,
    ) -> Result<Json<Torrent>, StatusCode> {
        tracker
            .torrents
            .read(&id, |_index, torrent| Json(torrent.clone()))
            .ok_or(StatusCode::NOT_FOUND)
    }
}

impl Deref for Map {
    type Target = HashMap<u32, Torrent, RandomState>;

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
    pub info_hash: String,
}
