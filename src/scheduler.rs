use std::sync::Arc;

pub mod history_update;
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
        .write()
        .await
        .flush_to_db(
            &tracker.pool,
            tracker.config.active_peer_ttl + tracker.config.peer_expiry_interval,
        )
        .await;
    tracker
        .peer_updates
        .write()
        .await
        .flush_to_db(&tracker.pool)
        .await;
    tracker
        .torrent_updates
        .write()
        .await
        .flush_to_db(&tracker.pool)
        .await;
    tracker
        .user_updates
        .write()
        .await
        .flush_to_db(&tracker.pool)
        .await;
}

/// Remove peers that have not announced for some time
pub async fn reap(tracker: &Arc<Tracker>) {
    let flush_interval = Duration::seconds(tracker.config.flush_interval.try_into().unwrap());
    let two_flushes_ago = Utc::now().checked_sub_signed(flush_interval * 2).unwrap();
    let ttl = Duration::seconds(tracker.config.active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(tracker.config.inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    for (_index, torrent) in tracker.torrents.write().await.iter_mut() {
        let mut num_inactivated_seeders: u32 = 0;
        let mut num_inactivated_leechers: u32 = 0;

        // If a peer is marked as inactive and it has not announced for
        // more than inactive_peer_ttl, then it is permanently deleted.
        // It is also permanently deleted if it was updated less than
        // two flushes ago and was marked as inactive (meaning the user
        // send a `stopped` event).
        torrent.peers.retain(|_index, peer| {
            (inactive_cutoff <= peer.updated_at && peer.updated_at <= two_flushes_ago)
                || peer.is_active
        });

        for (_index, peer) in torrent.peers.iter_mut() {
            // Peers get marked as inactive if not announced for more than
            // active_peer_ttl seconds. User peer count and torrent peer
            // count are updated to reflect.
            if peer.updated_at < active_cutoff && peer.is_active {
                peer.is_active = false;
                tracker
                    .users
                    .write()
                    .await
                    .entry(peer.user_id)
                    .and_modify(|user| {
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
        }

        // Update peer count of torrents and users
        if num_inactivated_seeders > 0 || num_inactivated_leechers > 0 {
            torrent.seeders = torrent.seeders.saturating_sub(num_inactivated_seeders);
            torrent.leechers = torrent.leechers.saturating_sub(num_inactivated_leechers);
            tracker.torrent_updates.write().await.upsert(
                torrent.id,
                torrent.seeders,
                torrent.leechers,
                torrent.times_completed,
            );
        }
    }
}
