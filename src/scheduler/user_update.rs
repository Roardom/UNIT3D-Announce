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
#[derive(Clone, Copy)]
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

    /// Flushes user updates to the mysql db
    pub async fn flush_to_db(&mut self, db: &MySqlPool) {
        let len = self.len();

        if len == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_USER_COLUMNS: usize = 9;
        const USER_LIMIT: usize = BIND_LIMIT / NUM_USER_COLUMNS;

        let mut user_updates: Vec<UserUpdate> = vec![];

        user_updates.extend(self.split_off(len - min(USER_LIMIT, len)).values());

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
            .push_values(user_updates.clone(), |mut bind, user_update| {
                bind.push_bind(user_update.user_id)
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind("")
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(0)
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

        match result {
            Ok(_) => (),
            Err(e) => {
                println!("User update failed: {}", e);
                user_updates.into_iter().for_each(|user_update| {
                    self.upsert(
                        user_update.user_id,
                        user_update.uploaded_delta,
                        user_update.downloaded_delta,
                    );
                })
            }
        }
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
