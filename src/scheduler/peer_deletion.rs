use std::ops::Deref;

use dashmap::DashSet;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::tracker::peer::PeerId;

pub struct PeerDeletionBuffer(pub DashSet<PeerDeletion>);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PeerDeletion {
    pub torrent_id: u32,
    pub user_id: u32,
    pub peer_id: crate::tracker::peer::PeerId,
}

impl PeerDeletionBuffer {
    pub fn new() -> PeerDeletionBuffer {
        PeerDeletionBuffer(DashSet::new())
    }

    pub fn upsert(&self, torrent_id: u32, user_id: u32, peer_id: PeerId) {
        self.insert(PeerDeletion {
            torrent_id,
            user_id,
            peer_id,
        });
    }

    pub async fn flush_to_db(&self, db: &MySqlPool) {
        if self.len() == 0 {
            return;
        }

        const BIND_LIMIT: usize = 65535;
        const NUM_PEER_COLUMNS: usize = 3;
        const PEER_LIMIT: usize = BIND_LIMIT / NUM_PEER_COLUMNS;

        let mut peer_deletions: Vec<_> = vec![];

        for _ in 0..std::cmp::min(PEER_LIMIT, self.len()) {
            let peer_deletion = *self.iter().next().unwrap();
            self.remove(&PeerDeletion {
                torrent_id: peer_deletion.torrent_id,
                user_id: peer_deletion.user_id,
                peer_id: peer_deletion.peer_id,
            });
            peer_deletions.push(peer_deletion);
        }

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

        query_builder.push_tuples(peer_deletions.clone(), |mut bind, peer_deletion| {
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
                peer_deletions.into_iter().for_each(|peer_deletion| {
                    self.upsert(
                        peer_deletion.torrent_id,
                        peer_deletion.user_id,
                        peer_deletion.peer_id,
                    );
                });
            }
        }
    }
}

impl Deref for PeerDeletionBuffer {
    type Target = DashSet<PeerDeletion>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
