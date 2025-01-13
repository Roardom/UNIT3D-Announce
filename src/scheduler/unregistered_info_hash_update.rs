use std::sync::Arc;

use crate::tracker::{torrent::InfoHash, Tracker};
use chrono::{DateTime, Utc};
use sqlx::{MySql, QueryBuilder};

use super::{Flushable, Mergeable};

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub user_id: u32,
    pub info_hash: InfoHash,
}

#[derive(Clone)]
pub struct UnregisteredInfoHashUpdate {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Mergeable for UnregisteredInfoHashUpdate {
    fn merge(&mut self, new: &Self) {
        if new.updated_at > self.updated_at {
            self.updated_at = new.updated_at;
        }

        self.created_at = std::cmp::min(self.created_at, new.created_at);
    }
}

impl Flushable<UnregisteredInfoHashUpdate> for super::Batch<Index, UnregisteredInfoHashUpdate> {
    async fn flush_to_db(&self, tracker: &Arc<Tracker>) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                INSERT INTO
                    unregistered_info_hashes(
                        user_id,
                        info_hash,
                        created_at,
                        updated_at
                    )
            "#,
        );

        query_builder
            // Trailing space required before the push values function
            // Leading space required after the push values function
            .push_values(
                self.iter(),
                |mut bind, (index, unregistered_info_hash_update)| {
                    bind.push_bind(index.user_id)
                        .push_bind(index.info_hash.to_vec())
                        .push_bind(unregistered_info_hash_update.created_at)
                        .push_bind(unregistered_info_hash_update.updated_at);
                },
            )
            // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
            // However, Mariadb doesn't yet support aliases
            .push(
                r#"
                ON DUPLICATE KEY UPDATE
                    updated_at = VALUES(updated_at)
            "#,
            );

        query_builder
            .build()
            .persistent(false)
            .execute(&tracker.pool)
            .await
            .map(|result| result.rows_affected())
    }
}
