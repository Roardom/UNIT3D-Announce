use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(IndexSet<PersonalFreeleech>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<Set, Error> {
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
        .map_err(|_| Error("Failed loading personal freeleeches."))?;

        let mut personal_freeleech_set = Set::new();

        for personal_freeleech in personal_freeleeches {
            personal_freeleech_set.insert(personal_freeleech);
        }

        Ok(personal_freeleech_set)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Query(personal_freeleech): Query<PersonalFreeleech>,
    ) {
        tracker
            .personal_freeleeches
            .write()
            .await
            .insert(personal_freeleech);
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Query(personal_freeleech): Query<PersonalFreeleech>,
    ) {
        tracker
            .personal_freeleeches
            .write()
            .await
            .remove(&personal_freeleech);
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
