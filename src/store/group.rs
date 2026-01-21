use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use futures_util::TryStreamExt;
use indexmap::IndexMap;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};
use tracing::info;

use crate::state::AppState;

pub struct GroupStore {
    inner: IndexMap<i32, Group>,
}

impl GroupStore {
    pub fn new() -> GroupStore {
        GroupStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<GroupStore> {
        let mut groups = sqlx::query_as!(
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
        .fetch(db);

        let mut group_map = GroupStore::new();

        while let Some(group) = groups.try_next().await.context("Failed loading groups.")? {
            group_map.insert(group.id, group);
        }

        Ok(group_map)
    }
}

impl Deref for GroupStore {
    type Target = IndexMap<i32, Group>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GroupStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Json(group): Json<APIInsertGroup>,
) -> StatusCode {
    info!("Inserting group with id {}.", group.id);

    state.stores.groups.write().insert(
        group.id,
        Group {
            id: group.id,
            slug: group.slug,
            download_slots: group.download_slots,
            is_immune: group.is_immune,
            download_factor: if group.is_freeleech { 0 } else { 100 },
            upload_factor: if group.is_double_upload { 200 } else { 100 },
        },
    );

    StatusCode::OK
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveGroup {
    pub id: i32,
}

pub async fn destroy(
    State(state): State<Arc<AppState>>,
    Json(group): Json<APIRemoveGroup>,
) -> StatusCode {
    info!("Removing group with id {}.", group.id);

    state.stores.groups.write().swap_remove(&group.id);

    StatusCode::OK
}
