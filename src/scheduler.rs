use std::sync::Arc;

mod history_update;
mod peer_deletion;
mod peer_update;
mod torrent_update;

pub use history_update::HistoryUpdateBuffer;
pub use peer_deletion::PeerDeletionBuffer;
pub use peer_update::PeerUpdateBuffer;
pub use torrent_update::TorrentUpdateBuffer;

use crate::tracker::Tracker;
use chrono::{Duration, Utc};

pub async fn handle(tracker: &Arc<Tracker>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut counter = 0_u64;

    loop {
        interval.tick().await;
        counter += 1;

        if counter % tracker.config.flush_interval == 0 {
            tracker
                .history_updates
                .flush_to_db(
                    &tracker.pool,
                    tracker.config.active_peer_ttl + tracker.config.peer_expiry_interval,
                )
                .await;
            tracker.peer_updates.flush_to_db(&tracker.pool).await;
            tracker.peer_deletions.flush_to_db(&tracker.pool).await;
            tracker.torrent_updates.flush_to_db(&tracker.pool).await;
        }

        if counter % tracker.config.peer_expiry_interval == 0 {
            let ttl = Duration::seconds(tracker.config.active_peer_ttl.try_into().unwrap());
            let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
            let ttl = Duration::seconds(tracker.config.inactive_peer_ttl.try_into().unwrap());
            let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

            tracker.torrents.iter().for_each(|torrent| {
                torrent.peers.iter_mut().for_each(|mut peer| {
                    // TODO: Decrease leechers/seeders. Only delete from mysql once. Verify mysql doesn't error if trying to delete non-existing peer tuple.
                    match peer.updated_at {
                        x if x < inactive_cutoff => {
                            if let Some((index, _)) = torrent.peers.remove(peer.key()) {
                                tracker.peer_deletions.upsert(
                                    torrent.id,
                                    index.user_id,
                                    index.peer_id,
                                );
                            }
                        }
                        x if x < active_cutoff => {
                            peer.is_active = false;
                            tracker.peer_deletions.upsert(
                                torrent.id,
                                peer.user_id,
                                peer.key().peer_id,
                            );
                        }
                        _ => (),
                    };
                });
            });
        }
    }
}
