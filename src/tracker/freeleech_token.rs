use std::ops::Deref;

use dashmap::DashSet;
use sqlx::MySqlPool;

use crate::Error;

pub struct FreeleechTokenSet(DashSet<FreeleechToken>);

impl FreeleechTokenSet {
    pub fn new() -> FreeleechTokenSet {
        FreeleechTokenSet(DashSet::new())
    }

    pub async fn from_db(db: &MySqlPool) -> Result<FreeleechTokenSet, Error> {
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

        let freeleech_token_set = FreeleechTokenSet::new();

        for freeleech_token in freeleech_tokens {
            freeleech_token_set.insert(freeleech_token);
        }

        Ok(freeleech_token_set)
    }
}

impl Deref for FreeleechTokenSet {
    type Target = DashSet<FreeleechToken>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, Hash, PartialEq)]
pub struct FreeleechToken {
    pub user_id: u32,
    pub torrent_id: u32,
}
