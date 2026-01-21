use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

pub struct FreeleechTokenStore {
    inner: IndexSet<FreeleechToken>,
}

impl FreeleechTokenStore {
    pub fn new() -> FreeleechTokenStore {
        FreeleechTokenStore {
            inner: IndexSet::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<FreeleechTokenStore> {
        sqlx::query_as!(
            FreeleechToken,
            r#"
                SELECT
                    user_id as `user_id: u32`,
                    torrent_id as `torrent_id: u32`
                FROM
                    freeleech_tokens
            "#
        )
        .fetch(db)
        .try_fold(FreeleechTokenStore::new(), |mut store, token| async move {
            store.insert(token);

            Ok(store)
        })
        .await
        .context("Failed loading freeleech tokens.")
    }
}

impl Deref for FreeleechTokenStore {
    type Target = IndexSet<FreeleechToken>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for FreeleechTokenStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
