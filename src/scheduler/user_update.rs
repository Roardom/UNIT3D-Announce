use sqlx::{MySql, MySqlPool, QueryBuilder};

use super::{Flushable, Mergeable, Upsertable};

#[derive(Eq, Hash, PartialEq)]
pub struct Index {
    pub user_id: u32,
}

// TODO: Ideally unit3d should have `num_seeding` and `num_leeching` columns
// on the user table so that the navbar doesn't query the history table.
// If those columns existed, they should be updated too.
#[derive(Clone)]
pub struct UserUpdate {
    pub user_id: u32,
    pub uploaded_delta: u64,
    pub downloaded_delta: u64,
}

impl Mergeable for UserUpdate {
    fn merge(&mut self, new: &Self) {
        self.uploaded_delta = self.uploaded_delta.saturating_add(new.uploaded_delta);
        self.downloaded_delta = self.downloaded_delta.saturating_add(new.downloaded_delta);
    }
}

impl Upsertable<UserUpdate> for super::Queue<Index, UserUpdate> {
    fn upsert(&mut self, new: UserUpdate) {
        self.records
            .entry(Index {
                user_id: new.user_id,
            })
            .and_modify(|user_update| {
                user_update.merge(&new);
            })
            .or_insert(new);
    }
}
impl Flushable<UserUpdate> for super::Batch<Index, UserUpdate> {
    type ExtraBindings = ();

    async fn flush_to_db(&self, db: &MySqlPool, _extra_bindings: ()) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
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

        query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected())
    }
}
