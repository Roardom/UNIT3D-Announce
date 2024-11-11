use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::Json;
use diesel::deserialize::Queryable;
use diesel::Selectable;
use indexmap::IndexSet;
use serde::Deserialize;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

use super::Db;

pub struct Set(IndexSet<FreeleechToken>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &Db) -> Result<Set> {
        use crate::schema::freeleech_tokens;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let freeleech_tokens_data: Vec<FreeleechToken> = freeleech_tokens::table
            .select(FreeleechToken::as_select())
            .load(&mut db.get().await?)
            .await
            .context("Failed loading freeleech tokens.")?;

        let mut freeleech_token_set = Set::new();

        for freeleech_token in freeleech_tokens_data {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Json(token): Json<FreeleechToken>) {
        println!(
            "Inserting freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.write().insert(token);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Json(token): Json<FreeleechToken>) {
        println!(
            "Removing freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.write().swap_remove(&token);
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

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::freeleech_tokens)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
