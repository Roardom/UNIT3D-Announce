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
    flush_history_updates(tracker).await;
    flush_peer_updates(tracker).await;
    flush_torrent_updates(tracker).await;
    flush_user_updates(tracker).await;
}

/// Send history updates to mysql database
async fn flush_history_updates(tracker: &Arc<Tracker>) {
    let history_update_batch = tracker.history_updates.lock().take_batch();
    let result = history_update_batch
        .flush_to_db(
            &tracker.pool,
            tracker.config.active_peer_ttl + tracker.config.peer_expiry_interval,
        )
        .await;

    match result {
        Ok(_) => (),
        Err(e) => {
            println!("History update failed: {}", e);
            tracker
                .history_updates
                .lock()
                .upsert_batch(history_update_batch);
        }
    }
}

/// Send peer updates to mysql database
async fn flush_peer_updates(tracker: &Arc<Tracker>) {
    let peer_update_batch = tracker.peer_updates.lock().take_batch();
    let result = peer_update_batch.flush_to_db(&tracker.pool).await;

    match result {
        Ok(_) => (),
        Err(e) => {
            println!("Peer update failed: {}", e);
            tracker.peer_updates.lock().upsert_batch(peer_update_batch);
        }
    }
}

/// Send torrent updates to mysql database
async fn flush_torrent_updates(tracker: &Arc<Tracker>) {
    let torrent_update_batch = tracker.torrent_updates.lock().take_batch();
    let result = torrent_update_batch.flush_to_db(&tracker.pool).await;

    match result {
        Ok(_) => (),
        Err(e) => {
            println!("Torrent update failed: {}", e);
            tracker
                .torrent_updates
                .lock()
                .upsert_batch(torrent_update_batch);
        }
    }
}

/// Send user updates to mysql database
async fn flush_user_updates(tracker: &Arc<Tracker>) {
    let user_update_batch = tracker.user_updates.lock().take_batch();
    let result = user_update_batch.flush_to_db(&tracker.pool).await;

    match result {
        Ok(_) => (),
        Err(e) => {
            println!("User update failed: {}", e);
            tracker.user_updates.lock().upsert_batch(user_update_batch);
        }
    }
}

/// Remove peers that have not announced for some time
pub async fn reap(tracker: &Arc<Tracker>) {
    let flush_interval = Duration::seconds(tracker.config.flush_interval.try_into().unwrap());
    let two_flushes_ago = Utc::now().checked_sub_signed(flush_interval * 2).unwrap();
    let ttl = Duration::seconds(tracker.config.active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(tracker.config.inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    for (_index, torrent) in tracker.torrents.lock().iter_mut() {
        let mut seeder_delta: i32 = 0;
        let mut leecher_delta: i32 = 0;

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
                    .entry(peer.user_id)
                    .and_modify(|user| {
                        if peer.is_seeder {
                            user.num_seeding = user.num_seeding.saturating_sub(1);
                        } else {
                            user.num_leeching = user.num_leeching.saturating_sub(1);
                        }
                    });
                match peer.is_seeder {
                    true => seeder_delta -= 1,
                    false => leecher_delta -= 1,
                }
            }
        }

        // Update peer count of torrents and users
        if seeder_delta != 0 || leecher_delta != 0 {
            torrent.seeders = torrent.seeders.saturating_add_signed(seeder_delta);
            torrent.leechers = torrent.leechers.saturating_add_signed(leecher_delta);

            tracker
                .torrent_updates
                .lock()
                .upsert(torrent.id, seeder_delta, leecher_delta, 0);
        }
    }
}
