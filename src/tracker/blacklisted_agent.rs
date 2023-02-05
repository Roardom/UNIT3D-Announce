use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use dashmap::DashSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(pub DashSet<Agent>);

impl Set {
    pub fn new() -> Set {
        Set(DashSet::new())
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
        .map_err(|_| Error("Failed loading blacklisted clients."))?;

        let agent_set = Set::new();

        for agent in agents {
            agent_set.insert(agent);
        }

        Ok(agent_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(agent): Query<Agent>) {
        tracker.agent_blacklist.insert(agent);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(agent): Query<Agent>) {
        tracker.agent_blacklist.remove(&agent);
    }
}

impl Deref for Set {
    type Target = DashSet<Agent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct Agent {
    pub name: String,
}
