use std::sync::Arc;

use crate::tracker::{torrent::InfoHash, Tracker};
use chrono::{DateTime, Utc};
use sqlx::{MySql, QueryBuilder};

use super::{Flushable, Mergeable, Upsertable};

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub user_id: u32,
    pub info_hash: InfoHash,
}

#[derive(Clone)]
pub struct UnregisteredInfoHashUpdate {
    pub user_id: u32,
    pub info_hash: InfoHash,
    pub updated_at: DateTime<Utc>,
}

impl Mergeable for UnregisteredInfoHashUpdate {
    fn merge(&mut self, new: &Self) {
        if new.updated_at > self.updated_at {
            self.updated_at = new.updated_at;
        }
    }
}

impl Upsertable<UnregisteredInfoHashUpdate> for super::Queue<Index, UnregisteredInfoHashUpdate> {
    fn upsert(&mut self, new: UnregisteredInfoHashUpdate) {
        self.records
            .entry(Index {
                user_id: new.user_id,
                info_hash: new.info_hash,
            })
            .and_modify(|unregistered_info_hash_update| {
                unregistered_info_hash_update.merge(&new);
            })
            .or_insert(new);
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
            .push_values(self.values(), |mut bind, unregistered_info_hash_update| {
                bind.push_bind(unregistered_info_hash_update.user_id)
                    .push_bind(unregistered_info_hash_update.info_hash.to_vec())
                    .push_bind(unregistered_info_hash_update.updated_at)
                    .push_bind(unregistered_info_hash_update.updated_at);
            })
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
