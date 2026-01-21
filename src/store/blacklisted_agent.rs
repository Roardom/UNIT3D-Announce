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
        sqlx::query_as!(
            Agent,
            r#"
                SELECT
                    peer_id_prefix
                FROM
                    blacklist_clients
            "#
        )
        .fetch(db)
        .try_fold(BlacklistedAgentStore::new(), |mut store, agent| async {
            store.insert(agent);

            Ok(store)
        })
        .await
        .context("Failed loading blacklisted clients.")
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
