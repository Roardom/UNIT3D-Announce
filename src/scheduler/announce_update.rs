use std::{
    cmp::min,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use sqlx::{MySql, QueryBuilder};

use crate::{
    announce::Event,
    tracker::{peer::PeerId, Tracker},
};

pub struct Queue(pub Vec<AnnounceUpdate>);

#[derive(Clone)]
pub struct AnnounceUpdate {
    pub user_id: u32,
    pub torrent_id: u32,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub corrupt: Option<u64>,
    pub peer_id: PeerId,
    pub port: u16,
    pub numwant: u16,
    pub created_at: DateTime<Utc>,
    pub event: Event,
    pub key: Option<String>,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(Vec::new())
    }

    pub fn upsert(&mut self, new: AnnounceUpdate) {
        self.push(new);
    }

    /// Determine the max amount of announce records that can be inserted at
    /// once
    const fn announce_limit() -> usize {
        /// Max amount of bindings in a mysql query
        const BIND_LIMIT: usize = 65535;

        /// Number of columns being updated in the announce table
        const ANNOUNCE_COLUMN_COUNT: usize = 12;

        BIND_LIMIT / ANNOUNCE_COLUMN_COUNT
    }

    /// Take a portion of the announce updates small enough to be inserted into
    /// the database.
    pub fn take_batch(&mut self) -> Queue {
        let len = self.len();

        Queue(self.drain(0..min(Queue::announce_limit(), len)).collect())
    }

    /// Merge a announce update batch into this announce update batch
    pub fn upsert_batch(&mut self, batch: Queue) {
        self.extend(batch.0);
    }

    /// Flushes announce updates to the mysql db
    pub async fn flush_to_db(&self, tracker: &Arc<Tracker>) -> Result<u64, sqlx::Error> {
        let len = self.len();

        if len == 0 {
            return Ok(0);
        }

        // Trailing space required before the push values function
        // Leading space required after the push values function
        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                INSERT INTO
                    announces(
                        user_id,
                        torrent_id,
                        uploaded,
                        downloaded,
                        `left`,
                        corrupt,
                        peer_id,
                        port,
                        numwant,
                        created_at,
                        event,
                        `key`
                    )
            "#,
        );

        query_builder.push_values(self.iter(), |mut bind, announce_update| {
            bind.push_bind(announce_update.user_id)
                .push_bind(announce_update.torrent_id)
                .push_bind(announce_update.uploaded)
                .push_bind(announce_update.downloaded)
                .push_bind(announce_update.left)
                .push_bind(announce_update.corrupt.unwrap_or(0))
                .push_bind(announce_update.peer_id.to_vec())
                .push_bind(announce_update.port)
                .push_bind(announce_update.numwant)
                .push_bind(announce_update.created_at)
                .push_bind(announce_update.event.to_string());

            if let Some(key) = &announce_update.key {
                bind.push_bind(key);
            } else {
                bind.push_bind("");
            }
        });

        query_builder
            .build()
            .persistent(false)
            .execute(&tracker.pool)
            .await
            .map(|result| result.rows_affected())
    }
}

impl Deref for Queue {
    type Target = Vec<AnnounceUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
