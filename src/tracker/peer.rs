use std::fmt::Display;
use std::net::IpAddr;
use std::ops::{Deref, DerefMut};

use chrono::naive::serde::ts_seconds_option;
use chrono::NaiveDateTime;
use diesel::deserialize::Queryable;
use diesel::Selectable;
use indexmap::IndexMap;
use serde::{Serialize, Serializer};

use anyhow::{Context, Result};

pub mod peer_id;
pub use peer_id::PeerId;
pub mod ip_address;
pub use ip_address::IpAddress;

use crate::config::Config;

use super::Db;

#[derive(Clone, Serialize)]
pub struct Map(IndexMap<Index, Peer>);

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::peers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
    pub user_id: u32,
    pub peer_id: PeerId,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::peers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Peer {
    #[diesel(deserialize_as = IpAddress)]
    #[diesel(column_name = "ip")]
    pub ip_address: IpAddr,
    pub user_id: u32,
    pub torrent_id: u32,
    pub port: u16,
    #[diesel(column_name = "seeder")]
    pub is_seeder: bool,
    #[diesel(column_name = "active")]
    pub is_active: bool,
    #[diesel(column_name = "visible")]
    pub is_visible: bool,
    #[diesel(column_name = "connectable")]
    pub is_connectable: bool,
    #[serde(with = "ts_seconds_option")]
    pub updated_at: Option<NaiveDateTime>,
    pub uploaded: u64,
    pub downloaded: u64,
}

impl Peer {
    /// Determines if the peer should be included in the peer list
    #[inline(always)]
    pub fn is_included_in_peer_list(&self, config: &Config) -> bool {
        if config.require_peer_connectivity {
            self.is_active && self.is_visible && self.is_connectable
        } else {
            self.is_active && self.is_visible
        }
    }

    /// Determines if the peer should be included in the list of seeds
    #[inline(always)]
    pub fn is_included_in_seed_list(&self, config: &Config) -> bool {
        self.is_seeder && self.is_included_in_peer_list(config)
    }

    /// Determines if the peer should be included in the list of leeches
    #[inline(always)]
    pub fn is_included_in_leech_list(&self, config: &Config) -> bool {
        !self.is_seeder && self.is_included_in_peer_list(config)
    }
}

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &Db) -> Result<Vec<(Index, Peer)>> {
        use crate::schema::peers;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let peers_data = peers::table
            .select((Index::as_select(), Peer::as_select()))
            .load(&mut db.get().await?)
            .await
            .context("Failed loading peers.")?;

        Ok(peers_data)
    }
}

impl Deref for Map {
    type Target = IndexMap<Index, Peer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Map {
    fn default() -> Self {
        Map::new()
    }
}

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.user_id, self.peer_id)
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
