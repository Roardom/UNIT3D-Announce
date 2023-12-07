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

pub struct Set(HashIndex<FreeleechToken, (), RandomState>);

impl Set {
    pub fn new() -> Set {
        Set(HashIndex::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let freeleech_tokens = sqlx::query_as!(
            FreeleechToken,
            r#"
                SELECT
                    user_id as `user_id: u32`,
                    torrent_id as `torrent_id: u32`
                FROM
                    freeleech_tokens
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading freeleech tokens.")?;

        let freeleech_token_set = Set::new();

        for freeleech_token in freeleech_tokens {
            freeleech_token_set.entry(freeleech_token).or_insert(());
        }

        Ok(freeleech_token_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Json(token): Json<FreeleechToken>) {
        println!(
            "Inserting freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.entry(token).or_insert(());
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(token): Json<FreeleechToken>) {
        println!(
            "Removing freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.remove(&token);
    }
}

impl Deref for Set {
    type Target = HashIndex<FreeleechToken, (), RandomState>;

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
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
