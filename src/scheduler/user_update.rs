use std::{
    cmp::min,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub IndexMap<Index, UserUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub user_id: u32,
}

// TODO: Ideally unit3d should have `num_seeding` and `num_leeching` columns
// on the user table so that the navbar doesn't query the history table.
// If those columns existed, they should be updated too.
pub struct UserUpdate {
    pub user_id: u32,
    pub uploaded_delta: u64,
    pub downloaded_delta: u64,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(IndexMap::new())
    }

    pub fn upsert(&mut self, user_id: u32, uploaded_delta: u64, downloaded_delta: u64) {
        self.insert(
            Index { user_id },
            UserUpdate {
                user_id,
                uploaded_delta,
                downloaded_delta,
            },
        );
    }

    /// Determine the max amount of user records that can be inserted at
    /// once
    const fn user_limit() -> usize {
        /// Max amount of bindings in a mysql query
        const BIND_LIMIT: usize = 65535;

        /// Number of columns being updated in the user table
        const USER_COLUMN_COUNT: usize = 9;

        BIND_LIMIT / USER_COLUMN_COUNT
    }

    /// Take a portion of the user updates small enough to be inserted into
    /// the database.
    pub fn take_batch(&mut self) -> Queue {
        let len = self.len();

        Queue(self.split_off(len - min(Queue::user_limit(), len)))
    }

    /// Merge a torrent update batch into this torrent update batch
    pub fn upsert_batch(&mut self, batch: Queue) -> () {
        for user_update in batch.values() {
            self.upsert(
                user_update.user_id,
                user_update.uploaded_delta,
                user_update.downloaded_delta,
            );
        }
    }

    /// Flushes user updates to the mysql db
    pub async fn flush_to_db(&self, db: &MySqlPool) -> Result<u64, sqlx::Error> {
        let len = self.len();

        if len == 0 {
            return Ok(0);
        }

        // Trailing space required before the push values function
        // Leading space required after the push values function
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
            .push_values(self.values(), |mut bind, user_update| {
                bind.push_bind(user_update.user_id)
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind(0)
                    .push_bind(user_update.uploaded_delta)
                    .push_bind(user_update.downloaded_delta)
                    .push_bind("");
            })
            .push(
                r#"
                    ON DUPLICATE KEY UPDATE
                        uploaded = uploaded + VALUES(uploaded),
                        downloaded = downloaded + values(downloaded)
                "#,
            );

        let result = query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected());

        return result;
    }
}

impl Deref for Queue {
    type Target = IndexMap<Index, UserUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
