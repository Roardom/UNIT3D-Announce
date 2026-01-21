use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexMap;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

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
