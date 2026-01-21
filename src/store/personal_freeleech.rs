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

pub struct PersonalFreeleechStore {
    inner: IndexSet<PersonalFreeleech>,
}

impl PersonalFreeleechStore {
    pub fn new() -> PersonalFreeleechStore {
        PersonalFreeleechStore {
            inner: IndexSet::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<PersonalFreeleechStore> {
        let mut personal_freeleeches = sqlx::query_as!(
            PersonalFreeleech,
            r#"
                SELECT
                    user_id as `user_id: u32`
                FROM
                    personal_freeleeches
            "#
        )
        .fetch(db);

        let mut personal_freeleech_set = PersonalFreeleechStore::new();

        while let Some(personal_freeleech) = personal_freeleeches
            .try_next()
            .await
            .context("Failed loading personal freeleeches.")?
        {
            personal_freeleech_set.insert(personal_freeleech);
        }

        Ok(personal_freeleech_set)
    }
}

impl Deref for PersonalFreeleechStore {
    type Target = IndexSet<PersonalFreeleech>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PersonalFreeleechStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct PersonalFreeleech {
    pub user_id: u32,
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Json(personal_freeleech): Json<PersonalFreeleech>,
) {
    info!(
        "Inserting personal freeleech with user_id {}.",
        personal_freeleech.user_id
    );

    state
        .stores
        .personal_freeleeches
        .write()
        .insert(personal_freeleech);
}

pub async fn destroy(
    State(state): State<Arc<AppState>>,
    Json(personal_freeleech): Json<PersonalFreeleech>,
) {
    info!(
        "Removing personal freeleech with user_id {}.",
        personal_freeleech.user_id
    );

    state
        .stores
        .personal_freeleeches
        .write()
        .swap_remove(&personal_freeleech);
}
