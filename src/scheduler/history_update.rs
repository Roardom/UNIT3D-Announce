use std::{
    cmp::min,
    ops::{Deref, DerefMut},
};

use chrono::{DateTime, Utc};
use compact_str::CompactString;
use indexmap::IndexMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

pub struct Queue(pub IndexMap<Index, HistoryUpdate>);

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
    pub user_id: u32,
}

pub struct HistoryUpdate {
    pub user_id: u32,
    pub torrent_id: u32,
    pub user_agent: CompactString,
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

    pub fn upsert(
        &mut self,
        user_id: u32,
        torrent_id: u32,
        user_agent: CompactString,
        credited_uploaded_delta: u64,
        uploaded_delta: u64,
        uploaded: u64,
        credited_downloaded_delta: u64,
        downloaded_delta: u64,
        downloaded: u64,
        is_seeder: bool,
        is_active: bool,
        is_immune: bool,
        completed_at: Option<DateTime<Utc>>,
    ) {
        self.entry(Index {
            torrent_id,
            user_id,
        })
        .and_modify(|history_update| {
            history_update.user_agent = user_agent.to_owned();
            history_update.is_active = is_active;
            history_update.is_seeder = is_seeder;
            history_update.uploaded = uploaded;
            history_update.downloaded = downloaded;
            history_update.uploaded_delta += uploaded_delta;
            history_update.downloaded_delta += downloaded_delta;
            history_update.credited_uploaded_delta += credited_uploaded_delta;
            history_update.credited_downloaded_delta += credited_downloaded_delta;
            history_update.completed_at = completed_at;
        })
        .or_insert(HistoryUpdate {
            user_id,
            torrent_id,
            user_agent: user_agent.to_owned(),
            is_active,
            is_seeder,
            is_immune,
            uploaded,
            downloaded,
            uploaded_delta,
            downloaded_delta,
            credited_uploaded_delta,
            credited_downloaded_delta,
            completed_at,
        });
    }

    /// Flushes history updates to the mysql db
    pub async fn flush_to_db(&mut self, db: &MySqlPool, seedtime_ttl: u64) {
        let len = self.len();

        if len == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_HISTORY_COLUMNS: usize = 3;
        const HISTORY_LIMIT: usize = BIND_LIMIT / NUM_HISTORY_COLUMNS;

        let now = Utc::now();

        let history_updates = self.split_off(len - min(HISTORY_LIMIT, len));

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
            .push_values(&history_updates, |mut bind, (_index, history_update)| {
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
                                                                SECOND) > VALUES(updated_at) AND seeder = 1 AND VALUES(seeder) = 1,
                            seedtime + TIMESTAMPDIFF(second, updated_at, VALUES(updated_at)),
                            seedtime
                        ),
                        updated_at = VALUES(updated_at),
                        seeder = VALUES(seeder),
                        active = VALUES(active),
                        completed_at = COALESCE(completed_at, VALUES(completed_at))
                "#,
            );

        let result = query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected());

        match result {
            Ok(_) => (),
            Err(e) => {
                println!("History update failed: {}", e);

                for (_index, history_update) in history_updates.iter() {
                    self.upsert(
                        history_update.user_id,
                        history_update.torrent_id,
                        history_update.user_agent.to_owned(),
                        history_update.credited_uploaded_delta,
                        history_update.uploaded_delta,
                        history_update.uploaded,
                        history_update.credited_downloaded_delta,
                        history_update.downloaded_delta,
                        history_update.downloaded,
                        history_update.is_seeder,
                        history_update.is_active,
                        history_update.is_immune,
                        history_update.completed_at,
                    );
                }
            }
        }
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
