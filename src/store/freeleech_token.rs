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
        let mut freeleech_tokens = sqlx::query_as!(
            FreeleechToken,
            r#"
                SELECT
                    user_id as `user_id: u32`,
                    torrent_id as `torrent_id: u32`
                FROM
                    freeleech_tokens
            "#
        )
        .fetch(db);

        let mut freeleech_token_set = FreeleechTokenStore::new();

        while let Some(freeleech_token) = freeleech_tokens
            .try_next()
            .await
            .context("Failed loading freeleech tokens.")?
        {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
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
