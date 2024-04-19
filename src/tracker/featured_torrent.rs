use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::Json;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub struct Set(IndexSet<FeaturedTorrent>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let featured_torrents = sqlx::query_as!(
            FeaturedTorrent,
            r#"
                SELECT
                    torrent_id as `torrent_id: u32`
                FROM
                    featured_torrents
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading featured torrents.")?;

        let mut featured_torrent_set = Set::new();

        for featured_torrent in featured_torrents {
            featured_torrent_set.insert(featured_torrent);
        }

        Ok(featured_torrent_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Json(token): Json<FeaturedTorrent>) {
        println!(
            "Inserting featured torrent with torrent_id {}.",
            token.torrent_id
        );

        tracker.featured_torrents.write().insert(token);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(token): Json<FeaturedTorrent>) {
        println!(
            "Removing featured torrent with torrent_id {}.",
            token.torrent_id
        );

        tracker.featured_torrents.write().swap_remove(&token);
    }
}

impl Deref for Set {
    type Target = IndexSet<FeaturedTorrent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Set {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct FeaturedTorrent {
    pub torrent_id: u32,
}
