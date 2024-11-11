use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use diesel::deserialize::Queryable;
use diesel::Selectable;
use indexmap::IndexMap;
use serde::Deserialize;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

use super::Db;

pub struct Map(IndexMap<i32, Group>);

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &Db) -> Result<Map> {
        use crate::schema::groups;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let groups_data = groups::table
            .select(DBImportGroup::as_select())
            .load(&mut db.get().await?)
            .await
            .context("Failed loading groups.")?;

        let mut group_map = Map::new();

        for group in groups_data {
            group_map.insert(group.id, group.into());
        }

        Ok(group_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(group): Json<APIInsertGroup>,
    ) -> StatusCode {
        println!("Inserting group with id {}.", group.id);

        tracker.groups.write().insert(
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

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(group): Json<APIRemoveGroup>,
    ) -> StatusCode {
        println!("Removing group with id {}.", group.id);

        tracker.groups.write().swap_remove(&group.id);

        StatusCode::OK
    }
}

impl Deref for Map {
    type Target = IndexMap<i32, Group>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DBImportGroup> for Group {
    fn from(value: DBImportGroup) -> Self {
        Self {
            id: value.id,
            slug: value.slug,
            download_slots: value.download_slots,
            is_immune: value.is_immune,
            download_factor: if value.is_freeleech { 0 } else { 100 },
            upload_factor: if value.is_double_upload { 200 } else { 100 },
        }
    }
}

#[diesel(table_name = crate::schema::groups)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Queryable, Selectable)]
pub struct DBImportGroup {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<i32>,
    pub is_immune: bool,
    pub is_freeleech: bool,
    pub is_double_upload: bool,
}

#[derive(Clone, Deserialize, Hash)]
pub struct Group {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<i32>,
    pub is_immune: bool,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertGroup {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<i32>,
    pub is_immune: bool,
    pub is_freeleech: bool,
    pub is_double_upload: bool,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveGroup {
    pub id: i32,
}
