use std::ops::Deref;

use crate::tracker::torrent::InfoHash;
use dashmap::DashMap;
use sqlx::MySqlPool;

use crate::Error;

pub struct Map(DashMap<InfoHash, u32>);

impl Deref for Map {
    type Target = DashMap<InfoHash, u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        let info_hash2ids = sqlx::query_as!(
            InfoHash2Id,
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.info_hash as `info_hash: InfoHash`
                FROM
                    torrents
            "#
        )
        .fetch_all(db)
        .await
        .map_err(|_| Error("Failed loading torrent infohash to id mappings."))?;

        let info_hash2id_map = Map::new();

        for info_hash2id in info_hash2ids {
            info_hash2id_map.insert(info_hash2id.info_hash, info_hash2id.id);
        }

        Ok(info_hash2id_map)
    }
}

pub struct InfoHash2Id {
    pub id: u32,
    pub info_hash: InfoHash,
}
