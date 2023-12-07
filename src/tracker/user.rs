use std::ops::DerefMut;
use std::str::FromStr;
use std::{ops::Deref, sync::Arc};

use ahash::RandomState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use scc::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub mod passkey;
pub use passkey::Passkey;

pub mod passkey2id;
pub use passkey2id::Passkey2Id;

#[derive(Serialize)]
pub struct Map(HashMap<u32, User, RandomState>);

impl Map {
    pub fn new() -> Map {
        Map(HashMap::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
        let users = sqlx::query_as!(
            User,
            r#"
                SELECT
                    users.id as `id: u32`,
                    users.group_id as `group_id: i32`,
                    users.passkey as `passkey: Passkey`,
                    users.can_download as `can_download: bool`,
                    CAST(COALESCE(SUM(peers.seeder = 1 AND peers.active = 1), 0) AS UNSIGNED) as `num_seeding: u32`,
                    CAST(COALESCE(SUM(peers.seeder = 0 AND peers.active = 1), 0) AS UNSIGNED) as `num_leeching: u32`
                FROM
                    users
                LEFT JOIN
                    peers
                ON
                    users.id = peers.user_id
                    AND users.deleted_at IS NULL
                GROUP BY
                    users.id
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading users.")?;

        let user_map = Map::new();

        for user in users {
            user_map.entry(user.id).or_insert(user);
        }
        Ok(user_map)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(insert_user): Json<APIInsertUser>,
    ) -> StatusCode {
        println!("Received user: {}", insert_user.id);
        if let Ok(passkey) = Passkey::from_str(&insert_user.passkey) {
            println!("Inserting user with id {}.", insert_user.id);

            tracker
                .users
                .entry(insert_user.id)
                .and_modify(|user| {
                    user.group_id = insert_user.group_id;
                    user.passkey = passkey;
                    user.can_download = insert_user.can_download;
                    user.num_seeding = insert_user.num_seeding;
                    user.num_leeching = insert_user.num_leeching;
                })
                .or_insert(User {
                    id: insert_user.id,
                    group_id: insert_user.group_id,
                    passkey,
                    can_download: insert_user.can_download,
                    num_seeding: insert_user.num_seeding,
                    num_leeching: insert_user.num_leeching,
                });

            // Safe since the value being modified implements Copy.
            unsafe {
                tracker
                    .passkey2id
                    .entry(passkey)
                    .and_modify(|id| {
                        *id = insert_user.id;
                    })
                    .or_insert(insert_user.id);
            }

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(user): Json<APIRemoveUser>,
    ) -> StatusCode {
        if let Ok(passkey) = Passkey::from_str(&user.passkey) {
            println!("Removing user with id {}.", user.id);

            tracker.users.remove(&user.id);
            tracker.passkey2id.remove(&passkey);

            return StatusCode::OK;
        }

        StatusCode::BAD_REQUEST
    }

    pub async fn show(
        State(tracker): State<Arc<Tracker>>,
        Path(id): Path<u32>,
    ) -> Result<Json<User>, StatusCode> {
        tracker
            .users
            .get(&id)
            .map(|user| Json(user.get().clone()))
            .ok_or(StatusCode::NOT_FOUND)
    }
}

impl Deref for Map {
    type Target = HashMap<u32, User, RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Deserialize, Hash, Serialize)]
pub struct User {
    pub id: u32,
    pub group_id: i32,
    pub passkey: Passkey,
    pub can_download: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertUser {
    pub id: u32,
    pub group_id: i32,
    pub passkey: String,
    pub can_download: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveUser {
    pub id: u32,
    pub passkey: String,
}
