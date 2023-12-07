use std::ops::{Deref, DerefMut};

use crate::tracker::torrent::InfoHash;
use scc::HashIndex;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct Map(HashIndex<InfoHash, u32, ahash::RandomState>);

impl Deref for Map {
    type Target = HashIndex<InfoHash, u32, ahash::RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Map {
    pub fn new() -> Map {
        Map(HashIndex::with_hasher(ahash::RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
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
        .context("Failed loading torrent infohash to id mappings.")?;

        let info_hash2id_map = Map::new();

        for info_hash2id in info_hash2ids {
            info_hash2id_map
                .entry(info_hash2id.info_hash)
                .or_insert(info_hash2id.id);
        }

        Ok(info_hash2id_map)
    }
}

pub struct InfoHash2Id {
    pub id: u32,
    pub info_hash: InfoHash,
}
