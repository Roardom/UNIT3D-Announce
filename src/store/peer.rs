use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use chrono::serde::ts_seconds;
use indexmap::IndexMap;
use serde::{Serialize, Serializer};
use sqlx::types::chrono::{DateTime, Utc};

use crate::model::peer_id::PeerId;

use crate::config::Config;

#[derive(Clone, Serialize)]
pub struct PeerStore {
    inner: IndexMap<Index, Peer>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
    pub user_id: u32,
    pub peer_id: PeerId,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Peer {
    pub ip_address: std::net::IpAddr,
    pub port: u16,
    pub is_seeder: bool,
    pub is_active: bool,
    pub is_visible: bool,
    pub is_connectable: bool,
    pub has_sent_completed: bool,
    #[serde(with = "ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub uploaded: u64,
    pub downloaded: u64,
}

impl Peer {
    /// Determines if the peer should be included in the peer list
    #[inline(always)]
    pub fn is_included_in_peer_list(&self, config: &Config) -> bool {
        if config.require_peer_connectivity {
            self.is_active && self.is_visible && self.is_connectable
        } else {
            self.is_active && self.is_visible
        }
    }

    /// Determines if the peer should be included in the list of seeds
    #[inline(always)]
    pub fn is_included_in_seed_list(&self, config: &Config) -> bool {
        self.is_seeder && self.is_included_in_peer_list(config)
    }

    /// Determines if the peer should be included in the list of leeches
    #[inline(always)]
    pub fn is_included_in_leech_list(&self, config: &Config) -> bool {
        !self.is_seeder && self.is_included_in_peer_list(config)
    }
}

impl PeerStore {
    pub fn new() -> PeerStore {
        PeerStore {
            inner: IndexMap::new(),
        }
    }
}

impl Deref for PeerStore {
    type Target = IndexMap<Index, Peer>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PeerStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for PeerStore {
    fn default() -> Self {
        PeerStore::new()
    }
}

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.user_id, self.peer_id)
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
