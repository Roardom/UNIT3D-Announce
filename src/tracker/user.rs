use std::{ops::Deref, str::FromStr, sync::Arc};

use axum::extract::{Query, State};
use dashmap::DashMap;
use serde::Deserialize;
use sqlx::{database::HasValueRef, Database, Decode, MySqlPool};

use crate::Error;

use crate::tracker::Tracker;

pub struct Map(DashMap<Passkey, User>);

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
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
                    COUNT(peers.id) as `num_seeding: u32`,
                    COUNT(peers.id) as `num_leeching: u32`,
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
                INNER JOIN
                    peers
                ON
                    users.id = peers.user_id
                GROUP BY
                    peers.user_id
            "#
        )
        .fetch_all(db)
        .await
        .map_err(|_| Error("Failed loading users."))?;

        let user_map = Map::new();

        for user in users {
            user_map.insert(user.passkey, user);
        }
        Ok(user_map)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(user): Query<User>) {
        tracker.users.insert(user.passkey, user);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(user): Query<User>) {
        tracker.users.remove(&user.passkey);
    }
}

impl Deref for Map {
    type Target = DashMap<Passkey, User>;

    fn deref(&self) -> &Self::Target {
        &self.0
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Passkey(pub [u8; 32]);

impl FromStr for Passkey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = s.bytes();

        if bytes.len() != 32 {
            return Err(Error("Invalid passkey length."));
        }

        let array = [(); 32].map(|_| bytes.next().unwrap());

        Ok(Passkey(array))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Passkey
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<Passkey, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&str as Decode<DB>>::decode(value)?;
        let mut bytes = value.bytes();

        let array = [(); 32].map(|_| bytes.next().expect("Invalid passkey length."));

        Ok(Passkey(array))
    }
}
