use std::{cmp::min, hash::Hash, sync::Arc};

pub mod announce_update;
pub mod history_update;
pub mod peer_update;
pub mod torrent_update;
pub mod unregistered_info_hash_update;
pub mod user_update;

use crate::tracker::Tracker;
use chrono::{Duration, Utc};
use indexmap::{
    map::{IntoIter, Iter},
    IndexMap,
};
use parking_lot::Mutex;
use tokio::{join, time::Instant};
use torrent_update::{Index, TorrentUpdate};
use tracing::info;

pub async fn handle(tracker: &Arc<Tracker>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
    let mut counter = 0_u64;

    loop {
        interval.tick().await;
        counter += 1;

        if counter % tracker.config.read().flush_interval_milliseconds == 0 {
            flush(tracker).await;
        }

        if counter % (tracker.config.read().peer_expiry_interval * 1000) == 0 {
            reap(tracker).await;
        }
    }
}

/// Send queued updates to mysql database
pub async fn flush(tracker: &Arc<Tracker>) {
    join!(
        flush_announce_updates(tracker),
        tracker.history_updates.flush(tracker, "histories"),
        tracker.peer_updates.flush(tracker, "peers"),
        tracker.torrent_updates.flush(tracker, "torrents"),
        tracker.user_updates.flush(tracker, "users"),
        tracker
            .unregistered_info_hash_updates
            .flush(tracker, "unregistered info hashes"),
    );
}

/// Send announce updates to mysql database
async fn flush_announce_updates(tracker: &Arc<Tracker>) {
    let announce_update_batch = tracker.announce_updates.lock().take_batch();
    let start = Instant::now();
    let len = announce_update_batch.len();
    let result = announce_update_batch.flush_to_db(tracker).await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} announces in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} announces after {elapsed} ms: {e}");
            tracker
                .announce_updates
                .lock()
                .upsert_batch(announce_update_batch);
        }
    }
}

/// Remove peers that have not announced for some time
pub async fn reap(tracker: &Arc<Tracker>) {
    let ttl = Duration::seconds(tracker.config.read().active_peer_ttl.try_into().unwrap());
    let active_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();
    let ttl = Duration::seconds(tracker.config.read().inactive_peer_ttl.try_into().unwrap());
    let inactive_cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    for (_index, torrent) in tracker.torrents.lock().iter_mut() {
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
                peer.is_active = false;

                if peer.is_visible {
                    tracker
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
            }
        }

        // Update peer count of torrents and users
        if seeder_delta != 0 || leecher_delta != 0 {
            torrent.seeders = torrent.seeders.saturating_add_signed(seeder_delta);
            torrent.leechers = torrent.leechers.saturating_add_signed(leecher_delta);

            tracker.torrent_updates.lock().upsert(
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

pub struct Queue<K, V> {
    records: IndexMap<K, V>,
    config: QueueConfig,
}

pub struct QueueConfig {
    pub max_bindings_per_flush: usize,
    pub bindings_per_record: usize,
    pub extra_bindings_per_flush: usize,
}

impl QueueConfig {
    fn max_batch_size(&mut self) -> usize {
        (self.max_bindings_per_flush - self.extra_bindings_per_flush) / self.bindings_per_record
    }
}

impl<K, V> Queue<K, V>
where
    K: Hash + Eq + Ord,
    V: Clone + Mergeable,
{
    /// Initialize a new queue
    pub fn new(config: QueueConfig) -> Queue<K, V> {
        Self {
            records: IndexMap::new(),
            config,
        }
    }

    /// Upsert a single update into the queue
    pub fn upsert(&mut self, key: K, value: V) {
        self.records
            .entry(key)
            .and_modify(|update| update.merge(&value))
            .or_insert(value);
    }

    /// Take a portion of the updates from the start of the queue with a max
    /// size defined by the buffer config
    fn take_batch(&mut self) -> Batch<K, V> {
        let mut batch = self
            .records
            .drain(0..min(self.records.len(), self.config.max_batch_size()))
            .collect::<IndexMap<K, V>>();

        batch.sort_unstable_keys();

        Batch(batch)
    }

    /// Bulk upsert a batch into the end of the queue
    fn upsert_batch(&mut self, batch: Batch<K, V>) {
        batch.into_iter().for_each(|(k, v)| self.upsert(k, v));
    }

    pub fn is_not_empty(&self) -> bool {
        !self.records.is_empty()
    }
}

pub trait Mergeable {
    /// Merge an existing record with a new record
    fn merge(&mut self, new: &Self);
}

trait MutexQueueExt {
    async fn flush<'a>(&self, tracker: &Arc<Tracker>, record_type: &'a str);
}

impl<K, V> MutexQueueExt for Mutex<Queue<K, V>>
where
    K: Hash + Eq + Ord,
    V: Clone + Mergeable,
    Batch<K, V>: Flushable<V>,
{
    async fn flush<'a>(&self, tracker: &Arc<Tracker>, record_type: &'a str) {
        let batch = self.lock().take_batch();
        let start = Instant::now();
        let len = batch.len();
        let result = batch.flush_to_db(tracker).await;
        let elapsed = start.elapsed().as_millis();

        match result {
            Ok(_) => {
                info!("Upserted {len} {record_type} in {elapsed} ms.");
            }
            Err(e) => {
                info!("Failed to update {len} {record_type} after {elapsed} ms: {e}");
                self.lock().upsert_batch(batch);
            }
        }
    }
}

pub struct Batch<K, V>(IndexMap<K, V>);

impl<'a, K, V> Batch<K, V> {
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    fn iter(&'a self) -> Iter<'a, K, V> {
        self.0.iter()
    }

    fn into_iter(self) -> IntoIter<K, V> {
        self.0.into_iter()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

pub trait Flushable<T> {
    /// Flushes batch of updates to MySQL database
    ///
    /// **Warning**: this function does not make sure that the query isn't too long
    /// or doesn't use too many bindings
    async fn flush_to_db(&self, tracker: &Arc<Tracker>) -> Result<u64, sqlx::Error>;
}
