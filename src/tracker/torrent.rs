use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use axum::extract::{Query, State};
use indexmap::IndexMap;
use serde::Deserialize;
use sqlx::MySqlPool;
use tokio::sync::RwLock;

use crate::tracker::peer;
use crate::Error;

pub mod infohash;
pub use infohash::InfoHash;

pub mod infohash2id;
pub use infohash2id::InfoHash2Id;

pub mod status;
pub use status::Status;

use crate::tracker::Tracker;

pub struct Map(IndexMap<u32, Torrent>);

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
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
            let mut peer_map = peer::Map::default();
            let mut seeders = 0;
            let mut leechers = 0;

            for (index, peer) in peers.iter() {
                if peer.torrent_id == row.id {
                    peer_map.insert(*index, *peer);

                    if peer.is_active {
                        if peer.is_seeder {
                            seeders += 1;
                        } else {
                            leechers += 1;
                        }
                    }
                }
            }

            let torrent = Torrent {
                id: row.id,
                status: row.status,
                seeders,
                leechers,
                times_completed: row.times_completed,
                download_factor: row.download_factor,
                upload_factor: row.upload_factor,
                is_deleted: row.is_deleted,
                peers: Arc::new(RwLock::new(peer_map)),
            };

            torrent
        })
        .fetch_all(db)
        .await
        .map_err(|error| {
            println!("{}", error);
            Error("Failed loading torrents.")
        })?;

        let mut torrent_map = Map::new();

        for torrent in torrents {
            torrent_map.insert(torrent.id, torrent);
        }

        Ok(torrent_map)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(torrent): Query<Torrent>) {
        tracker.torrents.write().await.insert(torrent.id, torrent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(torrent): Query<Torrent>) {
        tracker.torrents.write().await.remove(&torrent.id);
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

#[derive(Clone, Deserialize)]
pub struct Torrent {
    pub id: u32,
    pub status: Status,
    pub is_deleted: bool,
    #[serde(skip)]
    pub peers: Arc<RwLock<peer::Map>>,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}
