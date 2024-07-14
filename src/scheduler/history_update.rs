use std::{
    cmp::min,
    ops::{Deref, DerefMut},
};

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub IndexMap<Index, HistoryUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
    pub user_id: u32,
}

#[derive(Clone)]
pub struct HistoryUpdate {
    pub user_id: u32,
    pub torrent_id: u32,
    pub user_agent: String,
    pub is_active: bool,
    pub is_seeder: bool,
    pub is_immune: bool,
    pub uploaded: u64,
    pub downloaded: u64,
    pub uploaded_delta: u64,
    pub downloaded_delta: u64,
    pub credited_uploaded_delta: u64,
    pub credited_downloaded_delta: u64,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(IndexMap::new())
    }

    pub fn upsert(&mut self, new: HistoryUpdate) {
        self.entry(Index {
            torrent_id: new.torrent_id,
            user_id: new.user_id,
        })
        .and_modify(|history_update| {
            history_update.user_agent = new.user_agent.to_owned();
            history_update.is_active = new.is_active;
            history_update.is_seeder = new.is_seeder;
            history_update.uploaded = new.uploaded;
            history_update.downloaded = new.downloaded;
            history_update.uploaded_delta += new.uploaded_delta;
            history_update.downloaded_delta += new.downloaded_delta;
            history_update.credited_uploaded_delta += new.credited_uploaded_delta;
            history_update.credited_downloaded_delta += new.credited_downloaded_delta;
            history_update.completed_at = new.completed_at;
        })
        .or_insert(new);
    }

    /// Determine the max amount of history records that can be inserted at
    /// once
    const fn history_limit() -> usize {
        /// Max amount of bindings in a mysql query
        const BIND_LIMIT: usize = 65535;

        /// Number of columns being updated in the history table
        const HISTORY_COLUMN_COUNT: usize = 16;

        /// 1 extra binding is used to insert the TTL
        const EXTRA_BINDING_COUNT: usize = 1;

        (BIND_LIMIT - EXTRA_BINDING_COUNT) / HISTORY_COLUMN_COUNT
    }

    /// Take a portion of the history updates small enough to be inserted into
    /// the database.
    pub fn take_batch(&mut self) -> Queue {
        let len = self.len();

        Queue(self.drain(0..min(Queue::history_limit(), len)).collect())
    }

    /// Merge a history update batch into this history update batch
    pub fn upsert_batch(&mut self, batch: Queue) {
        for history_update in batch.values() {
            self.upsert(history_update.clone());
        }
    }

    /// Flushes history updates to the mysql db
    ///
    /// **Warning**: this function does not make sure that the query isn't too long
    /// or doesn't use too many bindings
    pub async fn flush_to_db(&self, db: &MySqlPool, seedtime_ttl: u64) -> Result<u64, sqlx::Error> {
        let len = self.len();

        if len == 0 {
            return Ok(0);
        }

        let now = Utc::now();

        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"INSERT INTO
                history(
                    user_id,
                    torrent_id,
                    agent,
                    uploaded,
                    actual_uploaded,
                    client_uploaded,
                    downloaded,
                    actual_downloaded,
                    client_downloaded,
                    seeder,
                    active,
                    seedtime,
                    immune,
                    created_at,
                    updated_at,
                    completed_at
                )
            "#,
        );

        // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
        query_builder
            // .push_values(history_updates., |mut bind, (index, history_update)| {
            .push_values(self.values(), |mut bind, history_update| {
                bind.push_bind(history_update.user_id)
                    .push_bind(history_update.torrent_id)
                    .push_bind(history_update.user_agent.as_str())
                    .push_bind(history_update.credited_uploaded_delta)
                    .push_bind(history_update.uploaded_delta)
                    .push_bind(history_update.uploaded)
                    .push_bind(history_update.credited_downloaded_delta)
                    .push_bind(history_update.downloaded_delta)
                    .push_bind(history_update.downloaded)
                    .push_bind(history_update.is_seeder)
                    .push_bind(history_update.is_active)
                    .push_bind(0)
                    .push_bind(history_update.is_immune)
                    .push_bind(now)
                    .push_bind(now)
                    .push_bind(history_update.completed_at);
            })
            .push(
                r#"
                    ON DUPLICATE KEY UPDATE
                        agent = VALUES(agent),
                        uploaded = uploaded + VALUES(uploaded),
                        actual_uploaded = actual_uploaded + VALUES(actual_uploaded),
                        client_uploaded = VALUES(client_uploaded),
                        downloaded = downloaded + VALUES(downloaded),
                        actual_downloaded = actual_downloaded + VALUES(actual_downloaded),
                        client_downloaded = VALUES(client_downloaded),
                        seedtime = IF(
                            DATE_ADD(updated_at, INTERVAL
            "#,
            )
            .push_bind(seedtime_ttl)
            .push(
                r#"
                                                                SECOND) > VALUES(updated_at) AND seeder = 1 AND active = 1 AND VALUES(seeder) = 1,
                            seedtime + TIMESTAMPDIFF(second, updated_at, VALUES(updated_at)),
                            seedtime
                        ),
                        updated_at = VALUES(updated_at),
                        seeder = VALUES(seeder),
                        active = VALUES(active),
                        immune = immune AND VALUES(immune),
                        completed_at = COALESCE(completed_at, VALUES(completed_at))
                "#,
            );

        query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected())
    }
}

impl Deref for Queue {
    type Target = IndexMap<Index, HistoryUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
