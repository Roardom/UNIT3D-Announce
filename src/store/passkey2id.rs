use std::ops::{Deref, DerefMut};

use crate::model::passkey::Passkey;
use futures_util::TryStreamExt;
use indexmap::IndexMap;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct Passkey2IdStore {
    inner: IndexMap<Passkey, u32>,
}

impl Deref for Passkey2IdStore {
    type Target = IndexMap<Passkey, u32>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Passkey2IdStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Passkey2IdStore {
    pub fn new() -> Passkey2IdStore {
        Passkey2IdStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Passkey2IdStore> {
        let mut passkey2ids = sqlx::query_as!(
            Passkey2Id,
            r#"
                SELECT
                    users.id as `id: u32`,
                    users.passkey as `passkey: Passkey`
                FROM
                    users
                WHERE
                    users.deleted_at IS NULL
            "#
        )
        .fetch(db);

        let mut passkey2id_map = Passkey2IdStore::new();

        while let Some(passkey2id) = passkey2ids
            .try_next()
            .await
            .context("Failed loading user passkey to id mappings.")?
        {
            passkey2id_map.insert(passkey2id.passkey, passkey2id.id);
        }

        Ok(passkey2id_map)
    }
}

pub struct Passkey2Id {
    pub id: u32,
    pub passkey: Passkey,
}
