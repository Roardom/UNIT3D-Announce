use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::Json;
use diesel::prelude::Queryable;
use diesel::Selectable;
use indexmap::IndexSet;
use serde::Deserialize;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

use super::Db;

pub struct Set(pub IndexSet<Agent>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &Db) -> Result<Set> {
        use crate::schema::blacklist_clients;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let agents: Vec<Agent> = blacklist_clients::table
            .select(Agent::as_select())
            .load(&mut db.get().await?)
            .await
            .context("Failed loading blacklisted clients.")?;

        let mut agent_set = Set::new();

        for agent in agents {
            agent_set.insert(agent);
        }

        Ok(agent_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Json(agent): Json<Agent>) {
        println!(
            "Inserting agent with peer_id_prefix {}.",
            agent.peer_id_prefix,
        );

        tracker.agent_blacklist.write().insert(agent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(agent): Json<Agent>) {
        println!(
            "Removing agent with peer_id_prefix {}.",
            agent.peer_id_prefix,
        );

        tracker.agent_blacklist.write().swap_remove(&agent);
    }
}

impl Deref for Set {
    type Target = IndexSet<Agent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Set {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::blacklist_clients)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct Agent {
    pub peer_id_prefix: String,
}
