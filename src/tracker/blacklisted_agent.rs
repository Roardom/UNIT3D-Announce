use std::ops::Deref;

use dashmap::DashSet;
use sqlx::MySqlPool;

use crate::Error;

pub struct AgentSet(pub DashSet<Agent>);

impl AgentSet {
    pub fn new() -> AgentSet {
        AgentSet(DashSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<AgentSet, Error> {
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

        let agent_set = AgentSet::new();

        for agent in agents {
            agent_set.insert(agent);
        }

        Ok(agent_set)
    }
}

impl Deref for AgentSet {
    type Target = DashSet<Agent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Hash, PartialEq)]
pub struct Agent {
    pub name: String,
}
