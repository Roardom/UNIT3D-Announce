use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::Json;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub struct Set(pub IndexSet<Agent>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let agents = sqlx::query_as!(
            Agent,
            r#"
                SELECT
                    peer_id_prefix
                FROM
                    blacklist_clients
            "#
        )
        .fetch_all(db)
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
            "Inserting agent with peer_id_prefix {:?}.",
            agent.peer_id_prefix
        );

        tracker.agent_blacklist.write().insert(agent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(agent): Json<Agent>) {
        println!(
            "Removing agent with peer_id_prefix {:?}.",
            agent.peer_id_prefix
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

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct Agent {
    pub peer_id_prefix: Vec<u8>,
}
