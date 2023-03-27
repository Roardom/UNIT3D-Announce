use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(IndexSet<FreeleechToken>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set, Error> {
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
        .map_err(|error| {
            println!("{}", error);
            Error("Failed loading freeleech tokens.")
        })?;

        let mut freeleech_token_set = Set::new();

        for freeleech_token in freeleech_tokens {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(token): Query<FreeleechToken>) {
        println!(
            "Inserting freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.write().await.insert(token);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(token): Query<FreeleechToken>) {
        println!(
            "Removing freeleech token with user_id {} and torrent_id {}.",
            token.user_id, token.torrent_id
        );

        tracker.freeleech_tokens.write().await.remove(&token);
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
