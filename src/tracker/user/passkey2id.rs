use std::ops::{Deref, DerefMut};

use super::Passkey;
use ahash::RandomState;
use scc::HashIndex;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct Map(HashIndex<Passkey, u32, RandomState>);

impl Deref for Map {
    type Target = HashIndex<Passkey, u32, RandomState>;

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
        Map(HashIndex::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Map> {
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
        .context("Failed loading user passkey to id mappings.")?;

        let passkey2id_map = Map::new();

        for passkey2id in passkey2ids {
            passkey2id_map
                .entry(passkey2id.passkey)
                .or_insert(passkey2id.id);
        }

        Ok(passkey2id_map)
    }
}

pub struct Passkey2Id {
    pub id: u32,
    pub passkey: Passkey,
}
