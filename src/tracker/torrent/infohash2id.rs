use std::ops::{Deref, DerefMut};

use crate::tracker::torrent::InfoHash;
use diesel::{prelude::Queryable, Selectable};
use indexmap::IndexMap;

use anyhow::{Context, Result};

use super::Db;

pub struct Map(IndexMap<InfoHash, u32>);

impl Deref for Map {
    type Target = IndexMap<InfoHash, u32>;

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
        use crate::schema::torrents;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let info_hash2ids_data = torrents::table
            .select(InfoHash2Id::as_select())
            .filter(torrents::deleted_at.is_null())
            .load::<InfoHash2Id>(&mut db.get().await?)
            .await
            .context("Failed loading torrent infohash to id mappings.")?;

        let mut info_hash2id_map = Map::new();

        for info_hash2id in info_hash2ids_data {
            info_hash2id_map.insert(info_hash2id.info_hash, info_hash2id.id);
        }

        Ok(info_hash2id_map)
    }
}

#[derive(Queryable, Selectable)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[diesel(table_name = crate::schema::torrents)]
pub struct InfoHash2Id {
    pub id: u32,
    pub info_hash: InfoHash,
}
