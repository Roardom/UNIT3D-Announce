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

pub struct Set(IndexSet<FreeleechToken>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
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

        let mut freeleech_token_set = Set::new();

        while let Some(freeleech_token) = freeleech_tokens
            .try_next()
            .await
            .context("Failed loading freeleech tokens.")?
        {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
    }

    pub async fn upsert(State(state): State<Arc<AppState>>, Json(token): Json<FreeleechToken>) {
        info!(
            "Inserting freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        state.stores.freeleech_tokens.write().insert(token);
    }

    pub async fn destroy(State(state): State<Arc<AppState>>, Json(token): Json<FreeleechToken>) {
        info!(
            "Removing freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        state.stores.freeleech_tokens.write().swap_remove(&token);
    }
}

impl Deref for Set {
    type Target = IndexSet<FreeleechToken>;

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
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
