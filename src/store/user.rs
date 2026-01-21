use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::rate::RateCollection;

use crate::model::passkey::Passkey;

#[derive(Serialize)]
pub struct UserStore {
    inner: IndexMap<u32, User>,
}

impl UserStore {
    pub fn new() -> UserStore {
        UserStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool, config: &Config) -> Result<UserStore> {
        sqlx::query_as!(
            DBImportUser,
            r#"
                SELECT
                    users.id as `id: u32`,
                    users.group_id as `group_id: i32`,
                    users.passkey as `passkey: Passkey`,
                    users.can_download as `can_download: bool`,
                    CAST(COALESCE(SUM(peers.seeder = 1 AND peers.active = 1 AND peers.visible = 1), 0) AS UNSIGNED) as `num_seeding!: u32`,
                    CAST(COALESCE(SUM(peers.seeder = 0 AND peers.active = 1 AND peers.visible = 1), 0) AS UNSIGNED) as `num_leeching!: u32`,
                    users.is_donor as `is_donor: bool`,
                    users.is_lifetime as `is_lifetime: bool`
                FROM
                    users
                LEFT JOIN
                    peers
                ON
                    users.id = peers.user_id
                WHERE
                    users.deleted_at IS NULL
                GROUP BY
                    users.id
            "#
        )
        .fetch(db)
        .try_fold(UserStore::new(), |mut store, user| async move {
            store.insert(
                user.id,
                User {
                    id: user.id,
                    group_id: user.group_id,
                    passkey: user.passkey,
                    can_download: user.can_download,
                    num_seeding: user.num_seeding,
                    num_leeching: user.num_leeching,
                    receive_seed_list_rates: config.user_receive_seed_list_rate_limits.clone(),
                    receive_leech_list_rates: config.user_receive_leech_list_rate_limits.clone(),
                    is_donor: user.is_donor,
                    is_lifetime: user.is_lifetime,
                },
            );

            Ok(store)
        })
        .await
        .context("Failed loading users.")
    }
}

impl Deref for UserStore {
    type Target = IndexMap<u32, User>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for UserStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Clone)]
pub struct DBImportUser {
    pub id: u32,
    pub group_id: i32,
    pub passkey: Passkey,
    pub can_download: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
    pub is_donor: bool,
    pub is_lifetime: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub id: u32,
    pub group_id: i32,
    pub passkey: Passkey,
    pub can_download: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
    pub is_donor: bool,
    pub is_lifetime: bool,
    pub receive_seed_list_rates: RateCollection,
    pub receive_leech_list_rates: RateCollection,
}
