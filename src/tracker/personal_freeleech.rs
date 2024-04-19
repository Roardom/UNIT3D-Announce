use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::State;
use axum::Json;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::tracker::Tracker;

pub struct Set(IndexSet<PersonalFreeleech>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set> {
        let personal_freeleeches = sqlx::query_as!(
            PersonalFreeleech,
            r#"
                SELECT
                    user_id as `user_id: u32`
                FROM
                    personal_freeleeches
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed loading personal freeleeches.")?;

        let mut personal_freeleech_set = Set::new();

        for personal_freeleech in personal_freeleeches {
            personal_freeleech_set.insert(personal_freeleech);
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
            .write()
            .insert(personal_freeleech);
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Json(personal_freeleech): Json<PersonalFreeleech>,
    ) {
        println!(
            "Removing personal freeleech with user_id {}.",
            personal_freeleech.user_id
        );

        tracker
            .personal_freeleeches
            .write()
            .swap_remove(&personal_freeleech);
    }
}

impl Deref for Set {
    type Target = IndexSet<PersonalFreeleech>;

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
pub struct PersonalFreeleech {
    pub user_id: u32,
}
