use std::ops::Deref;

use super::Passkey;
use dashmap::DashMap;
use sqlx::MySqlPool;

use crate::Error;

pub struct Map(DashMap<Passkey, u32>);

impl Deref for Map {
    type Target = DashMap<Passkey, u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Map {
    pub fn new() -> Map {
        Map(DashMap::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map, Error> {
        let passkey2ids = sqlx::query_as!(
            Passkey2Id,
            r#"
                SELECT
                    users.id as `id: u32`,
                    users.passkey as `passkey: Passkey`
                FROM
                    users
            "#
        )
        .fetch_all(db)
        .await
        .map_err(|_| Error("Failed loading user passkey to id mappings."))?;

        let passkey2id_map = Map::new();

        for passkey2id in passkey2ids {
            passkey2id_map.insert(passkey2id.passkey, passkey2id.id);
        }

        Ok(passkey2id_map)
    }
}

pub struct Passkey2Id {
    pub id: u32,
    pub passkey: Passkey,
}
