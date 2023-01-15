use std::ops::Deref;

use dashmap::DashSet;
use sqlx::MySqlPool;

use crate::Error;

pub struct PersonalFreeleechSet(DashSet<PersonalFreeleech>);

impl PersonalFreeleechSet {
    pub fn new() -> PersonalFreeleechSet {
        PersonalFreeleechSet(DashSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<PersonalFreeleechSet, Error> {
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

        let personal_freeleech_set = PersonalFreeleechSet::new();

        for personal_freeleech in personal_freeleeches {
            personal_freeleech_set.insert(personal_freeleech);
        }

        Ok(personal_freeleech_set)
    }
}

impl Deref for PersonalFreeleechSet {
    type Target = DashSet<PersonalFreeleech>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Hash, PartialEq)]
pub struct PersonalFreeleech {
    pub user_id: u32,
}
