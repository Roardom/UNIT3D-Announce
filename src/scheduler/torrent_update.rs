use std::{
    cmp::min,
    ops::{Deref, DerefMut},
};

use chrono::Utc;
use indexmap::IndexMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub IndexMap<Index, TorrentUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
}

pub struct TorrentUpdate {
    pub torrent_id: u32,
    pub seeder_delta: i32,
    pub leecher_delta: i32,
    pub times_completed_delta: u32,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(IndexMap::new())
    }

    pub fn upsert(
        &mut self,
        torrent_id: u32,
        seeder_delta: i32,
        leecher_delta: i32,
        times_completed_delta: u32,
    ) {
        self.entry(Index { torrent_id })
            .and_modify(|torrent_update| {
                torrent_update.seeder_delta =
                    torrent_update.seeder_delta.saturating_add(seeder_delta);
                torrent_update.leecher_delta =
                    torrent_update.leecher_delta.saturating_add(leecher_delta);
                torrent_update.times_completed_delta = torrent_update
                    .times_completed_delta
                    .saturating_add(times_completed_delta);
            })
            .or_insert(TorrentUpdate {
                torrent_id,
                seeder_delta,
                leecher_delta,
                times_completed_delta,
            });
    }

    /// Determine the max amount of torrent records that can be inserted at
    /// once
    const fn torrent_limit() -> usize {
        /// Max amount of bindings in a mysql query
        const BIND_LIMIT: usize = 65535;

        /// Number of columns being updated in the torrent table
        const TORRENT_COLUMN_COUNT: usize = 17;

        BIND_LIMIT / TORRENT_COLUMN_COUNT
    }

    /// Take a portion of the torrent updates small enough to be inserted into
    /// the database.
    pub fn take_batch(&mut self) -> Queue {
        let len = self.len();

        Queue(self.drain(0..min(Queue::torrent_limit(), len)).collect())
    }

    /// Merge a torrent update batch into this torrent update batch
    pub fn upsert_batch(&mut self, batch: Queue) {
        for torrent_update in batch.values() {
            self.upsert(
                torrent_update.torrent_id,
                torrent_update.seeder_delta,
                torrent_update.leecher_delta,
                torrent_update.times_completed_delta,
            );
        }
    }

    /// Flushes torrent updates to the mysql db
    pub async fn flush_to_db(&self, db: &MySqlPool) -> Result<u64, sqlx::Error> {
        let len = self.len();

        if len == 0 {
            return Ok(0);
        }

        let now = Utc::now();

        // Trailing space required before the push values function
        // Leading space required after the push values function
        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                INSERT INTO
                    torrents(
                        id,
                        name,
                        description,
                        info_hash,
                        file_name,
                        num_file,
                        size,
                        seeders,
                        leechers,
                        times_completed,
                        user_id,
                        created_at,
                        updated_at,
                        type_id,
                        balance,
                        balance_offset
                    )
            "#,
        );

        query_builder
            .push_values(self.values(), |mut bind, torrent_update| {
                bind.push_bind(torrent_update.torrent_id)
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(torrent_update.seeder_delta)
                    .push_bind(torrent_update.leecher_delta)
                    .push_bind(torrent_update.times_completed_delta)
                    .push_bind(1)
                    .push_bind(now)
                    .push_bind(now)
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(0);
            })
            .push(
                r#"
                    ON DUPLICATE KEY UPDATE
                        seeders = seeders + VALUES(seeders),
                        leechers = leechers + VALUES(leechers),
                        times_completed = times_completed + VALUES(times_completed),
                        updated_at = VALUES(updated_at)
                "#,
            );

        let rows_affected_res = query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected());

        rows_affected_res
    }
}

impl Deref for Queue {
    type Target = IndexMap<Index, TorrentUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
