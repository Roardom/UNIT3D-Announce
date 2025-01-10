use std::{cmp::min, hash::Hash, sync::Arc};

pub mod announce_update;
pub mod history_update;
pub mod peer_update;
pub mod torrent_update;
pub mod unregistered_info_hash_update;
pub mod user_update;

use crate::tracker::Tracker;
use chrono::{Duration, Utc};
use indexmap::{map::Values, IndexMap};
use sqlx::MySqlPool;
use tokio::{join, sync::mpsc, time::Instant};
use torrent_update::TorrentUpdate;
use tracing::info;

use self::history_update::HistoryUpdateExtraBindings;

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
        flush_history_updates(tracker),
        flush_peer_updates(tracker),
        flush_torrent_updates(tracker),
        flush_user_updates(tracker),
        flush_announce_updates(tracker),
        flush_unregistered_info_hash_updates(tracker),
    );
}

/// Send history updates to mysql database
async fn flush_history_updates(tracker: &Arc<Tracker>) {
    let history_update_batch = tracker.history_updates.lock().take_batch();
    let start = Instant::now();
    let len = history_update_batch.len();
    let seedtime_ttl =
        tracker.config.read().active_peer_ttl + tracker.config.read().peer_expiry_interval;
    let result = history_update_batch
        .flush_to_db(&tracker.pool, HistoryUpdateExtraBindings { seedtime_ttl })
        .await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} histories in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} histories after {elapsed} ms: {e}");
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
    let start = Instant::now();
    let len = peer_update_batch.len();
    let result = peer_update_batch.flush_to_db(&tracker.pool, ()).await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} peers in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} peers after {elapsed} ms: {e}");
            tracker.peer_updates.lock().upsert_batch(peer_update_batch);
        }
    }
}

/// Send torrent updates to mysql database
async fn flush_torrent_updates(tracker: &Arc<Tracker>) {
    let torrent_update_batch = tracker.torrent_updates.lock().take_batch();
    let start = Instant::now();
    let len = torrent_update_batch.len();
    let result = torrent_update_batch.flush_to_db(&tracker.pool, ()).await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} torrents in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} torrents after {elapsed} ms: {e}");
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
    let start = Instant::now();
    let len = user_update_batch.len();
    let result = user_update_batch.flush_to_db(&tracker.pool, ()).await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} users in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} users after {elapsed} ms: {e}");
            tracker.user_updates.lock().upsert_batch(user_update_batch);
        }
    }
}

/// Send announce updates to mysql database
async fn flush_announce_updates(tracker: &Arc<Tracker>) {
    let announce_update_batch = tracker.announce_updates.lock().take_batch();
    let start = Instant::now();
    let len = announce_update_batch.len();
    let result = announce_update_batch.flush_to_db(&tracker.pool).await;
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

/// Send unregistered info hash updates to mysql database
async fn flush_unregistered_info_hash_updates(tracker: &Arc<Tracker>) {
    let unregistered_info_hash_update_batch =
        tracker.unregistered_info_hash_updates.lock().take_batch();
    let start = Instant::now();
    let len = unregistered_info_hash_update_batch.len();
    let result = unregistered_info_hash_update_batch
        .flush_to_db(&tracker.pool, ())
        .await;
    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            info!("Upserted {len} unregistered info hashes in {elapsed} ms.");
        }
        Err(e) => {
            info!("Failed to update {len} unregistered info hashes after {elapsed} ms: {e}");
            tracker
                .unregistered_info_hash_updates
                .lock()
                .upsert_batch(unregistered_info_hash_update_batch);
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

        for (_index, peer) in torrent.peers.iter_mut() {
            // Peers get marked as inactive if not announced for more than
            // active_peer_ttl seconds. User peer count and torrent peer
            // count are updated to reflect.
            if peer.updated_at < active_cutoff && peer.is_active {
                peer.is_active = false;

                if peer.is_visible {
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
        }

        // Update peer count of torrents and users
        if seeder_delta != 0 || leecher_delta != 0 {
            torrent.seeders = torrent.seeders.saturating_add_signed(seeder_delta);
            torrent.leechers = torrent.leechers.saturating_add_signed(leecher_delta);

            tracker.torrent_update_tx.send(TorrentUpdate {
                torrent_id: torrent.id,
                seeder_delta,
                leecher_delta,
                times_completed_delta: 0,
                balance_delta: 0,
            });
        }
    }
}

pub struct Queue<K, V> {
    records: IndexMap<K, V>,
    config: QueueConfig,
    receiver: mpsc::UnboundedReceiver<V>,
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
    K: Hash + Eq,
    V: Clone + Mergeable,
    Queue<K, V>: Upsertable<V>,
{
    /// Initialize a new queue
    pub fn new(config: QueueConfig, receiver: mpsc::UnboundedReceiver<V>) -> Queue<K, V> {
        Self {
            records: IndexMap::new(),
            config,
            receiver,
        }
    }

    /// Take a portion of the updates from the start of the queue with a max
    /// size defined by the buffer config
    fn take_batch(&mut self) -> Batch<K, V> {
        let len = self.records.len();

        Batch(
            self.records
                .drain(0..min(len, self.config.max_batch_size()))
                .collect(),
        )
    }

    /// Bulk upsert a batch into the end of the queue
    fn upsert_batch(&mut self, batch: Batch<K, V>) {
        for record in batch.values() {
            self.upsert(record.clone());
        }
    }

    pub fn is_not_empty(&self) -> bool {
        !self.records.is_empty()
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            self.upsert(message);
        }
    }
}

pub trait Mergeable {
    /// Merge a record with a new record
    fn merge(&mut self, new: &Self);
}

pub trait Upsertable<T> {
    fn upsert(&mut self, new: T);
}

pub struct Batch<K, V>(IndexMap<K, V>);

impl<'a, K, V> Batch<K, V> {
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    fn values(&'a self) -> Values<'a, K, V> {
        self.0.values()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

pub trait Flushable<T> {
    /// Used to store extra bindings used in the query when the record already
    /// exists in the database
    type ExtraBindings;

    /// Flushes batch of updates to MySQL database
    ///
    /// **Warning**: this function does not make sure that the query isn't too long
    /// or doesn't use too many bindings
    async fn flush_to_db(
        &self,
        db: &MySqlPool,
        extra_bindings: Self::ExtraBindings,
    ) -> Result<u64, sqlx::Error>;
}
