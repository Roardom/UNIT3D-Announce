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

pub struct Set(IndexSet<PersonalFreeleech>);

impl Set {
    pub fn new() -> Set {
        Set(IndexSet::new())
    }

    pub async fn from_db(db: &Db) -> Result<Set> {
        use crate::schema::personal_freeleeches;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let personal_freeleeches_data = personal_freeleeches::table
            .select(PersonalFreeleech::as_select())
            .load(&mut db.get().await?)
            .await
            .context("Failed loading personal_freeleeches.")?;

        let mut personal_freeleech_set = Set::new();

        for personal_freeleech in personal_freeleeches_data {
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

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::personal_freeleeches)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[derive(Eq, Deserialize, Hash, PartialEq)]
pub struct PersonalFreeleech {
    pub user_id: u32,
}
