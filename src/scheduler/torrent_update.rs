use chrono::Utc;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use super::{Flushable, Mergeable, Upsertable};

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
}

#[derive(Clone)]
pub struct TorrentUpdate {
    pub torrent_id: u32,
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

impl Upsertable<TorrentUpdate> for super::Queue<Index, TorrentUpdate> {
    fn upsert(&mut self, new: TorrentUpdate) {
        self.records
            .entry(Index {
                torrent_id: new.torrent_id,
            })
            .and_modify(|torrent_update| {
                torrent_update.merge(&new);
            })
            .or_insert(new);
    }
}
impl Flushable<TorrentUpdate> for super::Batch<Index, TorrentUpdate> {
    type ExtraBindings = ();

    async fn flush_to_db(&self, db: &MySqlPool, _extra_bindings: ()) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
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
                    .push_bind(torrent_update.balance_delta)
                    .push_bind(0);
            })
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
            .execute(db)
            .await
            .map(|result| result.rows_affected())
    }
}
