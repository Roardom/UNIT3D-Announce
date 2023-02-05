use std::ops::Deref;

use chrono::Utc;
use dashmap::DashMap;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::tracker::peer::UserAgent;

pub struct Queue(pub DashMap<Index, HistoryUpdate>);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Index {
    pub torrent_id: u32,
    pub user_id: u32,
}

#[derive(Clone, Copy)]
pub struct HistoryUpdate {
    pub user_id: u32,
    pub torrent_id: u32,
    pub user_agent: UserAgent,
    pub is_active: bool,
    pub is_seeder: bool,
    pub is_immune: bool,
    pub uploaded: u64,
    pub downloaded: u64,
    pub uploaded_delta: u64,
    pub downloaded_delta: u64,
    pub credited_uploaded_delta: u64,
    pub credited_downloaded_delta: u64,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(DashMap::new())
    }

    pub fn upsert(
        &self,
        user_id: u32,
        torrent_id: u32,
        user_agent: UserAgent,
        credited_uploaded_delta: u64,
        uploaded_delta: u64,
        uploaded: u64,
        credited_downloaded_delta: u64,
        downloaded_delta: u64,
        downloaded: u64,
        is_seeder: bool,
        is_active: bool,
        is_immune: bool,
    ) {
        self.entry(Index {
            torrent_id,
            user_id,
        })
        .and_modify(|history_update| {
            history_update.user_agent = user_agent;
            history_update.is_active = is_active;
            history_update.is_seeder = is_seeder;
            history_update.uploaded = uploaded;
            history_update.downloaded = downloaded;
            history_update.uploaded_delta += uploaded_delta;
            history_update.downloaded_delta += downloaded_delta;
            history_update.credited_uploaded_delta += credited_uploaded_delta;
            history_update.credited_downloaded_delta += credited_downloaded_delta;
        })
        .or_insert(HistoryUpdate {
            user_id,
            torrent_id,
            user_agent,
            is_active,
            is_seeder,
            is_immune,
            uploaded,
            downloaded,
            uploaded_delta,
            downloaded_delta,
            credited_uploaded_delta,
            credited_downloaded_delta,
        });
    }

    /// Flushes history updates to the mysql db
    pub async fn flush_to_db(&self, db: &MySqlPool, seedtime_ttl: u64) {
        if self.len() == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_HISTORY_COLUMNS: usize = 3;
        const HISTORY_LIMIT: usize = BIND_LIMIT / NUM_HISTORY_COLUMNS;

        let now = Utc::now();

        let mut history_updates: Vec<_> = vec![];

        for _ in 0..std::cmp::min(HISTORY_LIMIT, self.len()) {
            let history_update = *self.iter().next().unwrap();
            self.remove(&Index {
                torrent_id: history_update.torrent_id,
                user_id: history_update.user_id,
            });
            history_updates.push(history_update);
        }

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
                    updated_at
                )
            "#,
        );

        // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
        query_builder
            .push_values(history_updates.clone(), |mut bind, history_update| {
                bind.push_bind(history_update.user_id)
                    .push_bind(history_update.torrent_id)
                    .push("TRIM(")
                    .push_bind_unseparated(history_update.user_agent.to_vec())
                    .push_unseparated(")")
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
                    .push_bind(now);
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
                        active = VALUES(active)
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
                history_updates.into_iter().for_each(|history_update| {
                    self.upsert(
                        history_update.user_id,
                        history_update.torrent_id,
                        history_update.user_agent,
                        history_update.credited_uploaded_delta,
                        history_update.uploaded_delta,
                        history_update.uploaded,
                        history_update.credited_downloaded_delta,
                        history_update.downloaded_delta,
                        history_update.downloaded,
                        history_update.is_seeder,
                        history_update.is_active,
                        history_update.is_immune,
                    );
                });
            }
        }
    }
}

impl Deref for Queue {
    type Target = DashMap<Index, HistoryUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
