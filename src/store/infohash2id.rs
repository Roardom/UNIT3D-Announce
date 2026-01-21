use std::ops::{Deref, DerefMut};

use crate::model::info_hash::InfoHash;
use futures_util::TryStreamExt;
use indexmap::IndexMap;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct InfoHash2IdStore {
    inner: IndexMap<InfoHash, u32>,
}

impl Deref for InfoHash2IdStore {
    type Target = IndexMap<InfoHash, u32>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for InfoHash2IdStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl InfoHash2IdStore {
    pub fn new() -> InfoHash2IdStore {
        InfoHash2IdStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<InfoHash2IdStore> {
        // Load one torrent per info hash. If multiple are found, prefer
        // undeleted torrents. If multiple are still found, prefer approved
        // torrents. If multiple are still found, prefer the oldest.
        sqlx::query_as!(
            InfoHash2Id,
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.info_hash as `info_hash: InfoHash`
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
        .fetch(db)
        .try_fold(InfoHash2IdStore::new(), |mut store, torrent| async move {
            store.insert(torrent.info_hash, torrent.id);

            Ok(store)
        })
        .await
        .context("Failed loading torrent infohash to id mappings.")
    }
}

pub struct InfoHash2Id {
    pub id: u32,
    pub info_hash: InfoHash,
}
