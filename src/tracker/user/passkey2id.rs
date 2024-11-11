use std::ops::{Deref, DerefMut};

use super::Passkey;
use diesel::{prelude::Queryable, Selectable};
use indexmap::IndexMap;

use anyhow::{Context, Result};

use super::Db;

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

    pub async fn from_db(db: &Db) -> Result<Map> {
        use crate::schema::users;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let passkey2ids_data = users::table
            .select(Passkey2Id::as_select())
            .filter(users::deleted_at.is_null())
            .load::<Passkey2Id>(&mut db.get().await?)
            .await
            .context("Failed loading user passkey to id mappings.")?;

        let mut passkey2id_map = Map::new();

        for passkey2id in passkey2ids_data {
            passkey2id_map.insert(passkey2id.passkey, passkey2id.id);
        }

        Ok(passkey2id_map)
    }
}

#[derive(Queryable, Selectable)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[diesel(table_name = crate::schema::users)]
pub struct Passkey2Id {
    pub id: u32,
    pub passkey: Passkey,
}
