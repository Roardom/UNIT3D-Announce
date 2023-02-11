use std::sync::Arc;

pub mod history_update;
pub mod peer_deletion;
pub mod peer_update;
pub mod torrent_update;
pub mod user_update;

use crate::tracker::Tracker;
use chrono::{Duration, Utc};

pub async fn handle(tracker: &Arc<Tracker>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut counter = 0_u64;

    loop {
        interval.tick().await;
        counter += 1;

        if counter % tracker.config.flush_interval == 0 {
            flush(tracker).await;
        }

        if counter % tracker.config.peer_expiry_interval == 0 {
            reap(tracker).await;
        }
    }
}

/// Send queued updates to mysql database
pub async fn flush(tracker: &Arc<Tracker>) {
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
    tracker.user_updates.flush_to_db(&tracker.pool).await;
}

/// Remove peers that have not announced for some time
pub async fn reap(tracker: &Arc<Tracker>) {
    let ttl = Duration::seconds(tracker.config.active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(tracker.config.inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    tracker.torrents.iter_mut().for_each(|mut torrent| {
        let mut num_inactivated_seeders: u32 = 0;
        let mut num_inactivated_leechers: u32 = 0;

        torrent.peers.iter_mut().for_each(|mut peer| {
            match peer.updated_at {
                // If a peer is marked as inactive and it has not announced for
                // more than inactive_peer_ttl, then it is permanently deleted.
                updated_at if updated_at < inactive_cutoff && !peer.is_active => {
                    if let Some((index, _)) = torrent.peers.remove(peer.key()) {
                        tracker
                            .peer_deletions
                            .upsert(torrent.id, index.user_id, index.peer_id);
                    }
                }
                // Peers get marked as inactive if not announced for more than
                // active_peer_ttl seconds. User peer count and torrent peer
                // count are updated to reflect.
                updated_at if updated_at < active_cutoff && peer.is_active => {
                    peer.is_active = false;
                    tracker.users.entry(peer.user_id).and_modify(|user| {
                        if peer.is_seeder {
                            user.num_seeding = user.num_seeding.saturating_sub(1);
                        } else {
                            user.num_leeching = user.num_leeching.saturating_sub(1);
                        }
                    });
                    match peer.is_seeder {
                        true => num_inactivated_seeders += 1,
                        false => num_inactivated_leechers += 1,
                    }
                }
                _ => (),
            };
        });

        // Update peer count of torrents and users
        if num_inactivated_seeders > 0 || num_inactivated_leechers > 0 {
            torrent.seeders = torrent.seeders.saturating_sub(num_inactivated_seeders);
            torrent.leechers = torrent.leechers.saturating_sub(num_inactivated_leechers);
            tracker.torrent_updates.upsert(
                torrent.id,
                torrent.seeders,
                torrent.leechers,
                torrent.times_completed,
            );
        }
    });
}
