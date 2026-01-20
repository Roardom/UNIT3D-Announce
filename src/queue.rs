use std::{cmp::min, collections::VecDeque, hash::Hash, slice::Iter, sync::Arc, vec::IntoIter};

pub mod announce_update;
pub mod history_update;
pub mod peer_update;
pub mod torrent_update;
pub mod unregistered_info_hash_update;
pub mod user_update;

use crate::tracker::Tracker;
use futures_util::future::join_all;
use history_update::HistoryUpdate;
use parking_lot::Mutex;
use peer_update::PeerUpdate;
use ringmap::RingMap;
use tokio::{join, time::Instant};
use torrent_update::TorrentUpdate;
use tracing::info;
use unregistered_info_hash_update::UnregisteredInfoHashUpdate;
use user_update::UserUpdate;

/// Holds queued database updates
pub struct Queues {
    pub announces: Mutex<announce_update::Queue>,
    pub histories: Mutex<Queue<history_update::Index, HistoryUpdate>>,
    pub peers: Mutex<Queue<peer_update::Index, PeerUpdate>>,
    pub torrents: Mutex<Queue<torrent_update::Index, TorrentUpdate>>,
    pub unregistered_info_hashes:
        Mutex<Queue<unregistered_info_hash_update::Index, UnregisteredInfoHashUpdate>>,
    pub users: Mutex<Queue<user_update::Index, UserUpdate>>,
}

impl Queues {
    /// Initializes new queues
    pub fn new() -> Queues {
        Queues {
            announces: Mutex::new(announce_update::Queue::new()),
            histories: Mutex::new(Queue::<history_update::Index, HistoryUpdate>::new(
                QueueConfig {
                    max_bindings_per_flush: 65_535,
                    bindings_per_record: 16,
                    // 1 extra binding is used to insert the TTL
                    extra_bindings_per_flush: 1,
                },
            )),
            peers: Mutex::new(Queue::<peer_update::Index, PeerUpdate>::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 15,
                extra_bindings_per_flush: 0,
            })),
            torrents: Mutex::new(Queue::<torrent_update::Index, TorrentUpdate>::new(
                QueueConfig {
                    max_bindings_per_flush: 65_535,
                    bindings_per_record: 15,
                    extra_bindings_per_flush: 0,
                },
            )),
            unregistered_info_hashes: Mutex::new(Queue::<
                unregistered_info_hash_update::Index,
                UnregisteredInfoHashUpdate,
            >::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 4,
                extra_bindings_per_flush: 0,
            })),
            users: Mutex::new(Queue::<user_update::Index, UserUpdate>::new(QueueConfig {
                max_bindings_per_flush: 65_535,
                bindings_per_record: 9,
                extra_bindings_per_flush: 0,
            })),
        }
    }

    /// Send queued updates to mysql database
    pub async fn flush(&self, tracker: &Arc<Tracker>) {
        join!(
            self.flush_announce_updates(tracker),
            self.histories.flush(tracker, "histories"),
            self.peers.flush(tracker, "peers"),
            self.torrents.flush(tracker, "torrents"),
            self.users.flush(tracker, "users"),
            self.unregistered_info_hashes
                .flush(tracker, "unregistered info hashes"),
        );
    }

    /// Send announce updates to mysql database
    async fn flush_announce_updates(&self, tracker: &Arc<Tracker>) {
        let announce_update_batch = self.announces.lock().take_batch();
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
                    .queues
                    .announces
                    .lock()
                    .upsert_batch(announce_update_batch);
            }
        }
    }

    pub fn are_not_empty(&self) -> bool {
        !self.announces.lock().is_empty()
            || self.histories.lock().is_not_empty()
            || self.peers.lock().is_not_empty()
            || self.torrents.lock().is_not_empty()
            || self.users.lock().is_not_empty()
            || self.unregistered_info_hashes.lock().is_not_empty()
    }
}

pub struct Queue<K, V> {
    records: RingMap<K, V>,
    config: QueueConfig,
}

pub struct QueueConfig {
    pub max_bindings_per_flush: usize,
    pub bindings_per_record: usize,
    pub extra_bindings_per_flush: usize,
}

impl QueueConfig {
    fn max_batch_size(&mut self, tracker: &Arc<Tracker>) -> usize {
        let max_bindings = (self.max_bindings_per_flush - self.extra_bindings_per_flush)
            / self.bindings_per_record;

        if let Some(max_records) = tracker.config.load().max_records_per_batch {
            return max_bindings.min(max_records);
        }

        max_bindings
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
            records: RingMap::new(),
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
    fn take_batches(&mut self, tracker: &Arc<Tracker>) -> VecDeque<Batch<K, V>> {
        let max_batches = tracker.config.load().max_batches_per_flush;
        let max_batch_size = self.config.max_batch_size(tracker);

        let mut records = self
            .records
            .drain(0..min(self.records.len(), max_batches * max_batch_size))
            .collect::<Vec<(K, V)>>();

        records.sort_unstable_by(move |a, b| K::cmp(&a.0, &b.0));

        let mut batches = VecDeque::new();

        while records.len() > 0 {
            let batch = records.split_off(records.len() - min(records.len(), max_batch_size));
            batches.push_front(Batch(batch));
        }

        batches
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

pub trait MutexQueueExt {
    async fn flush<'a>(&self, tracker: &Arc<Tracker>, record_type: &'a str);
}

impl<K, V> MutexQueueExt for Mutex<Queue<K, V>>
where
    K: Hash + Eq + Ord,
    V: Clone + Mergeable,
    Batch<K, V>: Flushable<V>,
{
    async fn flush<'a>(&self, tracker: &Arc<Tracker>, record_type: &'a str) {
        let batches = self.lock().take_batches(tracker);

        if batches.is_empty() {
            info!("Upserted 0 {record_type} in 0 ms.");

            return;
        }

        let tasks = batches
            .into_iter()
            .map(|batch| async move {
                let start = Instant::now();
                let len = batch.len();
                let result = batch.flush_to_db(tracker).await;
                let elapsed = start.elapsed().as_millis();

                (len, elapsed, result, batch)
            })
            .collect::<Vec<_>>();

        let results = join_all(tasks).await;

        for (len, elapsed, result, batch) in results {
            match result {
                Ok(_) => {
                    info!("Upserted {len} {record_type} in {elapsed} ms.");
                }
                Err(e) => {
                    info!("Failed to update {len} {record_type} after {elapsed} ms: {e}",);
                    self.lock().upsert_batch(batch);
                }
            }
        }
    }
}

pub struct Batch<K, V>(Vec<(K, V)>);

impl<'a, K, V> Batch<K, V> {
    fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    fn iter(&'a self) -> Iter<'a, (K, V)> {
        self.0.iter()
    }

    fn into_iter(self) -> IntoIter<(K, V)> {
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
