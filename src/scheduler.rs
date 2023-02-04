use std::sync::Arc;

mod history_update;
mod peer_deletion;
mod peer_update;
mod torrent_update;
mod user_update;

pub use history_update::HistoryUpdateBuffer;
pub use peer_deletion::PeerDeletionBuffer;
pub use peer_update::PeerUpdateBuffer;
pub use torrent_update::TorrentUpdateBuffer;
pub use user_update::UserUpdateBuffer;

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
        let mut num_reaped_seeders: u32 = 0;
        let mut num_reaped_leechers: u32 = 0;

        torrent.peers.iter_mut().for_each(|mut peer| {
            match peer.updated_at {
                updated_at if updated_at < inactive_cutoff => {
                    if let Some((index, _)) = torrent.peers.remove(peer.key()) {
                        tracker
                            .peer_deletions
                            .upsert(torrent.id, index.user_id, index.peer_id);

                        match peer.is_seeder {
                            true => num_reaped_seeders += 1,
                            false => num_reaped_leechers += 1,
                        }
                    }
                }
                updated_at if updated_at < active_cutoff => {
                    peer.is_active = false;
                }
                _ => (),
            };
        });

        torrent.seeders += num_reaped_seeders;
        torrent.leechers += num_reaped_leechers;
    });
}
