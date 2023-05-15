use std::ops::DerefMut;
use std::str::FromStr;
use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use indexmap::IndexMap;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub mod passkey;
pub use passkey::Passkey;

pub mod passkey2id;
pub use passkey2id::Passkey2Id;

pub struct Map(IndexMap<u32, User>);

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        let users = sqlx::query_as!(
            User,
            r#"
                SELECT
                    users.id as `id: u32`,
                    users.passkey as `passkey: Passkey`,
                    users.can_download as `can_download: bool`,
                    groups.download_slots as `download_slots: u32`,
                    groups.is_immune as `is_immune: bool`,
                    COALESCE(SUM(peers.seeder = 1), 0) as `num_seeding: u32`,
                    COALESCE(SUM(peers.seeder = 0), 0) as `num_leeching: u32`,
                    IF(groups.is_freeleech, 0, 100) as `download_factor: u8`,
                    IF(groups.is_double_upload, 200, 100) as `upload_factor: u8`
                FROM
                    users
                INNER JOIN
                    `groups`
                ON
                    users.group_id = `groups`.id
                    AND groups.slug NOT IN ('banned', 'validating', 'disabled')
                    AND users.deleted_at IS NULL
                LEFT JOIN
                    peers
                ON
                    users.id = peers.user_id
                GROUP BY
                    users.id,
                    users.passkey,
                    users.can_download,
                    groups.download_slots,
                    groups.is_immune,
                    groups.is_freeleech,
                    groups.is_double_upload
            "#
        )
        .fetch_all(db)
        .await
        .map_err(|error| {
            println!("{}", error);
            Error("Failed loading users.")
        })?;

        let mut user_map = Map::new();

        for user in users {
            user_map.insert(user.id, user);
        }
        Ok(user_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Query(user): Query<APIInsertUser>,
    ) -> StatusCode {
        if let Ok(passkey) = Passkey::from_str(&user.passkey) {
            println!("Inserting user with id {}.", user.id);

            tracker.users.write().await.insert(
                user.id,
                User {
                    id: user.id,
                    passkey,
                    can_download: user.can_download,
                    download_slots: user.download_slots,
                    is_immune: user.is_immune,
                    num_seeding: user.num_seeding,
                    num_leeching: user.num_leeching,
                    download_factor: user.download_factor,
                    upload_factor: user.upload_factor,
                },
            );

            tracker.passkey2id.write().await.insert(passkey, user.id);

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Query(user): Query<APIRemoveUser>,
    ) -> StatusCode {
        if let Ok(passkey) = Passkey::from_str(&user.passkey) {
            println!("Removing user with id {}.", user.id);

            tracker.users.write().await.remove(&user.id);
            tracker.passkey2id.write().await.remove(&passkey);

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }
}

impl Deref for Map {
    type Target = IndexMap<u32, User>;

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
pub struct User {
    pub id: u32,
    pub passkey: Passkey,
    pub can_download: bool,
    pub download_slots: Option<u32>,
    pub is_immune: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertUser {
    pub id: u32,
    pub passkey: String,
    pub can_download: bool,
    pub download_slots: Option<u32>,
    pub is_immune: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveUser {
    pub id: u32,
    pub passkey: String,
}
