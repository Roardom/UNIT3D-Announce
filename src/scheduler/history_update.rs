use super::{/*Flushable, */ Mergeable, Upsertable};
use chrono::{DateTime, Utc};

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

impl Mergeable for HistoryUpdate {
    fn merge(&mut self, new: &Self) {
        self.user_agent = new.user_agent.to_owned();
        self.is_active = new.is_active;
        self.is_seeder = new.is_seeder;
        self.uploaded = new.uploaded;
        self.downloaded = new.downloaded;
        self.uploaded_delta += new.uploaded_delta;
        self.downloaded_delta += new.downloaded_delta;
        self.credited_uploaded_delta += new.credited_uploaded_delta;
        self.credited_downloaded_delta += new.credited_downloaded_delta;
        self.completed_at = new.completed_at;
    }
}

impl Upsertable<HistoryUpdate> for super::Queue<Index, HistoryUpdate> {
    fn upsert(&mut self, new: HistoryUpdate) {
        self.records
            .entry(Index {
                torrent_id: new.torrent_id,
                user_id: new.user_id,
            })
            .and_modify(|history_update| {
                history_update.merge(&new);
            })
            .or_insert(new);
    }
}

pub struct HistoryUpdateExtraBindings {
    pub seedtime_ttl: u64,
}

// impl Flushable<HistoryUpdate> for super::Batch<Index, HistoryUpdate> {
//     type ExtraBindings = HistoryUpdateExtraBindings;

//     async fn flush_to_db(
//         &self,
//         db: &MySqlPool,
//         extra_bindings: HistoryUpdateExtraBindings,
//     ) -> Result<u64, sqlx::Error> {
//         if self.is_empty() {
//             return Ok(0);
//         }

//         let now = Utc::now();

//         let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
//             r#"INSERT INTO
//                 history(
//                     user_id,
//                     torrent_id,
//                     agent,
//                     uploaded,
//                     actual_uploaded,
//                     client_uploaded,
//                     downloaded,
//                     actual_downloaded,
//                     client_downloaded,
//                     seeder,
//                     active,
//                     seedtime,
//                     immune,
//                     created_at,
//                     updated_at,
//                     completed_at
//                 )
//             "#,
//         );

//         // Mysql 8.0.20 deprecates use of VALUES() so will have to update it eventually to use aliases instead
//         query_builder
//             // .push_values(history_updates., |mut bind, (index, history_update)| {
//             .push_values(self.values(), |mut bind, history_update| {
//                 bind.push_bind(history_update.user_id)
//                     .push_bind(history_update.torrent_id)
//                     .push_bind(history_update.user_agent.as_str())
//                     .push_bind(history_update.credited_uploaded_delta)
//                     .push_bind(history_update.uploaded_delta)
//                     .push_bind(history_update.uploaded)
//                     .push_bind(history_update.credited_downloaded_delta)
//                     .push_bind(history_update.downloaded_delta)
//                     .push_bind(history_update.downloaded)
//                     .push_bind(history_update.is_seeder)
//                     .push_bind(history_update.is_active)
//                     .push_bind(0)
//                     .push_bind(history_update.is_immune)
//                     .push_bind(now)
//                     .push_bind(now)
//                     .push_bind(history_update.completed_at);
//             })
//             .push(
//                 r#"
//                     ON DUPLICATE KEY UPDATE
//                         agent = VALUES(agent),
//                         uploaded = uploaded + VALUES(uploaded),
//                         actual_uploaded = actual_uploaded + VALUES(actual_uploaded),
//                         client_uploaded = VALUES(client_uploaded),
//                         downloaded = downloaded + VALUES(downloaded),
//                         actual_downloaded = actual_downloaded + VALUES(actual_downloaded),
//                         client_downloaded = VALUES(client_downloaded),
//                         seedtime = IF(
//                             DATE_ADD(updated_at, INTERVAL
//             "#,
//             )
//             .push_bind(extra_bindings.seedtime_ttl)
//             .push(
//                 r#"
//                                                                 SECOND) > VALUES(updated_at) AND seeder = 1 AND active = 1 AND VALUES(seeder) = 1,
//                             seedtime + TIMESTAMPDIFF(second, updated_at, VALUES(updated_at)),
//                             seedtime
//                         ),
//                         updated_at = VALUES(updated_at),
//                         seeder = VALUES(seeder),
//                         active = VALUES(active),
//                         immune = immune AND VALUES(immune),
//                         completed_at = COALESCE(completed_at, VALUES(completed_at))
//                 "#,
//             );

//         query_builder
//             .build()
//             .persistent(false)
//             .execute(db)
//             .await
//             .map(|result| result.rows_affected())
//     }
// }
