use std::ops::Deref;
use std::sync::Arc;

use axum::extract::{Query, State};
use dashmap::DashMap;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::tracker::peer::{self, Peer};
use crate::Error;

pub mod infohash;
pub use infohash::InfoHash;

pub mod infohash2id;
pub use infohash2id::InfoHash2Id;

pub mod status;
pub use status::Status;

use crate::tracker::Tracker;

pub struct Map(DashMap<u32, Torrent>);

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        let peers = peer::Map::from_db(db).await?;
        // TODO: deleted_at column still needs added to unit3d. Until then, no
        // torrents are considered deleted.
        let torrents: Vec<Torrent> = sqlx::query!(
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.status as `status: Status`,
                    torrents.seeders as `seeders: u32`,
                    torrents.leechers as `leechers: u32`,
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
        .map(|row| {
            let torrent = Torrent {
                id: row.id,
                status: row.status,
                seeders: row.seeders,
                leechers: row.leechers,
                times_completed: row.times_completed,
                download_factor: row.download_factor,
                upload_factor: row.upload_factor,
                is_deleted: row.is_deleted,
                peers: Arc::new(peer::Map::default()),
            };

            // TODO: use drain_filter once stabilized.
            let peers = peers.iter().filter_map(|peer| {
                if peer.torrent_id == row.id {
                    Some((
                        peer::Index {
                            user_id: peer.user_id,
                            peer_id: peer.key().peer_id
                        },
                        Peer {
                            ip_address: peer.ip_address,
                            user_id: peer.user_id,
                            torrent_id: peer.torrent_id,
                            port: peer.port,
                            is_seeder: peer.is_seeder,
                            is_active: peer.is_active,
                            updated_at: peer.updated_at,
                            uploaded: peer.uploaded,
                            downloaded: peer.downloaded,
                        }
                    ))
                } else {
                    None
                }
            });

            for (index, peer) in peers {
                torrent.peers.insert(index, peer);
            }

            torrent
    })
        .fetch_all(db)
        .await
        .map_err(|_| Error("Failed loading torrents."))?;

        let torrent_map = Map::new();

        for torrent in torrents {
            torrent_map.insert(torrent.id, torrent);
        }

        Ok(torrent_map)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(torrent): Query<Torrent>) {
        tracker.torrents.insert(torrent.id, torrent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(torrent): Query<Torrent>) {
        tracker.torrents.remove(&torrent.id);
    }
}

impl Deref for Map {
    type Target = DashMap<u32, Torrent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Deserialize)]
pub struct Torrent {
    pub id: u32,
    pub status: Status,
    pub is_deleted: bool,
    #[serde(skip)]
    pub peers: Arc<peer::Map>,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}
