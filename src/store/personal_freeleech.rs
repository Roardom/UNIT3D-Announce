use std::ops::Deref;
use std::ops::DerefMut;

use futures_util::TryStreamExt;
use indexmap::IndexSet;
use serde::Deserialize;
use sqlx::MySqlPool;

use anyhow::{Context, Result};

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
        sqlx::query_as!(
            PersonalFreeleech,
            r#"
                SELECT
                    user_id as `user_id: u32`
                FROM
                    personal_freeleeches
            "#
        )
        .fetch(db)
        .try_fold(
            PersonalFreeleechStore::new(),
            |mut store, personal_freeleech| async move {
                store.insert(personal_freeleech);

                Ok(store)
            },
        )
        .await
        .context("Failed loading personal freeleeches.")
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
