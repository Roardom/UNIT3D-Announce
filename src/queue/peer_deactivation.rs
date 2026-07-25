use std::sync::Arc;

use sqlx::{MySql, QueryBuilder};

use crate::state::AppState;

use super::{peer_update::Index, Flushable, Mergeable};

/// A queued request to mark a peer inactive in the database.
///
/// Emitted by the reaper ([`crate::scheduler::reap`]) when a peer transitions
/// from active to inactive in the in-memory store. It carries no payload beyond
/// its key: the only mutation is flipping the `active` flag off.
///
/// Without this, the reaper only ever mutates the in-memory store, so the peer's
/// `active = 1` row lingers in the database forever. The [`super::peer_update`]
/// flush only sets `active` from a live announce, and an explicit `stopped`
/// event is the sole other path that writes `active = 0`. Any client that
/// abandons a `peer_id` without sending `stopped` — qBittorrent regenerating its
/// `peer_id` on restart/re-add, a crash, or a dropped connection — therefore
/// leaves a ghost `active = 1` row, which surfaces as the same torrent appearing
/// multiple times in UNIT3D's peer lists.
#[derive(Clone, Copy)]
pub struct PeerDeactivation;

impl Mergeable for PeerDeactivation {
    fn merge(&mut self, _new: &Self) {
        // A deactivation carries no data beyond its key, so repeated
        // deactivations of the same peer collapse to a single no-op merge.
    }
}

impl Flushable<PeerDeactivation> for super::Batch<Index, PeerDeactivation> {
    async fn flush_to_db(&self, state: &Arc<AppState>) -> Result<u64, sqlx::Error> {
        if self.is_empty() {
            return Ok(0);
        }

        // Only the `active` flag is touched, matched on the full primary key, so
        // no other column is clobbered. This matters because the in-memory peer
        // does not retain `ip`, `port`, `agent` or `left`, and by the time it is
        // marked inactive both endpoints are already `None` — there is simply no
        // faithful `PeerUpdate` to emit, and a synthetic one would overwrite good
        // data via the `peer_update` upsert.
        let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
            "UPDATE peers SET active = FALSE WHERE (peer_id, torrent_id, user_id) IN (",
        );

        let mut is_first = true;

        for (index, _) in self.iter() {
            if !is_first {
                query_builder.push(", ");
            }
            is_first = false;

            query_builder.push("(");
            query_builder.push_bind(index.peer_id.to_vec());
            query_builder.push(", ");
            query_builder.push_bind(index.torrent_id);
            query_builder.push(", ");
            query_builder.push_bind(index.user_id);
            query_builder.push(")");
        }

        query_builder.push(")");

        query_builder
            .build()
            .persistent(false)
            .execute(&state.pool)
            .await
            .map(|result| result.rows_affected())
    }
}
