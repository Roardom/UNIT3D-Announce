use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use ahash::RandomState;
use axum::extract::State;
use axum::Json;
use scc::HashIndex;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub struct Set(pub HashIndex<Agent, (), RandomState>);

impl Set {
    pub fn new() -> Set {
        Set(HashIndex::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let agents = sqlx::query_as!(
            Agent,
            r#"
                SELECT
                    name
                FROM
                    blacklist_clients
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading blacklisted clients.")?;

        let agent_set = Set::new();

        for agent in agents {
            agent_set.entry(agent).or_insert(());
        }

        Ok(agent_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Json(agent): Json<Agent>) {
        println!("Inserting agent with name {}.", agent.name);

        tracker.agent_blacklist.entry(agent).or_insert(());
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(agent): Json<Agent>) {
        println!("Removing agent with name {}.", agent.name);

        tracker.agent_blacklist.remove(&agent);
    }
}

impl Deref for Set {
    type Target = HashIndex<Agent, (), RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Set {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Eq, Deserialize, Hash, PartialEq)]
pub struct Agent {
    pub name: String,
}
