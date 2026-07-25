use std::sync::Arc;

use crate::queue::torrent_update::{Index, TorrentUpdate};
use crate::state::AppState;
use chrono::{Duration, Utc};
use tokio::time::Instant;
use tracing::info;

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
    use rayon::prelude::*;

    let start = Instant::now();
    let config = state.config.load();
    let ttl = Duration::seconds(config.active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(config.inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    let mut torrent_store = state.stores.torrents.lock();

    torrent_store.par_values_mut().for_each(|torrent| {
        // Copied out so it can be read while `torrent.peers` is borrowed mutably.
        let torrent_id = torrent.id;
        let mut seeder_delta: i32 = 0;
        let mut leecher_delta: i32 = 0;

        // If a peer is marked as inactive and it has not announced for
        // more than inactive_peer_ttl, then it is permanently deleted.
        torrent
            .peers
            .retain(|_, peer| inactive_cutoff <= peer.updated_at || peer.is_active);

        for (index, peer) in torrent.peers.iter_mut() {
            let was_included = peer.is_included_in_peer_list(&config);
            let was_seeder = peer.is_seeder;

            // Expire individual endpoints based on their own updated_at
            if peer.ipv4.is_some_and(|ep| ep.updated_at < active_cutoff) {
                peer.ipv4 = None;
            }
            if peer.ipv6.is_some_and(|ep| ep.updated_at < active_cutoff) {
                peer.ipv6 = None;
            }

            // If all endpoints are gone, mark peer inactive
            if peer.ipv4.is_none() && peer.ipv6.is_none() && peer.is_active {
                peer.is_active = false;

                // Propagate the deactivation to the database. The reaper only
                // mutates the in-memory store; the peer_update flush sets
                // `active` solely from a live announce and an explicit `stopped`
                // is the only other path that clears it. Without this enqueue a
                // client that abandons a peer_id without sending `stopped`
                // (qBittorrent regenerating its peer_id on restart/re-add, a
                // crash, a dropped connection) leaves a ghost `active = 1` row,
                // which shows up as the torrent appearing once per stale peer_id
                // in UNIT3D's peer lists.
                state.queues.peer_deactivations.lock().upsert(
                    crate::queue::peer_update::Index {
                        user_id: index.user_id,
                        torrent_id,
                        peer_id: index.peer_id,
                    },
                    crate::queue::peer_deactivation::PeerDeactivation,
                );
            }

            let is_included = peer.is_included_in_peer_list(&config);

            if was_included && !is_included {
                state
                    .stores
                    .users
                    .write()
                    .entry(index.user_id)
                    .and_modify(|user| {
                        if was_seeder {
                            user.num_seeding = user.num_seeding.saturating_sub(1);
                        } else {
                            user.num_leeching = user.num_leeching.saturating_sub(1);
                        }
                    });
                match was_seeder {
                    true => seeder_delta -= 1,
                    false => leecher_delta -= 1,
                }
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
    });

    let elapsed = start.elapsed().as_millis();
    info!("Expired stale peers in {elapsed} ms.")
}
