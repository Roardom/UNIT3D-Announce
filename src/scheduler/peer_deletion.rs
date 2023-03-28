use std::{
    cmp::min,
    ops::{Deref, DerefMut},
};

use indexmap::IndexSet;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::tracker::peer::PeerId;

pub struct Queue(pub IndexSet<PeerDeletion>);

#[derive(Eq, Hash, PartialEq)]
pub struct PeerDeletion {
    pub torrent_id: u32,
    pub user_id: u32,
    pub peer_id: crate::tracker::peer::PeerId,
}

impl Queue {
    pub fn new() -> Queue {
        Queue(IndexSet::new())
    }

    pub fn upsert(&mut self, torrent_id: u32, user_id: u32, peer_id: PeerId) {
        self.insert(PeerDeletion {
            torrent_id,
            user_id,
            peer_id,
        });
    }

    pub async fn flush_to_db(&mut self, db: &MySqlPool) {
        let len = self.len();

        if len == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_PEER_COLUMNS: usize = 3;
        const PEER_LIMIT: usize = BIND_LIMIT / NUM_PEER_COLUMNS;

        let peer_deletions = self.split_off(len - min(PEER_LIMIT, len));

        // Requires trailing space last before push_tuples
        // Requires leading space after push_tuples
        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            r#"
                DELETE FROM
                    peers
                WHERE
                    (torrent_id, user_id, peer_id)
                IN
            "#,
        );

        query_builder.push_tuples(&peer_deletions, |mut bind, peer_deletion| {
            bind.push_bind(peer_deletion.torrent_id)
                .push_bind(peer_deletion.user_id)
                .push_bind(peer_deletion.peer_id.to_vec());
        });

        let result = query_builder
            .build()
            .persistent(false)
            .execute(db)
            .await
            .map(|result| result.rows_affected());

        match result {
            Ok(_) => (),
            Err(e) => {
                println!("Peer deletion failed: {}", e);

                for peer_deletion in peer_deletions.iter() {
                    self.upsert(
                        peer_deletion.torrent_id,
                        peer_deletion.user_id,
                        peer_deletion.peer_id,
                    );
                }
            }
        }
    }
}

impl Deref for Queue {
    type Target = IndexSet<PeerDeletion>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
