use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(pub IndexSet<Agent>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set, Error> {
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
        .map_err(|error| {
            println!("{}", error);
            Error("Failed loading blacklisted clients.")
        })?;

        let mut agent_set = Set::new();

        for agent in agents {
            agent_set.insert(agent);
        }

        Ok(agent_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(agent): Query<Agent>) {
        tracker.agent_blacklist.write().await.insert(agent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(agent): Query<Agent>) {
        tracker.agent_blacklist.write().await.remove(&agent);
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
    pub name: String,
}
