use std::ops::Deref;
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::MySqlPool;

use crate::tracker::peer;
use crate::Error;

pub mod infohash;
pub use infohash::InfoHash;

pub mod status;
pub use status::Status;

pub struct Map(DashMap<InfoHash, Torrent>);

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        // TODO: deleted_at column still needs added to unit3d. Until then, no
        // torrents are considered deleted.
        let torrents: Vec<Torrent> = sqlx::query!(
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.info_hash as `info_hash: InfoHash`,
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
        // If https://github.com/launchbadge/sqlx/issues/1106 is ever solved,
        // then `map` would not be necessary as we could just add
        // "SELECT NULL as `peers: PeerMap`" to the query and use sqlx's
        // query_as! macro
        .map(|row| Torrent {
            id: row.id,
            info_hash: row.info_hash,
            status: row.status,
            seeders: row.seeders,
            leechers: row.leechers,
            times_completed: row.times_completed,
            download_factor: row.download_factor,
            upload_factor: row.upload_factor,
            is_deleted: row.is_deleted,
            peers: Arc::new(peer::Map::default()),
        })
        .fetch_all(db)
        .await
        .map_err(|_| Error("Failed loading torrents."))?;

        let torrent_map = Map::new();

        for torrent in torrents {
            torrent_map.insert(torrent.info_hash, torrent);
        }

        Ok(torrent_map)
    }
}

impl Deref for Map {
    type Target = DashMap<InfoHash, Torrent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Torrent {
    pub id: u32,
    pub info_hash: InfoHash,
    pub status: Status,
    pub is_deleted: bool,
    pub peers: Arc<peer::Map>,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}
