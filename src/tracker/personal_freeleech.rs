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

pub struct Set(HashIndex<PersonalFreeleech, (), RandomState>);

impl Set {
    pub fn new() -> Set {
        Set(HashIndex::with_hasher(RandomState::new()))
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let personal_freeleeches = sqlx::query_as!(
            PersonalFreeleech,
            r#"
                SELECT
                    user_id as `user_id: u32`
                FROM
                    personal_freeleech
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading personal freeleeches.")?;

        let personal_freeleech_set = Set::new();

        for personal_freeleech in personal_freeleeches {
            personal_freeleech_set
                .entry(personal_freeleech)
                .or_insert(());
        }

        Ok(personal_freeleech_set)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Json(personal_freeleech): Json<PersonalFreeleech>,
    ) {
        println!(
            "Inserting personal freeleech with user_id {}.",
            personal_freeleech.user_id
        );

        tracker
            .personal_freeleeches
            .entry(personal_freeleech)
            .or_insert(());
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(personal_freeleech): Json<PersonalFreeleech>,
    ) {
        println!(
            "Removing personal freeleech with user_id {}.",
            personal_freeleech.user_id
        );

        tracker.personal_freeleeches.remove(&personal_freeleech);
    }
}

impl Deref for Set {
    type Target = HashIndex<PersonalFreeleech, (), RandomState>;

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
pub struct PersonalFreeleech {
    pub user_id: u32,
}
