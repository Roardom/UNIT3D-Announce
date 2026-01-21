use std::net::IpAddr;
use std::ops::{Deref, DerefMut};

use futures_util::TryStreamExt;
use indexmap::IndexMap;
use serde::Serialize;
use sqlx::MySqlPool;
use sqlx::types::chrono::{DateTime, Utc};

use anyhow::{Context, Result};

use crate::model::{peer_id::PeerId, torrent_status::TorrentStatus};
use crate::store::peer::{Index, Peer, PeerStore};

pub struct TorrentStore {
    inner: IndexMap<u32, Torrent>,
}

impl TorrentStore {
    pub fn new() -> TorrentStore {
        TorrentStore {
            inner: IndexMap::new(),
        }
    }

    pub async fn from_db(db: &MySqlPool) -> Result<TorrentStore> {
        // Load one torrent per info hash. If multiple are found, prefer
        // undeleted torrents. If multiple are still found, prefer approved
        // torrents. If multiple are still found, prefer the oldest.
        let torrents = sqlx::query_as!(
            DBImportTorrent,
            r#"
                SELECT
                    torrents.id as `id: u32`,
                    torrents.status as `status: TorrentStatus`,
                    torrents.seeders as `seeders: u32`,
                    torrents.leechers as `leechers: u32`,
                    torrents.times_completed as `times_completed: u32`,
                    100 - LEAST(torrents.free, 100) as `download_factor: u8`,
                    IF(torrents.doubleup, 200, 100) as `upload_factor: u8`,
                    torrents.deleted_at IS NOT NULL as `is_deleted: bool`
                FROM
                    torrents
                JOIN (
                    SELECT
                        COALESCE(
                            MIN(CASE WHEN deleted_at IS NULL AND status = 1 THEN id END),
                            MIN(CASE WHEN deleted_at IS NULL AND status != 1 THEN id END),
                            MIN(CASE WHEN deleted_at IS NOT NULL THEN id END)
                        ) AS id
                    FROM
                        torrents
                    GROUP BY
                        info_hash
                ) AS distinct_torrents
                    ON distinct_torrents.id = torrents.id
            "#
        )
        .fetch(db)
        .try_fold(TorrentStore::new(), |mut store, torrent| async move {
            store.insert(
                torrent.id,
                Torrent {
                    id: torrent.id,
                    status: torrent.status,
                    seeders: torrent.seeders,
                    leechers: torrent.leechers,
                    times_completed: torrent.times_completed,
                    download_factor: torrent.download_factor,
                    upload_factor: torrent.upload_factor,
                    is_deleted: torrent.is_deleted,
                    peers: PeerStore::new(),
                },
            );

            Ok(store)
        })
        .await
        .context("Failed loading torrents.")?;

        // Load peers into each torrent
        sqlx::query!(
            r#"
                SELECT
                    INET6_NTOA(peers.ip) as `ip_address: IpAddr`,
                    peers.user_id as `user_id: u32`,
                    peers.torrent_id as `torrent_id: u32`,
                    peers.port as `port: u16`,
                    peers.seeder as `is_seeder: bool`,
                    peers.active as `is_active: bool`,
                    peers.visible as `is_visible: bool`,
                    peers.connectable as `is_connectable: bool`,
                    peers.updated_at as `updated_at: DateTime<Utc>`,
                    peers.uploaded as `uploaded: u64`,
                    peers.downloaded as `downloaded: u64`,
                    peers.peer_id as `peer_id: PeerId`
                FROM
                    peers
            "#
        )
        .fetch(db)
        .try_fold(torrents, |mut store, peer| async move {
            store.entry(peer.torrent_id).and_modify(|torrent| {
                torrent.peers.insert(
                    Index {
                        user_id: peer.user_id,
                        peer_id: peer.peer_id,
                    },
                    Peer {
                        ip_address: peer
                            .ip_address
                            .expect("INET6_NTOA failed to decode peer ip."),
                        port: peer.port,
                        is_seeder: peer.is_seeder,
                        is_active: peer.is_active,
                        is_visible: peer.is_visible,
                        is_connectable: peer.is_connectable,
                        has_sent_completed: false,
                        updated_at: peer
                            .updated_at
                            .expect("Peer with a null updated_at found in database."),
                        uploaded: peer.uploaded,
                        downloaded: peer.downloaded,
                    },
                );
            });

            Ok(store)
        })
        .await
        .context("Failed loading peers.")
    }
}

impl Deref for TorrentStore {
    type Target = IndexMap<u32, Torrent>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TorrentStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Clone, Default)]
pub struct DBImportTorrent {
    pub id: u32,
    pub status: TorrentStatus,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
    pub is_deleted: bool,
}

#[derive(Clone, Default, Serialize)]
pub struct Torrent {
    pub id: u32,
    pub status: TorrentStatus,
    pub is_deleted: bool,
    pub peers: PeerStore,
    pub seeders: u32,
    pub leechers: u32,
    pub times_completed: u32,
    pub download_factor: u8,
    pub upload_factor: u8,
}
