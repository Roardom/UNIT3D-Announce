use std::{ops::Deref, sync::Arc};

use axum::extract::{Query, State};
use dashmap::DashSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use crate::Error;

use crate::tracker::Tracker;

pub struct Set(DashSet<PersonalFreeleech>);

impl Set {
    pub fn new() -> Set {
        Set(DashSet::new())
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

        let personal_freeleech_set = Set::new();

        for personal_freeleech in personal_freeleeches {
            personal_freeleech_set.insert(personal_freeleech);
        }

        Ok(personal_freeleech_set)
    }

    pub async fn upsert(
        State(tracker): State<Arc<Tracker>>,
        Query(personal_freeleech): Query<PersonalFreeleech>,
    ) {
        tracker.personal_freeleeches.insert(personal_freeleech);
    }

    pub async fn destroy(
        State(tracker): State<Arc<Tracker>>,
        Query(personal_freeleech): Query<PersonalFreeleech>,
    ) {
        tracker.personal_freeleeches.remove(&personal_freeleech);
    }
}

impl Deref for Set {
    type Target = DashSet<PersonalFreeleech>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct PersonalFreeleech {
    pub user_id: u32,
}
