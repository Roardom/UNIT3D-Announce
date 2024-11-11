use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use chrono::NaiveDateTime;
use diesel::deserialize::Queryable;
use indexmap::IndexMap;

use anyhow::{Context, Result};

use super::Db;

#[derive(Clone)]
pub struct Map(IndexMap<SocketAddr, ConnectablePort>);

#[derive(Clone, Copy, Debug)]
pub struct ConnectablePort {
    pub connectable: bool,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Clone, Copy, Debug)]
pub struct DBImportConnectablePort {
    #[diesel(column_name = ip)]
    #[diesel(deserialize_as = crate::tracker::peer::IpAddress)]
    pub ip: IpAddr,
    pub port: u16,
    pub connectable: bool,
    pub updated_at: Option<NaiveDateTime>,
}

impl Map {
    pub fn new() -> Map {
        Map(IndexMap::new())
    }

    pub async fn from_db(db: &Db) -> Result<Map> {
        use crate::schema::*;
        use diesel::dsl::max;
        use diesel::dsl::sql;
        use diesel::prelude::*;
        use diesel::sql_types::Bool;
        use diesel_async::RunQueryDsl;
        let peers_data = peers::table
            .group_by((peers::ip, peers::port))
            .select((
                peers::ip,
                peers::port,
                sql::<Bool>("COALESCE(MAX(peers.connectable), 0)"),
                max(peers::updated_at),
            ))
            .load::<DBImportConnectablePort>(&mut db.get().await?)
            .await
            .context("Failed loading peers.")?;

        let mut peer_map = Map::new();

        for DBImportConnectablePort {
            ip,
            port,
            connectable,
            updated_at,
        } in peers_data
        {
            peer_map.insert(
                SocketAddr::from((ip, port)),
                ConnectablePort {
                    connectable,
                    updated_at: updated_at.expect("Nullable peer updated_at."),
                },
            );
        }

        Ok(peer_map)
    }
}

impl Deref for Map {
    type Target = IndexMap<SocketAddr, ConnectablePort>;

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
