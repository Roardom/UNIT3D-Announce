use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use dashmap::DashSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(DashSet<FreeleechToken>);

impl Set {
    pub fn new() -> Set {
        Set(DashSet::new())
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
        .map_err(|_| Error("Failed loading freeleech tokens."))?;

        let freeleech_token_set = Set::new();

        for freeleech_token in freeleech_tokens {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
    }

    pub async fn upsert(State(tracker): State<Arc<Tracker>>, Query(token): Query<FreeleechToken>) {
        tracker.freeleech_tokens.insert(token);
    }

    pub async fn destroy(State(tracker): State<Arc<Tracker>>, Query(token): Query<FreeleechToken>) {
        tracker.freeleech_tokens.remove(&token);
    }
}

impl Deref for Set {
    type Target = DashSet<FreeleechToken>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
