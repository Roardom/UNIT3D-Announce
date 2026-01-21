use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct FeaturedTorrentStore {
    inner: IndexSet<FeaturedTorrent>,
}

impl FeaturedTorrentStore {
    pub fn new() -> FeaturedTorrentStore {
        FeaturedTorrentStore {
            inner: IndexSet::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<FeaturedTorrentStore> {
        sqlx::query_as!(
            FeaturedTorrent,
            r#"
                SELECT
                    torrent_id as `torrent_id: u32`
                FROM
                    featured_torrents
            "#
        )
        .fetch(db)
        .try_fold(
            FeaturedTorrentStore::new(),
            |mut store, featured_torrent| async move {
                store.insert(featured_torrent);

                Ok(store)
            },
        )
        .await
        .context("Failed loading featured torrents.")
    }
}

impl Deref for FeaturedTorrentStore {
    type Target = IndexSet<FeaturedTorrent>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for FeaturedTorrentStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct FeaturedTorrent {
    pub torrent_id: u32,
}
