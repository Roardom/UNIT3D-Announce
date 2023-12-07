use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use ahash::RandomState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use scc::HashMap;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub struct Map(HashMap<i32, Group, RandomState>);

impl Map {
    pub fn new() -> Map {
        Map(HashMap::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        let groups = sqlx::query_as!(
            Group,
            r#"
                SELECT
                    id as `id: i32`,
                    slug as `slug: String`,
                    download_slots as `download_slots: u32`,
                    is_immune as `is_immune: bool`,
                    IF(is_freeleech, 0, 100) as `download_factor: u8`,
                    IF(is_double_upload, 200, 100) as `upload_factor: u8`
                FROM
                    `groups`
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading groups.")?;

        let group_map = Map::new();

        for group in groups {
            group_map.entry(group.id).or_insert(group);
        }

        Ok(group_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(insert_group): Json<APIInsertGroup>,
    ) -> StatusCode {
        println!("Inserting group with id {}.", insert_group.id);

        tracker
            .groups
            .entry(insert_group.id)
            .and_modify(|group| {
                group.slug = insert_group.slug.clone();
                group.download_slots = insert_group.download_slots;
                group.is_immune = insert_group.is_immune;
                group.download_factor = if insert_group.is_freeleech { 0 } else { 100 };
                group.upload_factor = if insert_group.is_double_upload {
                    200
                } else {
                    100
                };
            })
            .or_insert(Group {
                id: insert_group.id,
                slug: insert_group.slug,
                download_slots: insert_group.download_slots,
                is_immune: insert_group.is_immune,
                download_factor: if insert_group.is_freeleech { 0 } else { 100 },
                upload_factor: if insert_group.is_double_upload {
                    200
                } else {
                    100
                },
            });

        return StatusCode::OK;
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(group): Json<APIRemoveGroup>,
    ) -> StatusCode {
        println!("Removing group with id {}.", group.id);

        tracker.groups.remove(&group.id);

        return StatusCode::OK;
    }
}

impl Deref for Map {
    type Target = HashMap<i32, Group, RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Deserialize, Hash)]
pub struct Group {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<u32>,
    pub is_immune: bool,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertGroup {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<u32>,
    pub is_immune: bool,
    pub is_freeleech: bool,
    pub is_double_upload: bool,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveGroup {
    pub id: i32,
}
