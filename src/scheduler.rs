use std::sync::Arc;

use crate::queue::torrent_update::{Index, TorrentUpdate};
use crate::state::AppState;
use chrono::{Duration, Utc};

pub async fn handle(state: &Arc<AppState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
    let mut counter = 0_u64;

    loop {
        interval.tick().await;
        counter += 1;

        if counter % state.config.load().flush_interval_milliseconds == 0 {
            state.queues.flush(state).await;
        }

        if counter % (state.config.load().peer_expiry_interval * 1000) == 0 {
            reap(state).await;
        }
    }
}

/// Remove peers that have not announced for some time
pub async fn reap(state: &Arc<AppState>) {
    let config = state.config.load();
    let ttl = Duration::seconds(config.active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(config.inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    for (_index, torrent) in state.stores.torrents.lock().iter_mut() {
        let mut seeder_delta: i32 = 0;
        let mut leecher_delta: i32 = 0;

        // If a peer is marked as inactive and it has not announced for
        // more than inactive_peer_ttl, then it is permanently deleted.
        torrent
            .peers
            .retain(|_index, peer| inactive_cutoff <= peer.updated_at || peer.is_active);

        for (index, peer) in torrent.peers.iter_mut() {
            // Peers get marked as inactive if not announced for more than
            // active_peer_ttl seconds. User peer count and torrent peer
            // count are updated to reflect.
            if peer.updated_at < active_cutoff && peer.is_active {
                if peer.is_included_in_peer_list(&config) {
                    state
                        .stores
                        .users
                        .write()
                        .entry(index.user_id)
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

                peer.is_active = false;
            }
        }

        // Update peer count of torrents and users
        if seeder_delta != 0 || leecher_delta != 0 {
            torrent.seeders = torrent.seeders.saturating_add_signed(seeder_delta);
            torrent.leechers = torrent.leechers.saturating_add_signed(leecher_delta);

            state.queues.torrents.lock().upsert(
                Index {
                    torrent_id: torrent.id,
                },
                TorrentUpdate {
                    seeder_delta,
                    leecher_delta,
                    times_completed_delta: 0,
                    balance_delta: 0,
                },
            );
        }
    }
}
