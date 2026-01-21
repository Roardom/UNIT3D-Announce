use std::sync::Arc;

use sqlx::{MySql, QueryBuilder};

use crate::state::AppState;

use super::{Flushable, Mergeable};

// Fields must be in same order as database primary key
#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Index {
    pub user_id: u32,
}

// TODO: Ideally unit3d should have `num_seeding` and `num_leeching` columns
// on the user table so that the navbar doesn't query the history table.
// If those columns existed, they should be updated too.
#[derive(Clone)]
pub struct UserUpdate {
    pub uploaded_delta: u64,
    pub downloaded_delta: u64,
}

impl Mergeable for UserUpdate {
    fn merge(&mut self, new: &Self) {
        self.uploaded_delta = self.uploaded_delta.saturating_add(new.uploaded_delta);
        self.downloaded_delta = self.downloaded_delta.saturating_add(new.downloaded_delta);
    }
}

impl Flushable<UserUpdate> for super::Batch<Index, UserUpdate> {
    async fn flush_to_db(&self, state: &Arc<AppState>) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                INSERT INTO
                    users(
                        id,
                        username,
                        email,
                        password,
                        passkey,
                        group_id,
                        uploaded,
                        downloaded,
                        rsskey
                    )
            "#,
        );

        query_builder
            // Trailing space required before the push values function
            // Leading space required after the push values function
            .push_values(self.iter(), |mut bind, (index, user_update)| {
                bind.push_bind(index.user_id)
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind(0)
                    .push_bind(user_update.uploaded_delta)
                    .push_bind(user_update.downloaded_delta)
                    .push_bind("");
            })
            // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
            // However, Mariadb doesn't yet support aliases
            .push(
                r#"
                    ON DUPLICATE KEY UPDATE
                        uploaded = uploaded + VALUES(uploaded),
                        downloaded = downloaded + values(downloaded)
                "#,
            );

        query_builder
            .build()
            .persistent(false)
            .execute(&state.pool)
            .await
            .map(|result| result.rows_affected())
    }
}
