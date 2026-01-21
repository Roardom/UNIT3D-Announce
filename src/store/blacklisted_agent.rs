use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::Json;
use axum::extract::State;
use futures_util::TryStreamExt;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};
use tracing::info;

use crate::state::AppState;

pub struct Set(pub IndexSet<Agent>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let mut agents = sqlx::query_as!(
            Agent,
            r#"
                SELECT
                    peer_id_prefix
                FROM
                    blacklist_clients
            "#
        )
        .fetch(db);

        let mut agent_set = Set::new();

        while let Some(agent) = agents
            .try_next()
            .await
            .context("Failed loading blacklisted clients.")?
        {
            agent_set.insert(agent);
        }

        Ok(agent_set)
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
    #[serde(with = "serde_bytes")]
    pub peer_id_prefix: Vec<u8>,
}

pub async fn upsert(State(state): State<Arc<AppState>>, Json(agent): Json<Agent>) {
    info!(
        "Inserting agent with peer_id_prefix {} ({:?}).",
        String::from_utf8_lossy(&agent.peer_id_prefix),
        agent.peer_id_prefix,
    );

    state.stores.agent_blacklist.write().insert(agent);
}

pub async fn destroy(State(state): State<Arc<AppState>>, Json(agent): Json<Agent>) {
    info!(
        "Removing agent with peer_id_prefix {} ({:?}).",
        String::from_utf8_lossy(&agent.peer_id_prefix),
        agent.peer_id_prefix,
    );

    state.stores.agent_blacklist.write().swap_remove(&agent);
}
