use std::sync::Arc;

use chrono::Utc;
use sqlx::{MySql, QueryBuilder};

use crate::tracker::Tracker;

use super::{Flushable, Mergeable};

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
}

#[derive(Clone)]
pub struct TorrentUpdate {
    pub seeder_delta: i32,
    pub leecher_delta: i32,
    pub times_completed_delta: u32,
    pub balance_delta: i64,
}

impl Mergeable for TorrentUpdate {
    fn merge(&mut self, new: &Self) {
        self.seeder_delta = self.seeder_delta.saturating_add(new.seeder_delta);
        self.leecher_delta = self.leecher_delta.saturating_add(new.leecher_delta);
        self.times_completed_delta = self
            .times_completed_delta
            .saturating_add(new.times_completed_delta);
        self.balance_delta = self.balance_delta.saturating_add(new.balance_delta);
    }
}

impl Flushable<TorrentUpdate> for super::Batch<Index, TorrentUpdate> {
    async fn flush_to_db(&self, tracker: &Arc<Tracker>) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
            return Ok(0);
        }

        let now = Utc::now();

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
                        balance,
                        balance_offset
                    )
            "#,
        );

        query_builder
            // Trailing space required before the push values function
            // Leading space required after the push values function
            .push_values(self.iter(), |mut bind, (index, torrent_update)| {
                bind.push_bind(index.torrent_id)
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
                    .push_bind(torrent_update.balance_delta)
                    .push_bind(0);
            })
            // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
            // However, Mariadb doesn't yet support aliases
            .push(
                r#"
                    ON DUPLICATE KEY UPDATE
                        seeders = seeders + VALUES(seeders),
                        leechers = leechers + VALUES(leechers),
                        times_completed = times_completed + VALUES(times_completed),
                        updated_at = VALUES(updated_at),
                        balance = balance + VALUES(balance)
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
