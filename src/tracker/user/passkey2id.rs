use std::ops::{Deref, DerefMut};

use super::Passkey;
use indexmap::IndexMap;
use sqlx::MySqlPool;

use crate::Error;

pub struct Map(IndexMap<Passkey, u32>);

impl Deref for Map {
    type Target = IndexMap<Passkey, u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
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

        let mut passkey2id_map = Map::new();

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
