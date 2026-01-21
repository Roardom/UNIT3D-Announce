use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct BlacklistedAgentStore {
    inner: IndexSet<Agent>,
}

impl BlacklistedAgentStore {
    pub fn new() -> BlacklistedAgentStore {
        BlacklistedAgentStore {
            inner: IndexSet::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<BlacklistedAgentStore> {
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

        let mut agent_set = BlacklistedAgentStore::new();

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

impl Deref for BlacklistedAgentStore {
    type Target = IndexSet<Agent>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BlacklistedAgentStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct Agent {
    #[serde(with = "serde_bytes")]
    pub peer_id_prefix: Vec<u8>,
}
